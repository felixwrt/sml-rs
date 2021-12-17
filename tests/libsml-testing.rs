
use std::{collections::HashSet, ffi::{OsStr, OsString}};
use std::iter::FromIterator;

use anyhow::{Result, bail};


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
        bail!("There are no test files in ./tests/libsml-testing. You probably need to initialize the git submodule. Try `git submodule init && git submodule update`.\n")
    }

    // check that for each golden file, there's also an input file
    let mut exp_filenames = HashSet::new();
    for entry in std::fs::read_dir("./tests/libsml-testing-expected").expect("folder with expected files doesn't exist") {
        let entry = entry?.path();
        match (entry.file_stem(), entry.extension().and_then(OsStr::to_str)) {
            (Some(name), Some("exp")) => {
                exp_filenames.insert(name.to_os_string());
            },
            _ => {}  // ignore other files
        }
    }
    if !exp_filenames.is_subset(&bin_filenames) {
        eprintln!("The following golden files don't have corresponding test input files:");
        for filename in exp_filenames.difference(&bin_filenames) {
            eprintln!("{}", filename.to_string_lossy());
        }
        eprintln!();
        panic!("Invalid golden files")
    }

    // check that bin and hex files contain the same content
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

#[cfg(feature = "alloc")]
#[test_generator::test_resources("./tests/libsml-testing/*.bin")]
fn basic_validation(path: &str) {
    let raw_bytes = std::fs::read(path).expect("Couldn't read file");
    
    test_bytes(&raw_bytes, std::path::Path::new(path).file_stem().unwrap())
}


// #[test_resources("./tests/libsml-testing/*.hex")]
// fn basic_validation_hex(path: &str) {
//     let byte_string = std::fs::read_to_string(path).expect("Couldn't read file");
//     let bytes = hex::decode(byte_string.trim()).expect("Invalid input");
//     test_bytes(&bytes)
// }

#[cfg(feature = "alloc")]
fn test_bytes(bytes: &[u8], filename: &OsStr) {
    use std::fmt::Write;
    use sml_rs::ParseRes;

    let exp_path = std::path::Path::new("./tests/libsml-testing-expected/").join(OsString::from_iter([filename.to_os_string(), ".exp".to_string().into()]));
    
    let mut s = String::new();

    let mut reader = sml_rs::SmlReader::<sml_rs::VecBuf>::new();
    for b in bytes {
        let res = reader.push_byte(*b);
        let res = match res {
            Ok(None) => None,
            Ok(Some(ParseRes::Transmission(msg_bytes))) => Some(Ok(ParseRes::Transmission(msg_bytes.len()))),
            Ok(Some(ParseRes::DiscardedBytes(n))) => Some(Ok(ParseRes::DiscardedBytes(n))),
            Err(e) => Some(Err(e))
        };
        match res {
            Some(res) => {
                writeln!(&mut s, "{:?}", res).unwrap();
            },
            None => {}
        }
    }
    if let Some(res) = reader.finalize() {
        writeln!(&mut s, "{:?}", res).unwrap();
    }

    // read golden file
    if exp_path.exists() {
        let exp_str = std::fs::read_to_string(exp_path).expect("Coudn't read golden file");
        if exp_str.trim() != s.trim() {
            eprintln!("Expected:\n{}\n", exp_str.trim());
            eprintln!("Actual:\n{}\n", s.trim());
            panic!("Output doesn't match")
        }
    } else {
        eprintln!("Expected: Golden file `{}` does not exist\n", exp_path.to_string_lossy());
        eprintln!("Actual:\n{}", s);

        panic!("Golden file does not exist")
    }
}

