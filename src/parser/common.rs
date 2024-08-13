//! Types used by both parsers.

pub use super::OctetStr;
use super::{
    map, take, take_byte,
    tlf::{Ty, TypeLengthField},
    NumberFormatter, OctetStrFormatter, ParseError, ResTy, SmlParse, SmlParseTlf,
};

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

impl<'i> SmlParseTlf<'i> for OpenResponse<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        *tlf == TypeLengthField::new(Ty::ListOf, 6usize as u32)
    }

    fn parse_with_tlf(input: &'i [u8], _tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let (input, codepage) = <Option<OctetStr<'i>>>::parse(input)?;
        let (input, client_id) = <Option<OctetStr<'i>>>::parse(input)?;
        let (input, req_file_id) = <OctetStr<'i>>::parse(input)?;
        let (input, server_id) = <OctetStr<'i>>::parse(input)?;
        let (input, ref_time) = <Option<Time>>::parse(input)?;
        let (input, sml_version) = <Option<u8>>::parse(input)?;
        let val = OpenResponse {
            codepage,
            client_id,
            req_file_id,
            server_id,
            ref_time,
            sml_version,
        };
        Ok((input, val))
    }
}

impl<'i> core::fmt::Debug for OpenResponse<'i> {
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
            x.field("ref_time", &e);
        }
        if let Some(e) = &self.sml_version {
            x.field("sml_version", &e);
        }
        x.finish()
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

impl<'i> SmlParseTlf<'i> for ListEntry<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        *tlf == TypeLengthField::new(Ty::ListOf, 7usize as u32)
    }

    fn parse_with_tlf(input: &'i [u8], _tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let (input, obj_name) = <OctetStr<'i>>::parse(input)?;
        let (input, status) = <Option<Status>>::parse(input)?;
        let (input, val_time) = <Option<Time>>::parse(input)?;
        let (input, unit) = <Option<Unit>>::parse(input)?;
        let (input, scaler) = <Option<i8>>::parse(input)?;
        let (input, value) = <Value<'i>>::parse(input)?;
        let (input, value_signature) = <Option<Signature<'i>>>::parse(input)?;
        let val = ListEntry {
            obj_name,
            status,
            val_time,
            unit,
            scaler,
            value,
            value_signature,
        };
        Ok((input, val))
    }
}

impl<'i> core::fmt::Debug for ListEntry<'i> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut x = f.debug_struct("ListEntry");
        x.field("obj_name", &OctetStrFormatter(self.obj_name));
        if let Some(e) = &self.status {
            x.field("status", &e);
        }
        if let Some(e) = &self.val_time {
            x.field("val_time", &e);
        }
        if let Some(e) = &self.unit {
            x.field("unit", &e);
        }
        if let Some(e) = &self.scaler {
            x.field("scaler", &e);
        }
        x.field("value", &self.value);
        if let Some(e) = &self.value_signature {
            x.field("value_signature", &e);
        }
        x.finish()
    }
}

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

impl<'i> SmlParseTlf<'i> for Value<'i> {
    fn check_tlf(_tlf: &TypeLengthField) -> bool {
        true
    }

    fn parse_with_tlf(input: &'i [u8], tlf: &TypeLengthField) -> ResTy<'i, Self> {
        match tlf {
            tlf if <bool>::check_tlf(tlf) => map(<bool>::parse_with_tlf(input, tlf), Self::Bool),
            tlf if <OctetStr<'i>>::check_tlf(tlf) => {
                map(<OctetStr<'i>>::parse_with_tlf(input, tlf), Self::Bytes)
            }
            tlf if <i8>::check_tlf(tlf) => map(<i8>::parse_with_tlf(input, tlf), Self::I8),
            tlf if <i16>::check_tlf(tlf) => map(<i16>::parse_with_tlf(input, tlf), Self::I16),
            tlf if <i32>::check_tlf(tlf) => map(<i32>::parse_with_tlf(input, tlf), Self::I32),
            tlf if <i64>::check_tlf(tlf) => map(<i64>::parse_with_tlf(input, tlf), Self::I64),
            tlf if <u8>::check_tlf(tlf) => map(<u8>::parse_with_tlf(input, tlf), Self::U8),
            tlf if <u16>::check_tlf(tlf) => map(<u16>::parse_with_tlf(input, tlf), Self::U16),
            tlf if <u32>::check_tlf(tlf) => map(<u32>::parse_with_tlf(input, tlf), Self::U32),
            tlf if <u64>::check_tlf(tlf) => map(<u64>::parse_with_tlf(input, tlf), Self::U64),
            tlf if <ListType>::check_tlf(tlf) => {
                map(<ListType>::parse_with_tlf(input, tlf), Self::List)
            }
            _ => Err(ParseError::TlfMismatch(core::any::type_name::<Self>())),
        }
    }
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

impl<'i> SmlParseTlf<'i> for ListType {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        tlf.ty == Ty::ListOf && tlf.len == 2
    }

    fn parse_with_tlf(input: &'i [u8], _tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let (input, tag) = u8::parse(input)?;
        match tag {
            1 => {
                let (input, x) = <Time>::parse(input)?;
                Ok((input, ListType::Time(x)))
            }
            _ => Err(ParseError::UnexpectedVariant),
        }
    }
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

impl<'i> SmlParseTlf<'i> for Status {
    fn check_tlf(_tlf: &TypeLengthField) -> bool {
        true
    }

    fn parse_with_tlf(input: &'i [u8], tlf: &TypeLengthField) -> ResTy<'i, Self> {
        match tlf {
            tlf if <u8>::check_tlf(tlf) => map(<u8>::parse_with_tlf(input, tlf), Self::Status8),
            tlf if <u16>::check_tlf(tlf) => map(<u16>::parse_with_tlf(input, tlf), Self::Status16),
            tlf if <u32>::check_tlf(tlf) => map(<u32>::parse_with_tlf(input, tlf), Self::Status32),
            tlf if <u64>::check_tlf(tlf) => map(<u64>::parse_with_tlf(input, tlf), Self::Status64),
            _ => Err(ParseError::TlfMismatch(core::any::type_name::<Self>())),
        }
    }
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
/// `SML_PublicClose.Res` message
pub struct CloseResponse<'i> {
    /// optional signature
    pub global_signature: Option<Signature<'i>>,
}

impl<'i> SmlParseTlf<'i> for CloseResponse<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        *tlf == TypeLengthField::new(Ty::ListOf, 1usize as u32)
    }

    fn parse_with_tlf(input: &'i [u8], _tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let (input, global_signature) = <Option<Signature<'i>>>::parse(input)?;
        let val = CloseResponse { global_signature };
        Ok((input, val))
    }
}

impl<'i> core::fmt::Debug for CloseResponse<'i> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut x = f.debug_struct("CloseResponse");
        if let Some(e) = &self.global_signature {
            x.field("global_signature", &e);
        }
        x.finish()
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

#[derive(PartialEq, Eq, Clone)]
/// SML Time type
pub enum Time {
    /// usually the number of seconds since the power meter was installed
    SecIndex(u32),
}

impl<'i> SmlParseTlf<'i> for Time {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        (tlf.ty == Ty::ListOf && tlf.len == 2) || *tlf == TypeLengthField::new(Ty::Unsigned, 4)
    }

    fn parse_with_tlf(input: &'i [u8], tlf: &TypeLengthField) -> ResTy<'i, Self> {
        // Workaround for Holley DTZ541:
        // For the `Time` type, this meter doesn't respect the spec.
        // Instead of a TLF of type ListOf and length 2, it directly sends an u32 integer,
        // which is encoded by a TLF of Unsigned and length 4 followed by four bytes containing
        // the data.
        if *tlf == TypeLengthField::new(Ty::Unsigned, 4) {
            let (input, bytes) = take::<4>(input)?;
            return Ok((input, Time::SecIndex(u32::from_be_bytes(*bytes))));
        }

        let (input, tag) = u8::parse(input)?;
        match tag {
            1 => {
                let (input, x) = <u32>::parse(input)?;
                Ok((input, Time::SecIndex(x)))
            }
            _ => Err(ParseError::UnexpectedVariant),
        }
    }
}

impl ::core::fmt::Debug for Time {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SecIndex(arg0) => write!(f, "SecIndex({})", arg0),
        }
    }
}

/// SML signature type
pub type Signature<'i> = OctetStr<'i>;

/// Child trees are not supported now.
#[derive(PartialEq, Debug, Eq, Clone)]
pub struct Unsupported;
impl<'i> SmlParse<'i> for Unsupported {
    fn parse(_input: &'i [u8]) -> ResTy<'i, Self> {
        Err(ParseError::NotSupported)
    }
}

/// SML tree
///
/// SML_Tree' can be used to build up individual parameters (leaves or nodes) with their children (for nodes)
/// below them.
/// Specifically, an ‘SML_Tree’ can be used to represent ...
/// ... a single parameter,
/// ... a node with an underlying list of further parameters or
/// ... a node with a list of further sub-trees hanging below it
/// 
/// *Note: SML tree is currently only partially supported. Feel free to open an issue if you need support for more attributes.*
#[derive(PartialEq, Debug, Eq, Clone)]
pub struct Tree<'i> {
    /// Name
    pub parameter_name: OctetStr<'i>,
}

impl<'i> SmlParseTlf<'i> for Tree<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        *tlf == TypeLengthField::new(Ty::ListOf, 3usize as u32)
    }

    fn parse_with_tlf(input: &'i [u8], _tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let (input, parameter_name) = <OctetStr<'i>>::parse(input)?;
        let (input, _parameter_value) = <Option<Unsupported>>::parse(input)?;
        let (input, _child_list) = <Option<Unsupported>>::parse(input)?;

        let val = Self {
            parameter_name,
        };

        Ok((input, val))
    }
}


/// Hint numbers gives information how the message was positive.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum HintNumber {
    /// Ok, positive acknowledgement.
    Positive,
    /// execute later and response will be send via Response-without-request to server address.
    DelayedResponse,
}

impl TryFrom<u8> for HintNumber {
    type Error = ParseError;
    
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::Positive,
            0x01 => Self::DelayedResponse,
            _ => return Err(ParseError::AttentionNumberReserved),
        })
    }
}

/// Attention error codes
///
/// This gives information, what kind of error occurred.
#[derive(PartialEq, Debug, Eq, Clone)]
#[repr(u8)]
pub enum AttentionErrorCode {
    /// Error message that cannot be assigned to any of the meanings defined below.
    UnknownError = 0x00,
    /// Unknown SML identifier.
    UnknownSml = 0x01,
    /// Insufficient authentication, user / password combination invalid.
    InsufficientAuth = 0x02,
    /// Target address (‘serverId’) not available.
    TargetAddressNotAvailable = 0x03,
    /// Request (‘reqFileId’) not available.
    RequestNotAvailable = 0x04,
    /// One or more target attributes cannot be written.
    TargetAttributesNotWritable = 0x05,
    /// One or more target attributes cannot be read.
    TargetAttributesNotReadable = 0x06,
    /// Communication with measuring point disrupted.
    CommunicationDisrupted = 0x07,
    /// Raw data cannot be interpreted.
    RawDataUnreadable = 0x08,
    /// Delivered value outside the allowed value range.
    ValueOutOfRange = 0x09,
    /// Order not executed (e.g. because the supplied ‘parameter-TreePath’
    /// points to a non-existent element).
    OrderNotExecuted = 0x0A,
    /// Checksum incorrect
    ChecksumIncorrect = 0x0B,
    /// Broadcast not supported
    BroadcastNotSupported = 0x0C,
    /// Unexpected SML message (e.g. an SML file without an open request)
    UnexpectedSmlMessage = 0x0D,
    /// Unknown object in the profile
    UnknownObjectInProfile = 0x0E,
    /// Unsupported data type used in a request
    UnsupportedDataType = 0x0F,
    /// Optional element not supported (an element defined as OPTIONAL in SML was
    /// received contrary to the assumption made by the application)
    OptionalElementNotSupported = 0x10,
    /// Requested profile does not have a single entry
    RequestedProfileEmpty = 0x11,
    /// For profile requests: end limit is before start limit
    EndLimitBeforeStartLimit = 0x12,
    /// For profile requests:
    /// There are no entries in the requested area.
    /// At least one entry exists in other areas
    NoEntriesInRequestedArea = 0x13,
    /// An SML file ended without an SML close message.
    SmlFileNoClose = 0x14,
    /// For profile requests: the profile cannot be output temporarily
    /// (for example, because it is being reorganized at the time of the request or a
    /// signature is to be calculated for the profile entry)
    ProfileCannotBeOutputTemporarily = 0x15,
}

impl TryFrom<u8> for AttentionErrorCode {
    type Error = ParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0x00 => Self::UnknownError,
            0x01 => Self::UnknownSml,
            0x02 => Self::InsufficientAuth,
            0x03 => Self::TargetAddressNotAvailable,
            0x04 => Self::RequestNotAvailable,
            0x05 => Self::TargetAttributesNotWritable,
            0x06 => Self::TargetAttributesNotReadable,
            0x07 => Self::CommunicationDisrupted,
            0x08 => Self::RawDataUnreadable,
            0x09 => Self::ValueOutOfRange,
            0x0A => Self::OrderNotExecuted,
            0x0B => Self::ChecksumIncorrect,
            0x0C => Self::BroadcastNotSupported,
            0x0D => Self::UnexpectedSmlMessage,
            0x0E => Self::UnknownObjectInProfile,
            0x0F => Self::UnsupportedDataType,
            0x10 => Self::OptionalElementNotSupported,
            0x11 => Self::RequestedProfileEmpty,
            0x12 => Self::EndLimitBeforeStartLimit,
            0x13 => Self::NoEntriesInRequestedArea,
            0x14 => Self::SmlFileNoClose,
            0x15 => Self::ProfileCannotBeOutputTemporarily,
            _ => return Err(ParseError::AttentionNumberReserved)
        })
    }
}

/// Attention number
/// 
/// Attention numbers are a sequence of 6 bytes.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum AttentionNumber {
    /// Application specific error code
    ApplicationSpecific([u8; 2]),
    /// Hint number
    HintNumber(HintNumber),
    /// Error codes
    AttentionErrorCode(AttentionErrorCode),
}

impl<'i> SmlParseTlf<'i> for AttentionNumber {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        OctetStr::check_tlf(tlf)
    }

    fn parse_with_tlf(input: &'i [u8], tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let (input, octet_str) = OctetStr::parse_with_tlf(input, tlf)?;

        let &[0x81, 0x81, 0xC7, 0xC7, x, y] = octet_str else {
            return Err(ParseError::AttentionNumberReserved)
        };
        let val = match x {
            0xE0..=0xFC => Self::ApplicationSpecific([x, y]),
            0xFD => Self::HintNumber(HintNumber::try_from(y)?),
            0xFE => Self::AttentionErrorCode(AttentionErrorCode::try_from(y)?),
            _ => return Err(ParseError::AttentionNumberReserved)
        };
        
        Ok((input, val))
    }
}

/// Attention response
#[derive(PartialEq, Eq, Clone)]
pub struct AttentionResponse<'i> {
    /// Server id
    pub server_id: OctetStr<'i>,
    /// Attention number
    pub number: AttentionNumber,
    /// message
    pub msg: Option<OctetStr<'i>>,
    /// Details of the attention response
    pub details: Option<Tree<'i>>,
}

impl<'i> SmlParseTlf<'i> for AttentionResponse<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        *tlf == TypeLengthField::new(Ty::ListOf, 4usize as u32)
    }

    fn parse_with_tlf(input: &'i [u8], _tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let (input, server_id) = <OctetStr<'i>>::parse(input)?;
        let (input, number) = <AttentionNumber>::parse(input)?;
        let (input, msg) = <Option<OctetStr<'i>>>::parse(input)?;
        let (input, details) = <Option<Tree<'i>>>::parse(input)?;

        let val = Self {
            server_id,
            number,
            msg,
            details,
        };
        Ok((input, val))
    }
}

impl<'i> core::fmt::Debug for AttentionResponse<'i> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut x = f.debug_struct("AttentionResponse");

        x.field("server_id", &OctetStrFormatter(self.server_id));
        x.field("number", &self.number);
        if let Some(e) = &self.msg {
            x.field("msg", &OctetStrFormatter(e));
        }
        if let Some(e) = &self.details {
            x.field("details", &e);
        }
        x.finish()
    }
}
