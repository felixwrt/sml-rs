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
mod transport;

pub use crate::octet_string::OctetString;

pub type IResultComplete<I, O> = Result<O, nom::Err<error::Error<I>>>;

pub trait SmlParse
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
pub enum Time {
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
pub struct OpenResponse {
    codepage: Option<OctetString>,
    client_id: Option<OctetString>,
    req_file_id: OctetString,
    server_id: OctetString,
    ref_time: Option<Time>,
    sml_version: Option<u8>,
}

#[derive(Debug, PartialEq, Eq, Clone, SmlParse)]
pub struct CloseRequest {
    global_signature: Option<Signature>,
}

#[derive(Debug, PartialEq, Eq, Clone, SmlParse)]
pub struct CloseResponse {
    global_signature: Option<Signature>,
}

type Signature = OctetString;

#[derive(Debug, PartialEq, Eq, Clone, SmlParse)]
pub struct GetListResponse {
    pub client_id: Option<OctetString>,
    pub server_id: OctetString,
    pub list_name: Option<OctetString>,
    pub act_sensor_time: Option<Time>,
    pub val_list: List,
    pub list_signature: Option<Signature>,
    pub act_gateway_time: Option<Time>,
}

pub type List = Vec<ListEntry>;

impl SmlParse for List {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, tlf) = crate::tlf::TypeLengthField::parse(input)?;

        if !matches!(tlf.ty, crate::tlf::Ty::ListOf) {
            return Err(error(input));
        }

        nom::multi::many_m_n(tlf.len, tlf.len, ListEntry::parse)(input)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, SmlParse)]
pub struct ListEntry {
    pub obj_name: OctetString,
    pub status: Option<Status>,
    pub val_time: Option<Time>,
    pub unit: Option<Unit>,
    pub scaler: Option<i8>,
    pub value: Value,
    pub value_signature: Option<Signature>,
}

impl ListEntry {
    pub fn value_as_usize(&self) -> usize {
        let val = self.value.as_usize().unwrap();
        match self.scaler {
            Some(x) if x > 0 => {
                val * 10usize.pow(x as u32)
            }
            Some(x) if x < 0 => {
                val / 10usize.pow((-x) as u32)
            }
            _ => val,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Status {
    Status8(u8),
    Status16(u16),
    Status32(u32),
    Status64(u64),
}

impl SmlParse for Status {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        nom::branch::alt((
            map(u8::parse, |n| Status::Status8(n)),
            map(u16::parse, |n| Status::Status16(n)),
            map(u32::parse, |n| Status::Status32(n)),
            map(u64::parse, |n| Status::Status64(n)),
        ))(input)
    }
}

// see IEC 62056-62
pub type Unit = u8; // proper enum?

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Value {
    Bool(bool),
    Bytes(OctetString),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    List(ListType),
}

impl Value {
    pub fn as_usize(&self) -> Option<usize> {
        match self {
            Value::U8(n) => Some(*n as usize),
            Value::U16(n) => Some(*n as usize),
            Value::U32(n) => Some(*n as usize),
            Value::U64(n) => Some(*n as usize),
            // FIXME: converting signed ints into unsigned here doesn't look very good. 
            Value::I8(n) => Some(*n as usize),
            Value::I16(n) => Some(*n as usize),
            Value::I32(n) => Some(*n as usize),
            Value::I64(n) => Some(*n as usize),
            _ => None
        }
    }
}

impl SmlParse for Value {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        nom::branch::alt((
            map(bool::parse, |x| Value::Bool(x)),
            map(OctetString::parse, |x| Value::Bytes(x)),
            map(i8::parse, |x| Value::I8(x)),
            map(i16::parse, |x| Value::I16(x)),
            map(i32::parse, |x| Value::I32(x)),
            map(i64::parse, |x| Value::I64(x)),
            map(u8::parse, |x| Value::U8(x)),
            map(u16::parse, |x| Value::U16(x)),
            map(u32::parse, |x| Value::U32(x)),
            map(u64::parse, |x| Value::U64(x)),
            map(ListType::parse, |x| Value::List(x)),
        ))(input)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, SmlParse)]
pub enum ListType {
    #[tag(0x01)]
    Time(Time),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct File {
    pub messages: Vec<Message>,
}

impl SmlParse for File {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        map(nom::multi::many1(Message::parse), |msgs| File {
            messages: msgs,
        })(input)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Message {
    pub transaction_id: OctetString,
    pub group_id: u8,
    pub abort_on_error: u8, // this should probably be an enum
    pub message_body: MessageBody,
}

impl SmlParse for Message {
    fn parse(input: &[u8]) -> nom::IResult<&[u8], Self> {
        let input_orig = input.clone();
        let (input, tlf) = tlf::TypeLengthField::parse(input)?;
        if tlf.ty != tlf::Ty::ListOf || tlf.len != 6 {
            return Err(error(input));
        }
        let (input, transaction_id) = OctetString::parse(input)?;
        let (input, group_id) = u8::parse(input)?;
        let (input, abort_on_error) = u8::parse(input)?;
        let (input, message_body) = MessageBody::parse(input)?;
        
        let num_bytes_read = input_orig.len() - input.len();
        
        let (input, crc) = u16::parse(input)?;
        let (input, _) = tag(&[0x00])(input)?;

        // validate crc16
        let digest = crc::crc16::checksum_x25(&input_orig[0..num_bytes_read]).swap_bytes();
        if digest != crc {
            return Err(error(input));
        }

        let val = Message {
            transaction_id,
            group_id,
            abort_on_error,
            message_body,
        };
        Ok((input, val))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EndOfSmlMessage;

impl SmlParse for EndOfSmlMessage {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        map(tag(&[0x00]), |_| EndOfSmlMessage)(input)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, SmlParse)]
pub enum MessageBody {
    #[tag(0x00000100)]
    OpenRequest(OpenRequest),
    #[tag(0x00000101)]
    OpenResponse(OpenResponse),

    #[tag(0x00000200)]
    CloseRequest(CloseRequest),
    #[tag(0x00000201)]
    CloseResponse(CloseResponse),

    // #[tag(0x00000300)]
    // GetProfilePackRequest(GetProfilePackRequest),
    // #[tag(0x00000301)]
    // GetProfilePackResponse(GetProfilePackResponse),

    // #[tag(0x00000400)]
    // GetProfileListRequest(GetProfileListRequest),
    // #[tag(0x00000401)]
    // GetProfileListResponse(GetProfileListResponse),

    // #[tag(0x00000500)]
    // GetProcParameterRequest(GetProcParameterRequest),
    // #[tag(0x00000501)]
    // GetProcParameterResponse(GetProcParameterResponse),

    // #[tag(0x00000600)]
    // SetProcParameterRequest(SetProcParameterRequest),
    // #[tag(0x00000601)]
    // SetProcParameterResponse(SetProcParameterResponse), // removed from the spec?

    // #[tag(0x00000700)]
    // GetListRequest(GetListRequest),
    #[tag(0x00000701)]
    GetListResponse(GetListResponse),

    // #[tag(0x00000800)]
    // GetCosemRequest(GetCosemRequest),
    // #[tag(0x00000801)]
    // GetCosemResponse(GetCosemResponse),

    // #[tag(0x00000900)]
    // SetCosemRequest(SetCosemRequest),
    // #[tag(0x00000901)]
    // SetCosemResponse(SetCosemResponse),

    // #[tag(0x00000A00)]
    // ActionCosemRequest(ActionCosemRequest),
    // #[tag(0x00000A01)]
    // ActionCosemResponse(ActionCosemResponse),

    // #[tag(0x0000FF01)]
    // AttentionResponse(AttentionResponse)
}


pub fn unpack_transport_v1<R: std::io::Read>(reader: &mut R) -> std::io::Result<Vec<u8>> {
    transport::SmlReader::new(reader).read_transmission()
}

pub fn parse_file(bytes: &[u8]) -> Result<File, String> {
    File::parse_complete(bytes)
        .map_err(|e| (format!("Error parsing File: {}", e)))
}


#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;
    #[test]
    fn test_open_result() {
        let input = hex!("760101050021171B0B0A0149534B00047A5544726201650021155A6201");

        let open_response = OpenResponse::parse_complete(&input);
        let exp = OpenResponse {
            codepage: None,
            client_id: None,
            req_file_id: vec![0, 33, 23, 27],
            server_id: vec![10, 1, 73, 83, 75, 0, 4, 122, 85, 68],
            ref_time: Some(Time::SecIndex(2168154)),
            sml_version: Some(1),
        };

        assert_eq!(open_response, Ok(exp))
    }

    #[test]
    fn test_file() {
        let input = hex!("7605006345516200620072630101760101050021171B0B0A0149534B00047A5544726201650021155A620163828E00760500634552620062007263070177010B0A0149534B00047A5544070100620AFFFF726201650021155A757707010060320101010101010449534B0177070100600100FF010101010B0A0149534B00047A55440177070100010800FF650010010401621E52FF65000C13610177070100020800FF0101621E52FF62000177070100100700FF0101621B5200530860010101638E71007605006345536200620072630201710163AD5500");

        let f = File::parse_complete(&input);
        let exp = File {
            messages: vec![
                Message {
                    transaction_id: vec![0, 99, 69, 81],
                    group_id: 0,
                    abort_on_error: 0,
                    message_body: MessageBody::OpenResponse(OpenResponse {
                        codepage: None,
                        client_id: None,
                        req_file_id: vec![0, 33, 23, 27],
                        server_id: vec![10, 1, 73, 83, 75, 0, 4, 122, 85, 68],
                        ref_time: Some(Time::SecIndex(2168154)),
                        sml_version: Some(1),
                    }),
                },
                Message {
                    transaction_id: vec![0, 99, 69, 82],
                    group_id: 0,
                    abort_on_error: 0,
                    message_body: MessageBody::GetListResponse(GetListResponse {
                        client_id: None,
                        server_id: vec![10, 1, 73, 83, 75, 0, 4, 122, 85, 68],
                        list_name: Some(vec![1, 0, 98, 10, 255, 255]),
                        act_sensor_time: Some(Time::SecIndex(2168154)),
                        val_list: vec![
                            ListEntry {
                                obj_name: vec![1, 0, 96, 50, 1, 1],
                                status: None,
                                val_time: None,
                                unit: None,
                                scaler: None,
                                value: Value::Bytes(vec![73, 83, 75]),
                                value_signature: None,
                            },
                            ListEntry {
                                obj_name: vec![1, 0, 96, 1, 0, 255],
                                status: None,
                                val_time: None,
                                unit: None,
                                scaler: None,
                                value: Value::Bytes(vec![10, 1, 73, 83, 75, 0, 4, 122, 85, 68]),
                                value_signature: None,
                            },
                            ListEntry {
                                obj_name: vec![1, 0, 1, 8, 0, 255],
                                status: Some(Status::Status32(1048836)),
                                val_time: None,
                                unit: Some(30),
                                scaler: Some(-1),
                                value: Value::U32(791393),
                                value_signature: None,
                            },
                            ListEntry {
                                obj_name: vec![1, 0, 2, 8, 0, 255],
                                status: None,
                                val_time: None,
                                unit: Some(30),
                                scaler: Some(-1),
                                value: Value::U8(0),
                                value_signature: None,
                            },
                            ListEntry {
                                obj_name: vec![1, 0, 16, 7, 0, 255],
                                status: None,
                                val_time: None,
                                unit: Some(27),
                                scaler: Some(0),
                                value: Value::I16(2144),
                                value_signature: None,
                            },
                        ],
                        list_signature: None,
                        act_gateway_time: None,
                    }),
                },
                Message {
                    transaction_id: vec![0, 99, 69, 83],
                    group_id: 0,
                    abort_on_error: 0,
                    message_body: MessageBody::CloseResponse(CloseResponse {
                        global_signature: None,
                    }),
                },
            ],
        };

        assert_eq!(f, Ok(exp));
    }
}
