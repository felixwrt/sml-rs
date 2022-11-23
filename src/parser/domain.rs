//! SML domain types and their parser implementations.
#![allow(missing_docs)]

use core::fmt::Display;

use sml_rs_macros::SmlParse;

use crate::CRC_X25;

use super::{SmlParse, ResTy, tlf::{TypeLengthField, Ty}, ParseError, octet_string::OctetStr, take_byte, map, OctetStrFormatter};

type Timestamp = u32; // unix timestamp

// NOTE: removed because it doesn't seem to be used in any devices
// #[derive(Debug, PartialEq, Eq, Clone, SmlParse)]
// pub struct TimestampLocal {
//     // localtime = timestamp + local_offset + season_time_offset
//     timestamp: Timestamp,
//     local_offset: i16,       // in minutes
//     season_time_offset: i16, // in minutes
// }

#[derive(PartialEq, Eq, Clone, SmlParse)]
pub enum Time {
    #[tag(0x01)]
    SecIndex(u32),
    // NOTE: removed because it doesn't seem to be used in any devices
    // #[tag(0x02)]
    // Timestamp(Timestamp),
    // #[tag(0x03)]
    // LocalTimestamp(TimestampLocal),
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

#[derive(PartialEq, Eq, Clone)]
pub struct Message<'i> {
    pub transaction_id: OctetStr<'i>,
    pub group_id: u8,
    pub abort_on_error: u8, // this should probably be an enum
    pub message_body: MessageBody<'i>,
}

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

impl<'i> ::core::fmt::Debug for Message<'i> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Message")
            .field("transaction_id", &OctetStrFormatter(self.transaction_id))
            .field("group_id", &self.group_id)
            .field("abort_on_error", &self.abort_on_error)
            .field("message_body", &self.message_body)
        .finish()
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


#[derive(PartialEq, Eq, Clone, SmlParse)]
pub enum MessageBody<'i> {
    #[tag(0x00000101)]
    OpenResponse(OpenResponse<'i>),
    #[tag(0x00000201)]
    CloseResponse(CloseResponse<'i>),
    #[tag(0x00000701)]
    GetListResponse(GetListResponse<'i>),
}

impl<'i> core::fmt::Debug for MessageBody<'i> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OpenResponse(arg0) => arg0.fmt(f),
            Self::CloseResponse(arg0) => arg0.fmt(f),
            Self::GetListResponse(arg0) => arg0.fmt(f),
        }
    }
}

#[derive(PartialEq, Eq, Clone, SmlParse)]
pub struct OpenResponse<'i> {
    codepage: Option<OctetStr<'i>>,
    client_id: Option<OctetStr<'i>>,
    req_file_id: OctetStr<'i>,
    server_id: OctetStr<'i>,
    ref_time: Option<Time>,
    sml_version: Option<u8>,
}

impl<'i> ::core::fmt::Debug for OpenResponse<'i> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut x = f.debug_struct("OpenResponse");
        if let Some(e) = &self.codepage {
            x.field("codepage", &OctetStrFormatter(e));
        }
        if let Some(e) = &self.client_id {
            x.field("client_id", &OctetStrFormatter(e));
        }
        x.field("req_file_id", &OctetStrFormatter(self.req_file_id));
        x.field("server_id", &OctetStrFormatter(self.server_id));
        if let Some(e) = &self.ref_time {
            x.field("ref_time", e);
        }
        if let Some(e) = &self.sml_version {
            x.field("sml_version", e);
        }
        x.finish()
    }
}


#[derive(PartialEq, Eq, Clone, SmlParse)]
pub struct CloseResponse<'i> {
    global_signature: Option<Signature<'i>>,
}

impl<'i> ::core::fmt::Debug for CloseResponse<'i> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut x = f.debug_struct("CloseResponse");
        if let Some(e) = &self.global_signature {
            x.field("global_signature", &OctetStrFormatter(e));
        }
        x.finish()
    }
}

type Signature<'i> = OctetStr<'i>;

#[derive(PartialEq, Eq, Clone, SmlParse)]
pub struct GetListResponse<'i> {
    pub client_id: Option<OctetStr<'i>>,
    pub server_id: OctetStr<'i>,
    pub list_name: Option<OctetStr<'i>>,
    pub act_sensor_time: Option<Time>,
    pub val_list: List<'i>,
    pub list_signature: Option<Signature<'i>>,
    pub act_gateway_time: Option<Time>,
}

impl<'i> ::core::fmt::Debug for GetListResponse<'i> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut x = f.debug_struct("GetListResponse");
        if let Some(e) = &self.client_id {
            x.field("client_id", &OctetStrFormatter(e));
        }
        x.field("server_id", &OctetStrFormatter(self.server_id));
        if let Some(e) = &self.list_name {
            x.field("list_name", &OctetStrFormatter(e));
        }
        if let Some(e) = &self.act_sensor_time {
            x.field("act_sensor_time", e);
        }
        x.field("val_list", &self.val_list);
        if let Some(e) = &self.list_signature {
            x.field("list_signature", e);
        }
        if let Some(e) = &self.act_gateway_time {
            x.field("act_gateway_time", e);
        }
        x.finish()
    }
}


#[cfg(feature = "alloc")]
pub type List<'i> = alloc::vec::Vec<ListEntry<'i>>;

impl<'i> SmlParse<'i> for List<'i> {
    fn parse(input: &'i [u8]) -> ResTy<Self> {
        let (mut input, tlf) = TypeLengthField::parse(input)?;

        if !matches!(tlf.ty, Ty::ListOf) {
            return Err(ParseError::TlfMismatch("List"));
        }

        let mut v = alloc::vec::Vec::with_capacity(tlf.len as usize);
        for _ in 0..tlf.len {
            let (new_input, x) = ListEntry::parse(input)?;
            v.push(x);
            input = new_input;
        }
        Ok((input, v))
    }
}


#[derive(PartialEq, Eq, Clone, SmlParse)]
pub struct ListEntry<'i> {
    pub obj_name: OctetStr<'i>,
    pub status: Option<Status>,
    pub val_time: Option<Time>,
    pub unit: Option<Unit>,
    pub scaler: Option<i8>,
    pub value: Value<'i>,
    pub value_signature: Option<Signature<'i>>,
}

impl<'i> ::core::fmt::Debug for ListEntry<'i> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut x = f.debug_struct("ListEntry");
        x.field("obj_name", &OctetStrFormatter(&self.obj_name));
        if let Some(e) = &self.status {
            x.field("status", e);
        }
        if let Some(e) = &self.val_time {
            x.field("val_time", e);
        }
        if let Some(e) = &self.unit {
            x.field("unit", e);
        }
        if let Some(e) = &self.scaler {
            x.field("scaler", e);
        }
        x.field("value", &self.value);
        if let Some(e) = &self.value_signature {
            x.field("value_signature", &OctetStrFormatter(e));
        }
        x.finish()
    }
}

#[derive(PartialEq, Eq, Clone)]
pub enum Status {
    Status8(u8),
    Status16(u16),
    Status32(u32),
    Status64(u64),
}

impl<'i> SmlParse<'i> for Status {
    fn parse(input: &'i [u8]) -> ResTy<Self> {
        let (_, tlf) = TypeLengthField::parse(input)?;

        if !matches!(tlf.ty, Ty::Unsigned) {
            return Err(ParseError::TlfMismatch("Status1"));
        }

        match tlf.len {
            0x01 => map(u8::parse(input), Status::Status8),
            0x02 => map(u16::parse(input), Status::Status16),
            0x03 | 0x04 => map(u32::parse(input), Status::Status32),
            x if x <= 0x08 => map(u64::parse(input), Status::Status64),
            _ => Err(ParseError::TlfMismatch("Status2"))
        }
    }
}

impl ::core::fmt::Debug for Status {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Status8(x) => write!(f, "{}", x),
            Self::Status16(x) => write!(f, "{}", x),
            Self::Status32(x) => write!(f, "{}", x),
            Self::Status64(x) => write!(f, "{}", x),
        }
    }
}

// see IEC 62056-62
pub type Unit = u8; // proper enum?

#[derive(PartialEq, Eq, Clone)]
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
            Self::I8(arg0) => write!(f, "{:?}", arg0),
            Self::I16(arg0) => write!(f, "{:?}", arg0),
            Self::I32(arg0) => write!(f, "{:?}", arg0),
            Self::I64(arg0) => write!(f, "{:?}", arg0),
            Self::U8(arg0) => write!(f, "{:?}", arg0),
            Self::U16(arg0) => write!(f, "{:?}", arg0),
            Self::U32(arg0) => write!(f, "{:?}", arg0),
            Self::U64(arg0) => write!(f, "{:?}", arg0),
            Self::List(arg0) => write!(f, "{:?}", arg0),
        }
    }
}

impl<'i> SmlParse<'i> for Value<'i> {
    fn parse(input: &'i [u8]) -> ResTy<Self> {
        let (_, tlf) = TypeLengthField::parse(input)?;

        match (tlf.ty, tlf.len) {
            (Ty::Boolean, 1) => map(bool::parse(input), Value::Bool),
            (Ty::OctetString, _) => map(OctetStr::parse(input), Value::Bytes),
            (Ty::Unsigned, 1) => map(u8::parse(input), Value::U8),
            (Ty::Unsigned, 2) => map(u16::parse(input), Value::U16),
            (Ty::Unsigned, 3 | 4) => map(u32::parse(input), Value::U32),
            (Ty::Unsigned, x) if x <= 8 => map(u64::parse(input), Value::U64),
            (Ty::Integer, 1) => map(i8::parse(input), Value::I8),
            (Ty::Integer, 2) => map(i16::parse(input), Value::I16),
            (Ty::Integer, 3 | 4) => map(i32::parse(input), Value::I32),
            (Ty::Integer, x) if x <= 8 => map(i64::parse(input), Value::I64),
            (Ty::ListOf, 2) => map(ListType::parse(input), Value::List),
            _ => Err(ParseError::TlfMismatch("Value"))
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, SmlParse)]
pub enum ListType {
    #[tag(0x01)]
    Time(Time),
}