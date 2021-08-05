# Loki - Stop Areas


## Description

Reads a [ntfs][1] or a gtfs and performs random journey queries between stop areas.


## Usage

The `loki_random` binary perform random queries on a provided dataset.
It is useful for benchmarking.
For exemple :

```bash
cargo run --release -- launch --input_data_path ../data/idfm/ntfs/ --input_data_type ntfs
```

will perform 10 queries between stop_areas chosen at random in the dataset.

If you want to perform more queries :
```bash
cargo run --release -- launch --input_data_path ../data/idfm/ntfs/ --input_data_type ntfs --nb_queries 1000
```

## More options
You can also create a config file and launch the binary with a config file instead of command line arguments.
You also have more configuration options.
Call the binary with  `--help` to see the docs.

## Log level
You can obtain more logs by setting the environment variable `RUST_LOG` the appropriate log level.
For example :

```bash
  RUST_LOG=TRACE cargo run --release -- launch --input_data_path ../data/idfm/ntfs/ --input_data_type ntfs
```

The allowed log levels are `TRACE, DEBUG, INFO, WARN, ERROR`.

## Disable logs at compile-time
You can disable a log level at compile time by specifying features for the [log][2] crate in [Cargo.toml][3], see the [log documentation][4] for more details.

## Profile with flamegraph
Install [flamegraph-rs][5] and launch
```bash
cargo flamegraph --bin loki_random -- launch --input_data_path ../data/idfm/ntfs/ --input_data_type ntfs --nb_queries 1000
```

[1]: https://github.com/CanalTP/ntfs-specification
[2]: https://crates.io/crates/log
[3]: ./Cargo.toml
[4]: https://docs.rs/log/0.4.11/log/#compile-time-filters
[5]: https://github.com/flamegraph-rs/flamegraph
