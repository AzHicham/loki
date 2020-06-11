

mod transit_data;
mod engine;
mod request;



use transit_model;
use std::path::PathBuf;

use transit_data::time::{ PositiveDuration, SecondsSinceDatasetStart};


fn main() {
    // let input_dir = PathBuf::from("tests/fixtures/small_ntfs/");
    // let start_stop_point_uri = "sp_1";
    // let end_stop_point_uri = "sp_3";

    let input_dir = PathBuf::from("tests/fixtures/ntfs_rennes/");
    let start_stop_point_uri = "GT5:SA:10030";
    let end_stop_point_uri = "GT5:SA:3716";

    let model = transit_model::ntfs::read(input_dir).unwrap();
    let transit_data = transit_data::data::TransitData::new(&model, PositiveDuration::zero());



    let start_stop_point_idx = model.stop_points.get_idx(&start_stop_point_uri).unwrap();
    let end_stop_point_idx = model.stop_points.get_idx(&end_stop_point_uri).unwrap();

    let start_stop = transit_data.stop_point_idx_to_stop(&start_stop_point_idx).unwrap().clone();
    let end_stop = transit_data.stop_point_idx_to_stop(&end_stop_point_idx).unwrap().clone();

    let start_stops = vec![(start_stop, PositiveDuration::zero())];
    let end_stops = vec![(end_stop, PositiveDuration::zero())];

    let departure_datetime = SecondsSinceDatasetStart::zero();

    let request = request::depart_after::Request::new(&transit_data, departure_datetime, start_stops, end_stops);

    let mut raptor = engine::multicriteria_raptor::MultiCriteriaRaptor::new(&request);
    raptor.compute();

    // let a_few_vj : Vec<_> = collections.vehicle_journeys.values().take(2).collect();
    // println!("{:#?}", a_few_vj);

}

