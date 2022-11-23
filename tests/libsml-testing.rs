use std::fmt::Write;
use std::iter::FromIterator;
use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
};

use sml_rs::parser::SmlParse;

#[test]
fn test_repo_validation()  {
    let dir = std::fs::read_dir("./tests/libsml-testing").expect("test folder does not exist");
    let mut bin_filenames = HashSet::new();
    let mut hex_filenames = HashSet::new();
    for entry in dir {
        let entry = entry.unwrap().path();
        match (entry.file_stem(), entry.extension().and_then(OsStr::to_str)) {
            (Some(name), Some("bin")) => {
                bin_filenames.insert(name.to_os_string());
            }
            (Some(name), Some("hex")) => {
                hex_filenames.insert(name.to_os_string());
            }
            _ => {} // ignore other files
        }
    }

    assert_eq!(bin_filenames, hex_filenames);

    if bin_filenames.is_empty() {
        panic!("There are no test files in ./tests/libsml-testing. You probably need to initialize the git submodule. Try `git submodule init && git submodule update`.\n")
    }

    // check that bin and hex files contain the same content
    for filename in bin_filenames {
        let path = std::path::Path::new("./tests/libsml-testing");

        let bin_path = path.join(OsString::from_iter([
            filename.clone(),
            ".bin".to_string().into(),
        ]));
        let bin_bytes = std::fs::read(bin_path).expect("Couldn't read file");
        let hex_path = path.join(OsString::from_iter([filename, ".hex".to_string().into()]));
        let hex_string = std::fs::read_to_string(hex_path).expect("Couldn't read file");
        let hex_bytes = hex::decode(hex_string.trim()).expect("Couldn't decode hex string");

        assert_eq!(bin_bytes, hex_bytes);
    }
}

#[test]
fn test_files() {
    insta::glob!("libsml-testing/*.bin", |path| {
        let bytes = std::fs::read(path).unwrap();

        let mut decoder = sml_rs::transport::decode_streaming::<heapless::Vec<u8, 2048>>(bytes);

        let mut s = String::new();
        while let Some(result) = decoder.next() {
            // write!(s, "{:?}\n", result.map(|x| x.len())).unwrap();
            write!(s, "{:#?}\n", result.map(|x| {
                let res = sml_rs::parser::domain::File::parse_complete(x);
                res.expect("Error while parsing:").messages
            })).unwrap();
        }
        insta::assert_snapshot!(s);
    });
}
