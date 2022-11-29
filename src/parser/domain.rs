//! SML domain types and their parser implementations.
#![allow(missing_docs)]

use sml_rs_macros::{SmlParse, CompactDebug};

use crate::CRC_X25;

use super::{SmlParse, ResTy, tlf::{TypeLengthField, Ty}, ParseError, octet_string::OctetStr, take_byte, map, OctetStrFormatter, SmlParseTlf, NumberFormatter};

#[derive(PartialEq, Eq, Clone, SmlParse)]
pub enum Time {
    #[tag(0x01)]
    SecIndex(u32),
}

impl ::core::fmt::Debug for Time {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SecIndex(arg0) => write!(f, "SecIndex({})", arg0),
        }
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct File<'i> {
    pub messages: alloc::vec::Vec<Message<'i>>,
}

#[cfg(feature = "alloc")]
impl<'i> SmlParse<'i> for File<'i> {
    fn parse(mut input: &'i [u8]) -> ResTy<Self> {
        let mut messages = alloc::vec::Vec::new();
        while !input.is_empty() {
            let (new_input, msg) = Message::parse(input)?;
            messages.push(msg);
            input = new_input;
        }
        
        Ok((input, File {
            messages
        }))
    }
}

#[cfg(feature = "alloc")]
#[derive(PartialEq, Eq, Clone, CompactDebug)]
pub struct Message<'i> {
    pub transaction_id: OctetStr<'i>,
    pub group_id: u8,
    pub abort_on_error: u8, // this should probably be an enum
    pub message_body: MessageBody<'i>,
}

#[cfg(feature = "alloc")]
impl<'i> SmlParse<'i> for Message<'i> {
    fn parse(input: &'i [u8]) -> ResTy<Self> {
        let input_orig = input.clone();
        let (input, tlf) = TypeLengthField::parse(input)?;
        if tlf.ty != Ty::ListOf || tlf.len != 6 {
            return Err(ParseError::TlfMismatch("Message"));
        }
        let (input, transaction_id) = OctetStr::parse(input)?;
        let (input, group_id) = u8::parse(input)?;
        let (input, abort_on_error) = u8::parse(input)?;
        let (input, message_body) = MessageBody::parse(input)?;
        
        let num_bytes_read = input_orig.len() - input.len();
        
        let (input, crc) = u16::parse(input)?;
        let (input, _) = EndOfSmlMessage::parse(input)?;

        // validate crc16
        let digest = CRC_X25.checksum(&input_orig[0..num_bytes_read]).swap_bytes();
        if digest != crc {
            return Err(ParseError::CrcMismatch);
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

impl<'i> SmlParse<'i> for EndOfSmlMessage {
    fn parse(input: &'i [u8]) -> ResTy<Self> {
        let (input, b) = take_byte(input)?;
        if b != 0x00 {
            return Err(ParseError::MsgEndMismatch);
        }
        Ok((input, EndOfSmlMessage))
    }
}

#[cfg(feature = "alloc")]
#[derive(PartialEq, Eq, Clone, SmlParse)]
pub enum MessageBody<'i> {
    #[tag(0x00000101)]
    OpenResponse(OpenResponse<'i>),
    #[tag(0x00000201)]
    CloseResponse(CloseResponse<'i>),
    #[tag(0x00000701)]
    GetListResponse(GetListResponse<'i>),
}

#[cfg(feature = "alloc")]
impl<'i> core::fmt::Debug for MessageBody<'i> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OpenResponse(arg0) => arg0.fmt(f),
            Self::CloseResponse(arg0) => arg0.fmt(f),
            Self::GetListResponse(arg0) => arg0.fmt(f),
        }
    }
}

#[derive(PartialEq, Eq, Clone, SmlParse, CompactDebug)]
pub struct OpenResponse<'i> {
    codepage: Option<OctetStr<'i>>,
    client_id: Option<OctetStr<'i>>,
    req_file_id: OctetStr<'i>,
    server_id: OctetStr<'i>,
    ref_time: Option<Time>,
    sml_version: Option<u8>,
}

#[derive(PartialEq, Eq, Clone, SmlParse, CompactDebug)]
pub struct CloseResponse<'i> {
    global_signature: Option<Signature<'i>>,
}

pub type Signature<'i> = OctetStr<'i>;

#[cfg(feature = "alloc")]
#[derive(PartialEq, Eq, Clone, SmlParse, CompactDebug)]
pub struct GetListResponse<'i> {
    pub client_id: Option<OctetStr<'i>>,
    pub server_id: OctetStr<'i>,
    pub list_name: Option<OctetStr<'i>>,
    pub act_sensor_time: Option<Time>,
    pub val_list: List<'i>,
    pub list_signature: Option<Signature<'i>>,
    pub act_gateway_time: Option<Time>,
}

#[cfg(feature = "alloc")]
pub type List<'i> = alloc::vec::Vec<ListEntry<'i>>;

#[cfg(feature = "alloc")]
impl<'i> SmlParseTlf<'i> for List<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        matches!(tlf.ty, Ty::ListOf)
    }

    fn parse_with_tlf(mut input: &'i [u8], tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let mut v = alloc::vec::Vec::with_capacity(tlf.len as usize);
        for _ in 0..tlf.len {
            let (new_input, x) = ListEntry::parse(input)?;
            v.push(x);
            input = new_input;
        }
        Ok((input, v))
    }
}


#[derive(PartialEq, Eq, Clone, SmlParse, CompactDebug)]
pub struct ListEntry<'i> {
    pub obj_name: OctetStr<'i>,
    pub status: Option<Status>,
    pub val_time: Option<Time>,
    pub unit: Option<Unit>,
    pub scaler: Option<i8>,
    pub value: Value<'i>,
    pub value_signature: Option<Signature<'i>>,
}

#[derive(PartialEq, Eq, Clone, SmlParse)]
pub enum Status {
    Status8(u8),
    Status16(u16),
    Status32(u32),
    Status64(u64),
}

impl ::core::fmt::Debug for Status {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Status8(x) => write!(f, "{:?}", NumberFormatter(x)),
            Self::Status16(x) => write!(f, "{:?}", NumberFormatter(x)),
            Self::Status32(x) => write!(f, "{:?}", NumberFormatter(x)),
            Self::Status64(x) => write!(f, "{:?}", NumberFormatter(x)),
        }
    }
}

// see IEC 62056-62
pub type Unit = u8; // proper enum?

#[derive(PartialEq, Eq, Clone, SmlParse)]
pub enum Value<'i> {
    Bool(bool),
    Bytes(OctetStr<'i>),
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

impl<'i> core::fmt::Debug for Value<'i> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Bool(arg0) => write!(f, "{:?}", arg0),
            Self::Bytes(arg0) => write!(f, "{:?}", OctetStrFormatter(arg0)),
            Self::I8(arg0) => write!(f, "{:?}", NumberFormatter(arg0)),
            Self::I16(arg0) => write!(f, "{:?}", NumberFormatter(arg0)),
            Self::I32(arg0) => write!(f, "{:?}", NumberFormatter(arg0)),
            Self::I64(arg0) => write!(f, "{:?}", NumberFormatter(arg0)),
            Self::U8(arg0) => write!(f, "{:?}", NumberFormatter(arg0)),
            Self::U16(arg0) => write!(f, "{:?}", NumberFormatter(arg0)),
            Self::U32(arg0) => write!(f, "{:?}", NumberFormatter(arg0)),
            Self::U64(arg0) => write!(f, "{:?}", NumberFormatter(arg0)),
            Self::List(arg0) => write!(f, "{:?}", arg0),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, SmlParse)]
pub enum ListType {
    #[tag(0x01)]
    Time(Time),
}