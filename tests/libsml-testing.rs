
use std::{collections::HashSet, ffi::{OsStr, OsString}};
use std::iter::FromIterator;

use anyhow::{Result, bail};
use test_generator::test_resources;

use sml_rs::parse_file_iter;

#[test]
fn test_repo_validation() -> Result<()> {
    let dir = std::fs::read_dir("./tests/libsml-testing").expect("test folder does not exist");
    let mut bin_filenames = HashSet::new();
    let mut hex_filenames = HashSet::new();
    for entry in dir {
        let entry = entry?.path();
        match (entry.file_stem(), entry.extension().and_then(OsStr::to_str)) {
            (Some(name), Some("bin")) => {
                bin_filenames.insert(name.to_os_string());
            },
            (Some(name), Some("hex")) => {
                hex_filenames.insert(name.to_os_string());
            }
            _ => {}  // ignore other files
        }
    }

    assert_eq!(bin_filenames, hex_filenames);

    if bin_filenames.is_empty() {
        bail!("There are no test files in ./tests/libsml-testing. You probably need to initialize the git submodule")
    }

    for filename in bin_filenames {
        let path = std::path::Path::new("./tests/libsml-testing");
        
        let bin_path = path.join(OsString::from_iter([filename.clone(), ".bin".to_string().into()]));
        let bin_bytes = std::fs::read(bin_path).expect("Couldn't read file");
        let hex_path = path.join(OsString::from_iter([filename, ".hex".to_string().into()]));
        let hex_string = std::fs::read_to_string(hex_path).expect("Couldn't read file");
        let hex_bytes = hex::decode(hex_string.trim()).expect("Couldn't decode hex string");

        assert_eq!(bin_bytes, hex_bytes);
    }

    Ok(())
}

#[test_resources("./tests/libsml-testing/*.bin")]
fn basic_validation(path: &str) {
    let raw_bytes = std::fs::read(path).expect("Couldn't read file");
    test_bytes(&raw_bytes)
}


#[test_resources("./tests/libsml-testing/*.hex")]
fn basic_validation_hex(path: &str) {
    let byte_string = std::fs::read_to_string(path).expect("Couldn't read file");
    let bytes = hex::decode(byte_string.trim()).expect("Invalid input");
    test_bytes(&bytes)
}

fn test_bytes(bytes: &[u8]) {
    let (buf, len) = sml_rs::unpack_transport_v1::<_, 1048>(&mut bytes.into_iter().cloned()).expect("Couldn't unpack data");
    let file = parse_file_iter(&buf[..len]);
    for msg in file {
        println!("{:?}", msg.expect("Couldn't parse message"));
    }
}