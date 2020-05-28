

mod transit_data;
mod engine;




use transit_model;
use std::path::PathBuf;


fn main() {
    let input_dir = PathBuf::from("tests/fixtures/small_ntfs/");
    //let input_dir = PathBuf::from("tests/fixtures/ntfs_rennes/");
    let model = transit_model::ntfs::read(input_dir).unwrap();
    let collections = model.into_collections();
    let a_few_vj : Vec<_> = collections.vehicle_journeys.values().take(2).collect();
    println!("{:#?}", a_few_vj);

}

