
use test_generator::test_resources;

use sml_rs::parse_file_iter;


#[test_resources("./tests/libsml-testing/*.bin")]
fn basic_validation(path: &str) {
    let raw_bytes = std::fs::read(path).expect("Couldn't read file");
    let (buf, len) = sml_rs::unpack_transport_v1::<_, 1024>(&mut raw_bytes.into_iter()).expect("Couldn't unpack data");
    let file = parse_file_iter(&buf[..len]);
    for msg in file {
        println!("{:?}", msg.expect("Couldn't parse message"));
    }
}


#[test_resources("./tests/libsml-testing/*.hex")]
fn basic_validation_hex(path: &str) {
    let byte_string = std::fs::read_to_string(path).expect("Couldn't read file");

    let bytes = hex::decode(byte_string.trim()).expect("Invalid input");

    let (buf, len) = sml_rs::unpack_transport_v1::<_, 1024>(&mut bytes.into_iter()).expect("Couldn't unpack data");
    let file = parse_file_iter(&buf[..len]);
    for msg in file {
        println!("{:?}", msg.expect("Couldn't parse message"));
    }
}