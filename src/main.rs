use anyhow::{anyhow, Error, Result};
use hex;
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use reqwest::{header::AUTHORIZATION, Url};
use serde_json::{self, json};
use std::{
    ops::Deref,
    sync::{Arc, RwLock},
    time::Duration,
};
use structopt::StructOpt;
use strum_macros::EnumString;
use tokio::{self, sync::Semaphore};
use tracing::info;
use tracing_subscriber;

mod http_structs;

use http_structs::*;

const TZKT: &str = "https://api.tzkt.io";
const TZKT_CHUNCKS: usize = 10000;
const IPFS_DAEMON: &str = "http://localhost:5001";
const ESTUARY: &str = "https://api.estuary.tech";
const CF_IPFS: &str = "https://cloudflare-ipfs.com";
const HICDEX: &str = "https://api.hicdex.com";

#[derive(Debug, StructOpt, Clone, EnumString)]
enum OptIndexer {
    HicDex,
    TzKT,
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(name = "henflow", about = "Hicetnunc backup tool.")]
struct Opt {
    #[structopt(long, default_value = "HicDex", help = "Way to discover OBJKTs from")]
    indexer: OptIndexer,
    #[structopt(
        long,
        default_value = "1000",
        help = "Maximum number of parallel tasks to process OBJKTs"
    )]
    num_tasks: usize,
    #[structopt(long, help = "Estuary API key")]
    estuary_token: String,
    #[structopt(long, help = "Individual HTTP requests' timeout, in seconds")]
    http_timeout: Option<u64>,
    #[structopt(subcommand)]
    cmd: OptCmd,
}

#[derive(Debug, StructOpt, Clone)]
enum OptCmd {
    #[structopt(about = "Back up an OBJKT or all of HicEtNunc")]
    Backup {
        #[structopt(short, long)]
        all: bool,
        #[structopt(global = true)]
        token_id: Option<u64>,
    },
    #[structopt(about = "Check if an OBJKT is pinned by Estuary")]
    Status {
        #[structopt(global = true)]
        token_id: u64,
    },
    #[structopt(about = "Get the size of all OBJKTs")]
    Size {
        #[structopt(subcommand)]
        cmd: OptCmdSize,
    },
}

#[derive(Debug, StructOpt, Clone)]
enum OptCmdSize {
    #[structopt(about = "Only OBJKTs pinned by Estuary")]
    Pins,
    #[structopt(about = "All OBJKTs in the contract's big map")]
    Artefacts,
}

async fn fetch_token_count(opt: Opt, http_client: reqwest::Client) -> Result<u64> {
    Ok(match opt.indexer {
        OptIndexer::HicDex => {
            let res: HicDexAggregate = http_client
    .post(Url::parse(&HICDEX)?.join("v1/graphql")?)
    .json(&json!({"operationName": "PriceHistory", "variables": null, "query": "query PriceHistory {  hic_et_nunc_token_aggregate(distinct_on: id) {    aggregate {      count(columns: artifact_uri)    }  }}"}))
    .send()
    .await?
    .json()
    .await?;
            res.data.hic_et_nunc_token_aggregate.aggregate.count
        }
        OptIndexer::TzKT => {
            let token_keys: TokenKeys = http_client
                .get(Url::parse(&TZKT)?.join("v1/bigmaps/514")?)
                .query(&[("active", "true")])
                .send()
                .await?
                .json()
                .await?;
            token_keys.active_keys
        }
    })
}

async fn fetch_token_cid(http_client: reqwest::Client, token_id: u64) -> Result<String> {
    let res: HicDexResponsePk = http_client
        .post(Url::parse(&HICDEX)?.join("v1/graphql")?)
        .json(&json!({"operationName": "PriceHistory", "query": format!("query PriceHistory {{  hic_et_nunc_token_by_pk(id: \"{}\") {{    artifact_uri  }}}}", token_id.to_string())}))
        .send()
        .await?
        .json()
        .await?;
    Ok(res
        .data
        .hic_et_nunc_token_by_pk
        .artifact_uri
        .replace("ipfs://", ""))
}

async fn fetch_artefact_cid(http_client: reqwest::Client, cid: String) -> Result<String> {
    // let metadata: TokenMetadataFile = http_client
    //     .post(Url::parse(&IPFS_DAEMON)?.join("/api/v0/object/get")?)
    //     .query(&[("arg", &format!("/ipfs/{}", cid))])
    //     .send()
    //     .await?
    //     .json()
    //     .await?;
    // let mut ipfs_res = metadata.data;
    // while ipfs_res.chars().nth(0).unwrap() != '{' {
    //     ipfs_res.remove(0);
    // }
    // while ipfs_res.chars().nth_back(0).unwrap() != '}' {
    //     ipfs_res.pop();
    // }
    // let metadata: TokenMetadata = serde_json::from_str(&ipfs_res)?;
    let metadata: TokenMetadata = http_client
        .post(Url::parse(&CF_IPFS)?.join(&format!("/ipfs/{}", cid))?)
        .send()
        .await?
        .json()
        .await?;
    Ok(metadata.artifact_uri.replace("ipfs://", ""))
}

async fn fetch_artefact_size(http_client: reqwest::Client, cid: String) -> Result<u64> {
    let artefact_stat: FileStat = http_client
        .post(Url::parse(&IPFS_DAEMON)?.join("/api/v0/object/stat")?)
        .query(&[("arg", &format!("/ipfs/{}", cid))])
        .send()
        .await?
        .json()
        .await?;
    Ok(artefact_stat.cumulative_size)
}

async fn pin_estuary(estuary_key: String, http_client: reqwest::Client, cid: String) -> Result<()> {
    if fetch_pin_estuary(estuary_key.clone(), http_client.clone(), cid.clone())
        .await?
        .is_some()
    {
        return Ok(());
    }
    let estuary_response: EstuaryUpload = http_client
        .post(Url::parse(&ESTUARY)?.join("pinning/pins")?)
        .header(AUTHORIZATION, &format!("Bearer {}", estuary_key))
        .json(&json!({ "cid": cid }))
        .send()
        .await?
        .json()
        .await?;
    Ok(())
}

async fn fetch_pin_estuary(
    estuary_key: String,
    http_client: reqwest::Client,
    cid: String,
) -> Result<Option<u64>> {
    let estuary_response = http_client
        .get(Url::parse(&ESTUARY)?.join(&format!("content/by-cid/{}", cid))?)
        .header(AUTHORIZATION, &format!("Bearer {}", estuary_key))
        .send()
        .await?;
    match estuary_response.error_for_status() {
        Ok(res) => {
            let text_res = res.text().await?;
            if text_res == "null\n" {
                return Ok(None);
            }
            let pin: Vec<EstuaryPin> = serde_json::from_str(&text_res)?;
            Ok(Some(pin.get(0).ok_or(anyhow!("No pin"))?.content.size))
        }
        Err(err) => {
            if err.status() == Some(reqwest::StatusCode::NOT_FOUND) {
                Ok(None)
            } else {
                Err(err)?
            }
        }
    }
}

async fn token_metadata(opt: Opt) -> Result<()> {
    let max_num_tasks = Arc::new(Semaphore::new(opt.num_tasks));
    let total_data = Arc::new(RwLock::new(0));
    let c_total_data = Arc::clone(&total_data);
    let mut offset = TZKT_CHUNCKS;
    let mut progress = 0;
    let http_client_builder = reqwest::Client::builder();
    let http_client_builder = if let Some(timeout) = opt.http_timeout {
        http_client_builder.timeout(Duration::from_secs(timeout))
    } else {
        http_client_builder
    };
    let http_client = http_client_builder.build()?;
    match opt.cmd {
        OptCmd::Backup {
            all: false,
            token_id: Some(token_id),
        } => {
            let cid = fetch_token_cid(http_client.clone(), token_id).await?;

            pin_estuary(opt.estuary_token.clone(), http_client.clone(), cid).await?;
            return Ok(());
        }
        OptCmd::Status { token_id } => {
            let cid = fetch_token_cid(http_client.clone(), token_id).await?;
            info!("CID: {}", cid);
            if fetch_pin_estuary(opt.estuary_token.clone(), http_client.clone(), cid)
                .await?
                .is_some()
            {
                println!("Pinned.");
            } else {
                println!("Not pinned.");
            };
            return Ok(());
        }
        _ => (),
    };
    let count_tokens = fetch_token_count(opt.clone(), http_client.clone()).await?;
    let bar = ProgressBar::new(count_tokens);
    bar.set_style(
        ProgressStyle::default_bar()
            .template(
                "[{elapsed} | ETA: {eta}] {bar:40.cyan/blue} {pos:>7}/{len:7}({percent}%) {msg}",
            )
            .progress_chars("##-"),
    );
    let mut handles: Vec<(String, tokio::task::JoinHandle<Result<(), Error>>)> = vec![];
    while offset == TZKT_CHUNCKS {
        let tokens = match opt.indexer {
            OptIndexer::HicDex => {
                let res: HicDexResponse = http_client
    .post(Url::parse(&HICDEX)?.join("v1/graphql")?)
    .json(&json!({"operationName": "PriceHistory", "variables": null, "query": format!("query PriceHistory {{  hic_et_nunc_token(limit: {}, offset: {}, order_by: {{id: asc}}) {{ artifact_uri id }}}}", TZKT_CHUNCKS, progress)}))
    .send()
    .await?
    .json()
    .await?;
                res.data.hic_et_nunc_token
            }
            OptIndexer::TzKT => {
                let res: Vec<TokenMetadataBigMap> = http_client
                    .get(Url::parse(&TZKT)?.join("v1/bigmaps/514/keys")?)
                    .query(&[
                        ("active", "true"),
                        ("limit", &TZKT_CHUNCKS.to_string()),
                        ("offset", &progress.to_string()),
                    ])
                    .send()
                    .await?
                    .json()
                    .await?;
                res.iter()
                    .map(|x| {
                        Ok(HicDexResponseDataTokens {
                            id: x.key.parse()?,
                            // TODO it's actually the CID for the document with the artefact uri
                            artifact_uri: String::from_utf8(hex::decode(
                                x.value.token_info.value.as_bytes().to_vec(),
                            )?)?,
                        })
                    })
                    .collect::<Result<Vec<_>, Error>>()?
            }
        };
        offset = tokens.len();
        for token_metadata in tokens.iter() {
            let permit = Arc::clone(&max_num_tasks).acquire_owned().await;
            let artefact_uri = token_metadata.artifact_uri.clone();
            let bar2 = bar.clone();
            let http_client2 = http_client.clone();
            let total_data2 = c_total_data.clone();
            let opt2 = opt.clone();
            let handle = tokio::spawn(async move {
                let _permit = permit;
                let cid = if let OptIndexer::TzKT = opt2.indexer {
                    fetch_artefact_cid(http_client2.clone(), artefact_uri.replace("ipfs://", ""))
                        .await?
                } else {
                    artefact_uri.replace("ipfs://", "")
                };
                if !cid.is_empty() {
                    match opt2.cmd {
                        OptCmd::Backup { .. } => {
                            pin_estuary(opt2.estuary_token.clone(), http_client2.clone(), cid)
                                .await?;
                        }
                        OptCmd::Size { cmd } => {
                            let size = match cmd {
                                OptCmdSize::Artefacts => {
                                    fetch_artefact_size(http_client2, cid).await?
                                }
                                OptCmdSize::Pins => {
                                    fetch_pin_estuary(opt2.estuary_token.clone(), http_client2, cid)
                                        .await?
                                        .unwrap_or(0)
                                }
                            };
                            let mut w = total_data2.write().unwrap();
                            *w += size;
                            bar2.set_message(format!("[{}]", HumanBytes(*w.deref()).to_string()));
                        }
                        _ => (),
                    }
                }
                bar2.inc(1);
                Ok(())
            });
            handles.push((token_metadata.id.to_string(), handle));
        }
        progress = progress + offset;
    }
    for (token_key, handle) in handles.iter_mut() {
        match handle.await? {
            Err(e) => println!("Token n.{}: {}", token_key, e),
            Ok(_) => (),
        };
    }
    bar.tick();
    bar.finish();
    Ok(())
}

#[tokio::main(flavor = "multi_thread", worker_threads = 8)]
async fn main() {
    let opt = Opt::from_args();
    tracing_subscriber::fmt::init();
    token_metadata(opt).await.unwrap();
}
