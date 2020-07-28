# Laxatips - Command Line Interface

Command Line Interface to obtain Tips and tricks to fluidify your (public) transit !

## Description

Reads a [ntfs][1] and performs journey queries on it with `laxatips` from the command line.

Launch

Two usages for now :
- you can specify an origin and destination stop_areas with
  ```bash
  cargo run --release -- --ntfs /path/to/ntfs stop-areas --start start_stop_area_uri --end end_stop_area_uri
  ```
  where `start_stop_area_uri` and `end_stop_area_uri` are uri of stop areas occuring in the ntfs dataset located in the directory `/path/to/ntfs/`.

  You can obtain more logs by specifying the log level as follows
   ```bash
    RUST_LOG=TRACE cargo run --release -- --ntfs /path/to/ntfs stop-areas --start start_stop_area_uri --end end_stop_area_uri
  ```
  where the allows log levels are `TRACE, DEBUG, INFO, WARN, ERROR`.

- you can perform random queries on a ntfs dataset with
  ```bash
  cargo run --release -- --ntfs /path/to/ntfs random
  ```
  This will perform 10 queries between stop_areas chosen at random in the ntfs dataset.
  You can specify the number of queries to perform with
  ```bash
  cargo run --release -- --ntfs /path/to/ntfs random --nb-queries 100
  ```

In both cases you can change some parameters (e.g. the maximum duration of a journey, the maximum number of public transit leg, etc.) with command line options. To obtain a list, launch :
```bash
cargo run --release -- help
```


# Disable logs at compile-time
You can disable a log level at compile time by specifying features for the [log][2] crate in [Cargo.toml][3], see the [log documentation][4] for more details.

# Profile with flamegraph
Install [flamegraph-rs][5] and launch 
```bash
cargo flamegraph --bin laxatips-cli -- --ntfs /path/to/ntfs random --nb-queries 1000
```

[1]: https://github.com/CanalTP/ntfs-specification
[2]: https://crates.io/crates/log
[3]: ./Cargo.toml
[4]: https://docs.rs/log/0.4.11/log/#compile-time-filters
[5]: https://github.com/flamegraph-rs/flamegraph