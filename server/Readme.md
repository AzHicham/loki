# Laxatips - Server

Serve you Tips and tricks to fluidify your (public) transit !

## Description

Reads a [ntfs][1] dataset and then process protobuf journey requests (the format is specified by the [navitia-proto][2] repo) send to a zmq socket, call the `laxatips` engine, and returns the protobuf response on the zmq socket.

## How to compile 

You should have 

Be sure to update the `navitia-proto` git submodule.
```bash
git submodule update
cargo build --release
```


## How to use

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


## Architecture

### Protobuf 

This crate uses [prost][4] to handle protobuf (de)serialization specified in [navitia-proto][2].
This means that Rust code is generated from `.proto` files at compile time, by [prost-build][3] in the build script [build.rs][5]. 
To see where the rust code is generated, run 
```bash
cargo build --release -vv
```
And you should see a line like the following in the output :
```bash
[server 0.1.0] Writing protobuf code in /home/pascal/laxatips/target/release/build/server-52f917f3d3486970/out/pbnavitia.rs
```

See [this page][6] for more information on build scripts.

### ZMQ


[1]: https://github.com/CanalTP/ntfs-specification
[2]: https://github.com/CanalTP/navitia-proto
[3]: https://crates.io/crates/prost-build
[4]: https://crates.io/crates/prost
[5]: ./build.rs
[6]: https://doc.rust-lang.org/cargo/reference/build-scripts.html