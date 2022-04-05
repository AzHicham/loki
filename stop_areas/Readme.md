# Loki - Stop Areas


## Description

Reads a [ntfs][1] or a gtfs and performs a journey query between two stop areas.


## Usage

### stop_areas
The `loki_stop_areas` binary perform a journey query between two stop areas.
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


[1]: https://github.com/hove-io/ntfs-specification
[2]: https://crates.io/crates/log
[3]: ./Cargo.toml
[4]: https://docs.rs/log/0.4.11/log/#compile-time-filters
[5]: ./config.toml
