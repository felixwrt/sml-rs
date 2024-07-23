use std::iter::FromIterator;
use std::{
    collections::HashSet,
    ffi::{OsStr, OsString},
};

#[test]
fn test_repo_validation() {
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
        panic!("There are no test files in tests/libsml-testing.\n")
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

#[cfg(feature = "alloc")]
#[test]
fn test_files() {
    use std::fmt::Write;

    insta::glob!("libsml-testing/*.bin", |path| {
        let bytes = std::fs::read(path).unwrap();

        let mut decoder =
            sml_rs::transport::decode_streaming::<sml_rs::util::ArrayBuf<2048>>(bytes);

        let mut s = String::new();
        while let Some(result) = decoder.next() {
            // write!(s, "{:?}\n", result.map(|x| x.len())).unwrap();
            write!(
                s,
                "{:#?}\n",
                result.map(|x| {
                    let res = sml_rs::parser::complete::parse(x);
                    res.expect("Error while parsing:").messages
                })
            )
            .unwrap();
        }
        insta::assert_snapshot!(s);
    });
}

#[cfg(test)]
mod test_attention_response {
    use sml_rs::parser::{
        common::{
            AttentionErrorCode, AttentionNumber, AttentionResponse, CloseResponse, HintNumber,
            OpenResponse, Time, Tree,
        },
        streaming::{self, MessageBody, MessageStart, ParseEvent},
    };

    #[test]
    fn attention_response_order_not_executed() {
        let bytes = vec![
            0x1B, 0x1B, 0x1B, 0x1B, 0x01, 0x01, 0x01, 0x01, 0x76, 0x02, 0x01, 0x62, 0x00, 0x62,
            0x00, 0x72, 0x63, 0x01, 0x01, 0x76, 0x01, 0x07, 0x43, 0x4C, 0x4E, 0x49, 0x44, 0x31,
            0x0A, 0x51, 0x00, 0x00, 0x00, 0x00, 0x66, 0x9F, 0x41, 0xA7, 0x0B, 0x0A, 0x01, 0x4C,
            0x47, 0x5A, 0x00, 0x03, 0xA9, 0xC6, 0x26, 0x72, 0x62, 0x01, 0x65, 0x00, 0x08, 0x5A,
            0xE0, 0x01, 0x63, 0x31, 0x66, 0x00, 0x76, 0x02, 0x02, 0x62, 0x00, 0x62, 0x00, 0x72,
            0x63, 0xFF, 0x01, 0x74, 0x0B, 0x0A, 0x01, 0x4C, 0x47, 0x5A, 0x00, 0x03, 0xA9, 0xC6,
            0x26, 0x07, 0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x0A, 0x01, 0x73, 0x0A, 0x01, 0x00, 0x5E,
            0x31, 0x00, 0x07, 0x00, 0x01, 0x00, 0x01, 0x01, 0x63, 0x5C, 0xF4, 0x00, 0x76, 0x02,
            0x03, 0x62, 0x00, 0x62, 0x00, 0x72, 0x63, 0x02, 0x01, 0x71, 0x01, 0x63, 0xD5, 0x35,
            0x00, 0x00, 0x1B, 0x1B, 0x1B, 0x1B, 0x1A, 0x01, 0xC4, 0x75,
        ];
        let mut decoder =
            sml_rs::transport::decode_streaming::<sml_rs::util::ArrayBuf<2048>>(bytes);
        let expected_response = vec![
            MessageStart {
                transaction_id: &[1],
                group_no: 0,
                abort_on_error: 0,
                message_body: MessageBody::OpenResponse(OpenResponse {
                    client_id: Some(&[67, 76, 78, 73, 68, 49]),
                    req_file_id: &[81, 0, 0, 0, 0, 102, 159, 65, 167],
                    codepage: None,
                    server_id: &[10, 1, 76, 71, 90, 0, 3, 169, 198, 38],
                    ref_time: Some(Time::SecIndex(547552)),
                    sml_version: None,
                }),
            },
            MessageStart {
                transaction_id: &[2],
                group_no: 0,
                abort_on_error: 0,
                message_body: MessageBody::AttentionResponse(AttentionResponse {
                    server_id: &[10, 1, 76, 71, 90, 0, 3, 169, 198, 38],
                    number: AttentionNumber::AttentionErrorCode(
                        AttentionErrorCode::OrderNotExecuted,
                    ),
                    msg: None,
                    details: Some(Tree {
                        parameter_name: &[0x01, 0x00, 0x5E, 0x31, 0x00, 0x07, 0x00, 0x01, 0x00],
                        parameter_value: None,
                        child_list: Box::new(None),
                    }),
                }),
            },
            MessageStart {
                transaction_id: &[3],
                group_no: 0,
                abort_on_error: 0,
                message_body: MessageBody::CloseResponse(CloseResponse {
                    global_signature: None,
                }),
            },
        ];
        while let Some(result) = decoder.next() {
            let input = result.unwrap();
            let parser = streaming::Parser::new(input);
            for (idx, val) in parser.enumerate().into_iter() {
                if let Ok(ParseEvent::MessageStart(val)) = val {
                    assert_eq!(expected_response[idx], val);
                }
            }
        }
    }

    #[test]
    fn attention_response_positive() {
        let bytes = vec![
            0x1B, 0x1B, 0x1B, 0x1B, 0x01, 0x01, 0x01, 0x01, 0x76, 0x02, 0x01, 0x62, 0x00, 0x62,
            0x00, 0x72, 0x63, 0x01, 0x01, 0x76, 0x01, 0x07, 0x43, 0x4C, 0x4E, 0x49, 0x44, 0x31,
            0x0A, 0x51, 0x00, 0x00, 0x00, 0x00, 0x66, 0x9F, 0x64, 0x3C, 0x0B, 0x0A, 0x01, 0x4C,
            0x47, 0x5A, 0x00, 0x03, 0xA9, 0xC6, 0x26, 0x72, 0x62, 0x01, 0x65, 0x00, 0x08, 0x7D,
            0x56, 0x01, 0x63, 0xC2, 0xF2, 0x00, 0x76, 0x02, 0x02, 0x62, 0x00, 0x62, 0x00, 0x72,
            0x63, 0xFF, 0x01, 0x74, 0x0B, 0x0A, 0x01, 0x4C, 0x47, 0x5A, 0x00, 0x03, 0xA9, 0xC6,
            0x26, 0x07, 0x81, 0x81, 0xC7, 0xC7, 0xFD, 0x00, 0x01, 0x73, 0x0A, 0x01, 0x00, 0x5E,
            0x31, 0x00, 0x07, 0x00, 0x01, 0x00, 0x01, 0x01, 0x63, 0x5B, 0xAF, 0x00, 0x76, 0x02,
            0x03, 0x62, 0x00, 0x62, 0x00, 0x72, 0x63, 0x02, 0x01, 0x71, 0x01, 0x63, 0xD5, 0x35,
            0x00, 0x00, 0x1B, 0x1B, 0x1B, 0x1B, 0x1A, 0x01, 0xAC, 0x0C,
        ];
        let expected_response = vec![
            MessageStart {
                transaction_id: &[1],
                group_no: 0,
                abort_on_error: 0,
                message_body: MessageBody::OpenResponse(OpenResponse {
                    client_id: Some(&[67, 76, 78, 73, 68, 49]),
                    req_file_id: &[81, 0, 0, 0, 0, 102, 159, 100, 60],
                    codepage: None,
                    server_id: &[10, 1, 76, 71, 90, 0, 3, 169, 198, 38],
                    ref_time: Some(Time::SecIndex(556374)),
                    sml_version: None,
                }),
            },
            MessageStart {
                transaction_id: &[2],
                group_no: 0,
                abort_on_error: 0,
                message_body: MessageBody::AttentionResponse(AttentionResponse {
                    server_id: &[10, 1, 76, 71, 90, 0, 3, 169, 198, 38],
                    number: AttentionNumber::HintNumber(HintNumber::Positive),
                    msg: None,
                    details: Some(Tree {
                        parameter_name: &[0x01, 0x00, 0x5E, 0x31, 0x00, 0x07, 0x00, 0x01, 0x00],
                        parameter_value: None,
                        child_list: Box::new(None),
                    }),
                }),
            },
            MessageStart {
                transaction_id: &[3],
                group_no: 0,
                abort_on_error: 0,
                message_body: MessageBody::CloseResponse(CloseResponse {
                    global_signature: None,
                }),
            },
        ];
        let mut decoder =
            sml_rs::transport::decode_streaming::<sml_rs::util::ArrayBuf<2048>>(bytes);
        while let Some(result) = decoder.next() {
            let input = result.unwrap();
            let parser = streaming::Parser::new(input);
            for (idx, val) in parser.enumerate().into_iter() {
                if let Ok(ParseEvent::MessageStart(val)) = val {
                    assert_eq!(expected_response[idx], val);
                }
            }
        }
    }
}
