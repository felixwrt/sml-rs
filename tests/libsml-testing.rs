
use test_generator::test_resources;

use sml_rs::{SmlParse, File};


#[test_resources("./tests/libsml-testing/*.bin")]
fn basic_validation(path: &str) {
    let raw_bytes = std::fs::read(path).expect("Couldn't read file");
    let mut cursor = std::io::Cursor::new(raw_bytes);
    let bytes = sml_rs::unpack_transport_v1(&mut cursor).expect("Couldn't unpack data");
    let file = File::parse(&bytes);
    println!("{:?}", file);
    assert!(file.is_ok());
}


// #[test_resources("./tests/libsml-testing/*.hex")]
// fn basic_validation_hex(path: &str) {
//     let byte_string = std::fs::read_to_string(path).expect("Couldn't read file");

//     let bytes = hex::decode(byte_string.trim()).expect("Invalid input");

//     let mut cursor = std::io::Cursor::new(bytes);
//     let bytes = sml_rs::unpack_transport_v1(&mut cursor).expect("Couldn't unpack data");
//     let file = File::parse(&bytes);
//     println!("{:?}", file);
//     //assert!(file.is_ok());
// }