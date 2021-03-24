# Loki - Command Line Interface

Command Line Interface to obtain Tips and tricks to fluidify your (public) transit !

## Description

Reads a [ntfs][1] and performs journey queries on it with `loki` from the command line.

Two binaries are provided : `stop_areas` and `random`.

## Usage 

### stop_areas
The `stop_areas` binary perform a journey query between two stop areas :

```bash
cargo run --release --bin stop_areas -- --ntfs /path/to/ntfs  --start start_stop_area_uri --end end_stop_area_uri
```

where `start_stop_area_uri` and `end_stop_area_uri` are uri of stop areas occuring in the ntfs dataset located in the directory `/path/to/ntfs/`.


### random 
The `random` binary perform random queries on a provided ntfs dataset. 
It is useful for benchmarking.
For exemple :

```bash
cargo run --release --bin random-- --ntfs /path/to/ntfs random
```

will perform 10 queries between stop_areas chosen at random in the ntfs dataset.

## More options 

Call with each binary with  `--help` to see more options.

## Log level
You can obtain more logs by setting the environment variable `RUST_LOG` the appropriate log level.
For example :

```bash
  RUST_LOG=TRACE cargo run --release --bin random-- --ntfs /path/to/ntfs 
```

The allowed log levels are `TRACE, DEBUG, INFO, WARN, ERROR`.

## Disable logs at compile-time
You can disable a log level at compile time by specifying features for the [log][2] crate in [Cargo.toml][3], see the [log documentation][4] for more details.

## Profile with flamegraph
Install [flamegraph-rs][5] and launch 
```bash
cargo flamegraph  --bin random -- --ntfs /path/to/ntfs 
```

[1]: https://github.com/CanalTP/ntfs-specification
[2]: https://crates.io/crates/log
[3]: ./Cargo.toml
[4]: https://docs.rs/log/0.4.11/log/#compile-time-filters
[5]: https://github.com/flamegraph-rs/flamegraph