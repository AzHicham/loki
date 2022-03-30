# Loki - Stop Areas


## Description

Reads a [ntfs][1] or a gtfs and performs random journey queries between stop areas.


## Usage

The `loki_random` binary perform random queries on a provided dataset.
It is useful for benchmarking.
See the sample [config file][6] for configuration options.
Run with

```bash
cargo run --release -- path/to/config.toml
```

## Log level
You can obtain more logs by setting the environment variable `RUST_LOG` the appropriate log level.
For example :

```bash
  RUST_LOG=TRACE cargo run --release -- path/to/config.toml
```

The allowed log levels are `TRACE, DEBUG, INFO, WARN, ERROR`.

## Disable logs at compile-time
You can disable a log level at compile time by specifying features for the [log][2] crate in [Cargo.toml][3], see the [log documentation][4] for more details.

## Profile with flamegraph
Install [flamegraph-rs][5] and launch
```bash
cargo flamegraph --bin loki_random -- path/to/config.toml
```

[1]: https://github.com/CanalTP/ntfs-specification
[2]: https://crates.io/crates/log
[3]: ./Cargo.toml
[4]: https://docs.rs/log/0.4.11/log/#compile-time-filters
[5]: https://github.com/flamegraph-rs/flamegraph
[6]: ./config.toml
