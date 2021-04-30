use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::{all_consuming, map},
    error::{self, make_error, ErrorKind, ParseError},
    IResult,
};

use sml_rs_macros::SmlParse;

mod num;
mod octet_string;
mod tlf;

pub use crate::octet_string::OctetString;

pub type IResultComplete<I, O> = Result<O, nom::Err<error::Error<I>>>;

pub(crate) trait SmlParse
where
    Self: Sized,
{
    fn parse(input: &[u8]) -> IResult<&[u8], Self>;

    fn parse_complete(input: &[u8]) -> IResultComplete<&[u8], Self> {
        let res = all_consuming(Self::parse)(input);
        res.map(|(rest, value)| {
            assert!(rest.is_empty());
            value
        })
    }
}

impl<T: SmlParse> SmlParse for Option<T> {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        alt((map(tag(&[0x01u8]), |_| None), map(T::parse, |s| Some(s))))(input)
    }
}

pub fn error<I, E: ParseError<I>>(input: I) -> nom::Err<E> {
    nom::Err::Error(make_error(input, ErrorKind::Alt))
}

type Timestamp = u32; // unix timestamp

#[derive(Debug, PartialEq, Eq, Clone, SmlParse)]
pub struct TimestampLocal {
    // localtime = timestamp + local_offset + season_time_offset
    timestamp: Timestamp,
    local_offset: i16,       // in minutes
    season_time_offset: i16, // in minutes
}

#[derive(Debug, PartialEq, Eq, Clone, SmlParse)]
enum Time {
    #[tag(0x01)]
    SecIndex(u32),
    #[tag(0x02)]
    Timestamp(Timestamp),
    #[tag(0x03)]
    LocalTimestamp(TimestampLocal),
}

#[derive(Debug, PartialEq, Eq, Clone, SmlParse)]
pub struct OpenRequest {
    codepage: Option<OctetString>,
    client_id: OctetString,
    req_file_id: OctetString,
    server_id: Option<OctetString>,
    username: Option<OctetString>,
    password: Option<OctetString>,
    sml_version: Option<u8>,
}

#[derive(Debug, PartialEq, Eq, Clone, SmlParse)]
pub struct OpenResult {
    codepage: Option<OctetString>,
    client_id: Option<OctetString>,
    req_file_id: OctetString,
    server_id: OctetString,
    ref_time: Time,
    sml_version: Option<u8>,
}
