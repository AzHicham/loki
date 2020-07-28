fn main() {
    // create rust usable structs from protobuf files
    // see https://docs.rs/prost-build/0.6.1/prost_build/
    prost_build::compile_protos(
        &[
            "navitia-proto/request.proto",
            "navitia-proto/response.proto",
        ],
        &["navitia-proto/"],
    )
    .unwrap();
    use std::env;
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("Writing protobuf code in {}/pbnavitia.rs", out_dir );
}
