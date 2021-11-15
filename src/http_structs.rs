use serde::Deserialize;

#[derive(Deserialize)]
pub struct TokenMetadataBigMapInfo {
    #[serde(rename = "")]
    pub value: String,
}

#[derive(Deserialize)]
pub struct TokenMetadataBigMapValue {
    pub token_info: TokenMetadataBigMapInfo,
}

#[derive(Deserialize)]
pub struct TokenMetadataBigMap {
    pub key: String,
    pub value: TokenMetadataBigMapValue,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenMetadata {
    // name: String,
    pub artifact_uri: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TokenMetadataFile {
    pub data: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenKeys {
    pub active_keys: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct FileStat {
    pub cumulative_size: u64,
}

#[derive(Deserialize)]
pub struct EstuaryUpload {}

#[derive(Deserialize)]
pub struct EstuaryContentContent {
    pub size: u64,
}

#[derive(Deserialize)]
pub struct EstuaryContent {
    pub content: EstuaryContentContent,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstuaryPinContent {
    pub size: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstuaryPin {
    // aggregated_in:
    pub content: EstuaryPinContent, // deals: Vec<>
}

#[derive(Deserialize)]
pub struct HicDexResponseDataTokens {
    pub artifact_uri: String,
    pub id: u64,
}

#[derive(Deserialize)]
pub struct HicDexResponseData {
    pub hic_et_nunc_token: Vec<HicDexResponseDataTokens>,
}

#[derive(Deserialize)]
pub struct HicDexResponse {
    pub data: HicDexResponseData,
}

#[derive(Deserialize)]
pub struct HicDexResponsePkDataTokens {
    pub artifact_uri: String,
}

#[derive(Deserialize)]
pub struct HicDexResponsePkData {
    pub hic_et_nunc_token_by_pk: HicDexResponsePkDataTokens,
}

#[derive(Deserialize)]
pub struct HicDexResponsePk {
    pub data: HicDexResponsePkData,
}

#[derive(Deserialize)]
pub struct HicDexAggregateDataAggregateCount {
    pub count: u64,
}

#[derive(Deserialize)]
pub struct HicDexAggregateDataAggregate {
    pub aggregate: HicDexAggregateDataAggregateCount,
}

#[derive(Deserialize)]
pub struct HicDexAggregateData {
    pub hic_et_nunc_token_aggregate: HicDexAggregateDataAggregate,
}

#[derive(Deserialize)]
pub struct HicDexAggregate {
    pub data: HicDexAggregateData,
}
