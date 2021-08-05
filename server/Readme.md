# Loki - Server

Serve you Tips and tricks to fluidify your (public) transit !

## Description

Reads a [ntfs][1] dataset and then process protobuf journey requests (the format is specified by the [navitia-proto][2] repo) send to a zmq socket, call the `loki` engine, and returns the protobuf response on the zmq socket.

## How to compile

Install libzmq (needed for the `zmq` crate, cf https://crates.io/crates/zmq/):
```bash
apt install libzmq3-dev
```

Be sure to fetch the `navitia-proto` git submodule.
```bash
git submodule init    # a first time, to fetch the navitia-proto repo
git submodule update  # each time the navitia-proto repo is updated
cargo build --release
```

## How to use

Loki-server can be used to answer the "public transit" part of distributed journey request.
The setup is as follow :
- a jormun server will receive the distributed journey request, and create several subrequest to be handled by backends, before
  serving the response
- a loki-server backend will answer all "pt_journey" subrequests
- a kraken backend will answer all other subrequests

You should have a ntfs dataset in `/path/to/ntfs` which has been binarized to `/path/to/data.nav.lz4` (you can use [eitry][8] for generating a `data.nav.lz4` from a ntfs dataset).
You need to setup :
- a jormun server from [this branch][7] which should be configured with
```json
{"key": "mycoverage", "zmq_socket": "ipc:///tmp/kraken", "pt_zmq_socket" : "ipc:///tmp/loki"}
```
- a kraken configured with
```
[GENERAL]
instance_name = "mycoverage"
database = /path/to/data.nav.lz4
zmq_socket = ipc:///tmp/kraken
```

- a loki server launched with
  ```bash
  cargo run --release -- launch --input_data_path /path/to/ntfs  --input_data_type ntfs --basic_requests_socket ipc:///tmp/loki
  ```

Then you can send http requests to the jormun server !

## Architecture

### Protobuf

This crate uses [prost][4] to handle (de)serialization of protobuf. The protobuf schema is specified in [navitia-proto][2].
This means that Rust code is generated from `.proto` files at compile time, by [prost-build][3] in the build script [build.rs][5].
To see where the rust code is generated, run
```bash
cargo build --release -vv
```
And you should see a line like the following in the output :
```bash
[server 0.1.0] Writing protobuf code in /home/pascal/loki/target/release/build/server-52f917f3d3486970/out/pbnavitia.rs
```

See [this page][6] for more information on build scripts.






[1]: https://github.com/CanalTP/ntfs-specification
[2]: https://github.com/CanalTP/navitia-proto
[3]: https://crates.io/crates/prost-build
[4]: https://crates.io/crates/prost
[5]: ./build.rs
[6]: https://doc.rust-lang.org/cargo/reference/build-scripts.html
[7]: https://github.com/CanalTP/navitia/pull/3251
[8]: https://github.com/CanalTP/navitia/blob/dev/source/eitri/Readme.md
