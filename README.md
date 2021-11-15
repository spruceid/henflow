# henflow

henflow backs up assets from Hic et Nunc to [Estuary](estuary.tech).

To back up the assets of an individual OBJKT with an ID of 531544:
```bash
$ henflow --estuary-token <API_TOKEN> backup 531544
```

To check the backup status of the asset for an individual OBJKT with an ID of 531544:
```bash
$ henflow --estuary-token <API_TOKEN> status 531544
```

----

```
$ henflow help
henflow 0.1.0
Hicetnunc backup tool.

USAGE:
    henflow [OPTIONS] --estuary-token <estuary-token> <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --estuary-token <estuary-token>    Estuary API key
        --http-timeout <http-timeout>      Individual HTTP requests' timeout, in seconds
        --indexer <indexer>                Way to discover OBJKTs from [default: HicDex]
        --num-tasks <num-tasks>            Maximum number of parallel tasks to process OBJKTs [default: 1000]

SUBCOMMANDS:
    backup    Back up an OBJKT or all of HicEtNunc
    help      Prints this message or the help of the given subcommand(s)
    size      Get the size of all OBJKTs
    status    Check if an OBJKT is pinned by Estuary
```
