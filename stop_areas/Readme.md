# Loki - Stop Areas


## Description

Reads a [ntfs][1] or a gtfs and performs a journey query between two stop areas.


## Usage 

### stop_areas
The `loki_stop_areas` binary perform a journey query between two stop areas :

```bash
cargo run --release -- launch --input_data_path /path/to/ntfs  --input_data_type ntfs --start start_stop_area_uri --end end_stop_area_uri
```

where `start_stop_area_uri` and `end_stop_area_uri` are uri of stop areas occuring in the ntfs dataset located in the directory `/path/to/ntfs/`.

For example : 
```bash
cargo run --release -- launch --input_data_path tests/one_line/  --input_data_type ntfs --start stop_area:massy --end stop_area:paris
```


## More options 
You can also create a config file and launch the binary with a config file instead of command line arguments.
You also have more configuration options.
Call the binary with  `--help` to see the docs.

## Log level
You can obtain more logs by setting the environment variable `RUST_LOG` the appropriate log level.
For example :

```bash
  RUST_LOG=TRACE cargo run --release -- launch --input_data_path tests/one_line/  --input_data_type ntfs --start stop_area:massy --end stop_area:paris
```

The allowed log levels are `TRACE, DEBUG, INFO, WARN, ERROR`.

## Disable logs at compile-time
You can disable a log level at compile time by specifying features for the [log][2] crate in [Cargo.toml][3], see the [log documentation][4] for more details.


[1]: https://github.com/CanalTP/ntfs-specification
[2]: https://crates.io/crates/log
[3]: ./Cargo.toml
[4]: https://docs.rs/log/0.4.11/log/#compile-time-filters
