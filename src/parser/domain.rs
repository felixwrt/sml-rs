//! SML domain types and their parser implementations.

use super::{
    octet_string::OctetStr, take_byte, tlf::TypeLengthField, NumberFormatter, OctetStrFormatter,
    ParseError, ResTy, SmlParse,
};

#[cfg(feature = "alloc")]
use super::SmlParseTlf;

#[derive(PartialEq, Eq, Clone)]
/// SML Time type
pub enum Time {
    /// usually the number of seconds since the power meter was installed
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
/// Holds multiple `Messages`
pub struct File<'i> {
    /// Vector of `Messsages`
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

        Ok((input, File { messages }))
    }
}

#[cfg(feature = "alloc")]
#[derive(PartialEq, Eq, Clone)]
/// An SML message
pub struct Message<'i> {
    /// transaction identifier
    pub transaction_id: OctetStr<'i>,
    /// allows grouping of SML messages
    pub group_no: u8,
    /// describes how to handle the Message in case of errors
    // this should probably be an enum
    pub abort_on_error: u8,
    /// main content of the message
    pub message_body: MessageBody<'i>,
}

#[cfg(feature = "alloc")]
impl<'i> SmlParse<'i> for Message<'i> {
    fn parse(input: &'i [u8]) -> ResTy<Self> {
        #[allow(clippy::clone_double_ref)]
        let input_orig = input.clone();
        let (input, tlf) = TypeLengthField::parse(input)?;
        if tlf.ty != super::tlf::Ty::ListOf || tlf.len != 6 {
            return Err(ParseError::TlfMismatch("Message"));
        }
        let (input, transaction_id) = OctetStr::parse(input)?;
        let (input, group_no) = u8::parse(input)?;
        let (input, abort_on_error) = u8::parse(input)?;
        let (input, message_body) = MessageBody::parse(input)?;

        let num_bytes_read = input_orig.len() - input.len();

        let (input, crc) = u16::parse(input)?;
        let (input, _) = EndOfSmlMessage::parse(input)?;

        // validate crc16
        let digest = crate::util::CRC_X25
            .checksum(&input_orig[0..num_bytes_read])
            .swap_bytes();
        if digest != crc {
            return Err(ParseError::CrcMismatch);
        }

        let val = Message {
            transaction_id,
            group_no,
            abort_on_error,
            message_body,
        };
        Ok((input, val))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct EndOfSmlMessage;

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
#[derive(PartialEq, Eq, Clone)]
/// SML message body
///
/// Hint: this type only implements the message types specified by SML that are
/// used in real-world power meters.
pub enum MessageBody<'i> {
    /// `SML_PublicOpen.Res` message
    OpenResponse(OpenResponse<'i>),
    /// `SML_PublicClose.Res` message
    CloseResponse(CloseResponse<'i>),
    /// `SML_GetList.Res` message
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

#[derive(PartialEq, Eq, Clone)]
/// `SML_PublicOpen.Res` message
pub struct OpenResponse<'i> {
    /// alternative codepage. Defaults to `ISO 8859-15`
    pub codepage: Option<OctetStr<'i>>,
    /// identification of the client
    pub client_id: Option<OctetStr<'i>>,
    /// identification of the request/response pair
    pub req_file_id: OctetStr<'i>,
    /// identification of the server
    pub server_id: OctetStr<'i>,
    /// reference time
    pub ref_time: Option<Time>,
    /// version of the SML protocol. Defaults to `1`
    pub sml_version: Option<u8>,
}

#[derive(PartialEq, Eq, Clone)]
/// `SML_PublicClose.Res` message
pub struct CloseResponse<'i> {
    /// optional signature
    pub global_signature: Option<Signature<'i>>,
}

/// SML signature type
pub type Signature<'i> = OctetStr<'i>;

#[cfg(feature = "alloc")]
#[derive(PartialEq, Eq, Clone)]
/// `SML_GetList.Res` message
pub struct GetListResponse<'i> {
    /// identification of the client
    pub client_id: Option<OctetStr<'i>>,
    /// identification of the server
    pub server_id: OctetStr<'i>,
    /// identification of the client
    pub list_name: Option<OctetStr<'i>>,
    /// optional sensor time information
    pub act_sensor_time: Option<Time>,
    /// list of data values
    pub val_list: List<'i>,
    /// signature of the list - whatever that means?!
    pub list_signature: Option<Signature<'i>>,
    /// optional gateway time information
    pub act_gateway_time: Option<Time>,
}

#[cfg(feature = "alloc")]
/// vector of SML list entries
pub type List<'i> = alloc::vec::Vec<ListEntry<'i>>;

#[cfg(feature = "alloc")]
impl<'i> SmlParseTlf<'i> for List<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        matches!(tlf.ty, super::tlf::Ty::ListOf)
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

#[derive(PartialEq, Eq, Clone)]
/// SML ListEntry type
pub struct ListEntry<'i> {
    /// name of the entry
    pub obj_name: OctetStr<'i>,
    /// status of the entry, content is unspecified in SML
    pub status: Option<Status>,
    /// time when the value was obtained
    pub val_time: Option<Time>,
    /// code of the value's unit according to DLMS-Unit-List (see IEC 62056-62)
    pub unit: Option<Unit>,
    /// scaler of the value. Calculation: `value = self.value * 10 ^ self.scaler`
    pub scaler: Option<i8>,
    /// the raw value. See `scaler` and `unit` for how to interpret the value
    pub value: Value<'i>,
    /// signature of the value?!
    pub value_signature: Option<Signature<'i>>,
}

#[derive(PartialEq, Eq, Clone)]
/// SML status type. Meaning of status values is not specified in SML.
pub enum Status {
    /// `u8` status
    Status8(u8),
    /// `u16` status
    Status16(u16),
    /// `u32` status
    Status32(u32),
    /// `u64` status
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

/// unit code according to DLMS-Unit-List (see IEC 62056-62)
pub type Unit = u8; // proper enum?

#[derive(PartialEq, Eq, Clone)]
/// SML value type
#[allow(missing_docs)]
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

#[derive(Debug, PartialEq, Eq, Clone)]
/// SML ListType type
pub enum ListType {
    /// variant containing time information
    Time(Time),
}
