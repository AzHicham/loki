
pub mod navitia_proto;
use std::path::Path;
use std::fs;
use failure::{ Error};
use prost::Message;
fn main() -> Result<(), Error> {
    println!("Connecting to hello world server...\n");

    let context = zmq::Context::new();
    let requester = context.socket(zmq::REQ).unwrap();

    assert!(requester.connect("tcp://localhost:5555").is_ok());

    let mut msg = zmq::Message::new();

    let request_filepath = Path::new("../server/tests/auvergne/with_resp/3_simplified.proto");
    // let request_filepath = Path::new("./tests/request1.proto");


    let proto_request_bytes = fs::read(request_filepath)?;
    let proto_request = navitia_proto::Request::decode(proto_request_bytes.clone().as_slice())?;



    for request_nbr in 0..10 {
        println!("Sending {}...", request_nbr);
        requester.send(proto_request_bytes.clone(), 0).unwrap();

        requester.recv(&mut msg, 0).unwrap();
        use std::ops::Deref;
        let proto_response = navitia_proto::Response::decode(msg.deref());
        println!("Received {:#?}: {}", proto_response, request_nbr);
    }

    Ok(())
}