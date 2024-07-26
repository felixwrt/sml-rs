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
        // Intead of a TLF of type ListOf and length 2, it directly sends an u32 integer,
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

/// Procedere parameter value
/// Not supported now.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum ProcParValue {
    /// value
    Value = 0x01,
    /// Period entry
    PeriodEntry = 0x02,
    /// Tupel Entry
    TupelEntry = 0x03,
    /// sml time
    Time = 0x04,
    /// list entry
    ListEntry = 0x05,
}

impl<'i> SmlParseTlf<'i> for ProcParValue {
    fn check_tlf(_tlf: &TypeLengthField) -> bool {
        false
    }

    fn parse_with_tlf(_input: &'i [u8], _tlf: &TypeLengthField) -> ResTy<'i, Self> {
        Err(ParseError::NotSupported)
    }
}

/// Child trees are not supported now.
#[derive(PartialEq, Debug, Eq, Clone)]
pub struct UnsupportedTree();
impl<'i> SmlParseTlf<'i> for UnsupportedTree {
    fn check_tlf(_tlf: &TypeLengthField) -> bool {
        false
    }

    fn parse_with_tlf(_input: &'i [u8], _tlf: &TypeLengthField) -> ResTy<'i, Self> {
        Err(ParseError::NotSupported)
    }
}

/// SML tree
///
/// SML_Tree' can be used to build up individual parameters (leaves or nodes) with their children (for nodes)
/// below them.
/// Specifically, an ‘SML_Tree’ can be used to ...
/// ... a single parameter,
/// ... a node with an underlying list of further parameters or
/// ... a node with a list of further sub-trees hanging below it
/// can be mapped.
#[derive(PartialEq, Debug, Eq, Clone)]
pub struct Tree<'i> {
    /// Name
    pub parameter_name: OctetStr<'i>,
    /// Value
    pub parameter_value: Option<ProcParValue>,
    /// The child list.
    pub child_list: Option<UnsupportedTree>,
}

impl<'i> SmlParseTlf<'i> for Tree<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        *tlf == TypeLengthField::new(Ty::ListOf, 3usize as u32)
    }

    fn parse_with_tlf(input: &'i [u8], _tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let (input, parameter_name) = <OctetStr<'i>>::parse(input)?;
        let (input, parameter_value) = <Option<ProcParValue>>::parse(input)?;
        let (input, child_list) = <Option<UnsupportedTree>>::parse(input)?;

        let val = Self {
            parameter_name,
            parameter_value,
            child_list,
        };

        Ok((input, val))
    }
}

/// Application specific attention number
///
/// This can be variate from application to application
#[derive(PartialEq, Debug, Eq, Clone)]
pub struct ApplicationSpecific<'i>(OctetStr<'i>);

/// Hint numbers gives informations how the message was positive.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum HintNumber<'i> {
    /// 81 81 C7 C7 FD 00
    ///
    /// Ok, positive acknowledgement.
    Positive,
    /// 81 81 C7 C7 FD 01
    ///
    /// execute lagter and response will be send via Response-without-request to server address.
    ExecuteLater,
    /// Reserved
    Reserved(OctetStr<'i>),
}

impl<'i> From<OctetStr<'i>> for HintNumber<'i> {
    fn from(value: OctetStr<'i>) -> Self {
        match value {
            &[0x81, 0x81, 0xC7, 0xC7, 0xFD, 0x00] => Self::Positive,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFD, 0x01] => Self::ExecuteLater,
            reserved => Self::Reserved(reserved),
        }
    }
}

/// Attention error codes
///
/// This gives information, what kind of error occured.
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum AttentionErrorCode<'i> {
    /// 81 81 C7 C7 FE 00
    ///
    /// Error message that cannot be assigned to any of the meanings defined below.
    UnknownError,
    /// 81 81 C7 C7 FE 01
    ///
    /// Unknown SML identifier.
    UnknownSml,
    /// 81 81 C7 C7 FE 02
    ///
    /// Insufficient authentication, user / password combination invalid.
    InsufficientAuth,
    /// 81 81 C7 C7 FE 03
    ///
    /// Destination address (‘serverId’) not available.
    DestAddressNotAvailable,
    /// 81 81 C7 C7 FE 04
    ///
    /// Request (‘reqFileId’) not available.
    RequestNotAvailable,
    /// 81 81 C7 C7 FE 05
    ///
    /// One or more destination attribute(s) cannot be described.
    DestinationAttributesNotDescribed,
    /// 81 81 C7 C7 FE 06
    ///
    /// One or more target attribute(s) cannot be read.
    TargetAttributesNotDescribed,
    /// 81 81 C7 C7 FE 07
    ///
    /// Communication with measuring point disrupted.
    CommunicationWithMeasuringDisturbed,
    /// 81 81 C7 C7 FE 08
    ///
    /// Raw data cannot be interpreted.
    RawDataCannotInterpreted,
    /// 81 81 C7 C7 FE 09
    ///
    /// Delivered value outside the permissible value range.
    DeliveredValueOutsideValueRange,
    /// 81 81 C7 C7 FE 0A
    ///
    /// Order not executed (e.g. because the supplied ‘parameter-TreePath’
    /// points to a non-existent element).
    OrderNotExecuted,
    /// 81 81 C7 C7 FE 0B
    ///
    /// Checksum incorrect
    ChecksumIncorrect,
    /// 81 81 C7 C7 FE 0C
    ///
    /// Broadcast not supported
    BroadcastNotSupported,
    /// 81 81 C7 C7 FE 0D
    ///
    /// Unexpected SML message (e.g. an SML file without an open request)
    UnexpectedSmlMessage,
    /// 81 81 C7 C7 FE 0E
    ///
    /// Unknown object in the profile (the OBIS code in a profile request refers
    /// to a data source that has not been
    UnknownObjectInProfile,
    /// 81 81 C7 C7 FE 0F
    ///
    /// Unknown object in the profile (the OBIS code in a profile request refers
    /// to a data source that has not been recorded in the profile)
    UnsupportedDataType,
    /// 81 81 C7 C7 FE 10
    ///
    /// Optional element not supported (An element defined as OPTIONAL in SML was
    /// received contrary to the assumption made by the application).
    OptionalElementNotSupported,
    /// 81 81 C7 C7 FE 11
    ///
    /// Requested profile does not have a single entry
    RequestedProfileNoSingleEntry,
    /// 81 81 C7 C7 FE 12
    ///
    /// For profile requests: End limit is before start limit
    EndLimitBeforeStartLimit,
    /// 81 81 C7 C7 FE 13
    /// For profile requests:
    /// There are no entries in the requested area.
    /// At least one entry exists in other areas
    NoEntriesInRequestedArea,
    /// 81 81 C7 C7 FE 14
    ///
    /// An SML file was ended without an SML close.
    SmlFileWasEnded,
    /// 81 81 C7 C7 FE 15
    ///
    /// For profile requests: The profile cannot be output temporarily
    /// (for example, because it is being reorganised at the time of the request or a
    /// signature is to be calculated for the profile entry)
    ProfileCannotBeOutputTemporarily,
    /// Reserved
    Reserved(OctetStr<'i>),
}

impl<'i> From<OctetStr<'i>> for AttentionErrorCode<'i> {
    fn from(value: OctetStr<'i>) -> Self {
        match value {
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x00] => Self::UnknownError,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x01] => Self::UnknownSml,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x02] => Self::InsufficientAuth,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x03] => Self::DestAddressNotAvailable,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x04] => Self::RequestNotAvailable,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x05] => Self::DestinationAttributesNotDescribed,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x06] => Self::TargetAttributesNotDescribed,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x07] => Self::CommunicationWithMeasuringDisturbed,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x08] => Self::RawDataCannotInterpreted,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x09] => Self::DeliveredValueOutsideValueRange,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x0A] => Self::OrderNotExecuted,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x0B] => Self::ChecksumIncorrect,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x0C] => Self::BroadcastNotSupported,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x0D] => Self::UnexpectedSmlMessage,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x0E] => Self::UnknownObjectInProfile,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x0F] => Self::UnsupportedDataType,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x10] => Self::OptionalElementNotSupported,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x11] => Self::RequestedProfileNoSingleEntry,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x12] => Self::EndLimitBeforeStartLimit,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x13] => Self::NoEntriesInRequestedArea,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x14] => Self::SmlFileWasEnded,
            &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x15] => Self::ProfileCannotBeOutputTemporarily,
            reserved => Self::Reserved(reserved),
        }
    }
}

impl<'i> From<AttentionErrorCode<'i>> for OctetStr<'i> {
    fn from(value: AttentionErrorCode<'i>) -> Self {
        match value {
            AttentionErrorCode::UnknownError => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x00],
            AttentionErrorCode::UnknownSml => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x01],
            AttentionErrorCode::InsufficientAuth => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x02],
            AttentionErrorCode::DestAddressNotAvailable => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x03],
            AttentionErrorCode::RequestNotAvailable => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x04],
            AttentionErrorCode::DestinationAttributesNotDescribed => {
                &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x05]
            }
            AttentionErrorCode::TargetAttributesNotDescribed => {
                &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x06]
            }
            AttentionErrorCode::CommunicationWithMeasuringDisturbed => {
                &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x07]
            }
            AttentionErrorCode::RawDataCannotInterpreted => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x08],
            AttentionErrorCode::DeliveredValueOutsideValueRange => {
                &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x09]
            }
            AttentionErrorCode::OrderNotExecuted => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x0A],
            AttentionErrorCode::ChecksumIncorrect => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x0B],
            AttentionErrorCode::BroadcastNotSupported => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x0C],
            AttentionErrorCode::UnexpectedSmlMessage => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x0D],
            AttentionErrorCode::UnknownObjectInProfile => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x0E],
            AttentionErrorCode::UnsupportedDataType => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x0F],
            AttentionErrorCode::OptionalElementNotSupported => {
                &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x10]
            }
            AttentionErrorCode::RequestedProfileNoSingleEntry => {
                &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x11]
            }
            AttentionErrorCode::EndLimitBeforeStartLimit => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x12],
            AttentionErrorCode::NoEntriesInRequestedArea => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x13],
            AttentionErrorCode::SmlFileWasEnded => &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x14],
            AttentionErrorCode::ProfileCannotBeOutputTemporarily => {
                &[0x81, 0x81, 0xC7, 0xC7, 0xFE, 0x15]
            }
            AttentionErrorCode::Reserved(r) => r,
        }
    }
}

/// Attention numbers
#[derive(PartialEq, Debug, Eq, Clone)]
pub enum AttentionNumber<'i> {
    /// Application specific error codes
    ApplicationSpecific(ApplicationSpecific<'i>),
    /// global defined hint numbers
    HintNumber(HintNumber<'i>),
    /// Error codes
    AttentionErrorCode(AttentionErrorCode<'i>),
}

impl<'i> From<OctetStr<'i>> for AttentionNumber<'i> {
    fn from(value: OctetStr<'i>) -> Self {
        let lower_application_specific: &[u8] = &[0x81, 0x81, 0xC7, 0xC7, 0xE0, 0x00];
        let upper_application_specific: &[u8] = &[0x81, 0x81, 0xC7, 0xC7, 0xFC, 0xFF];
        let lower_hintnumber: &[u8] = &[0x81, 0x81, 0xC7, 0xC7, 0xFD, 0x00];
        let upper_hintnumber: &[u8] = &[0x81, 0x81, 0xC7, 0xC7, 0xFD, 0xFF];

        if (lower_application_specific..=upper_application_specific).contains(&value) {
            Self::ApplicationSpecific(ApplicationSpecific(value))
        } else if (lower_hintnumber..=upper_hintnumber).contains(&value) {
            Self::HintNumber(HintNumber::from(value))
        } else {
            Self::AttentionErrorCode(AttentionErrorCode::from(value))
        }
    }
}

/// Attention response
#[derive(PartialEq, Eq, Clone)]
pub struct AttentionResponse<'i> {
    /// Server id
    pub server_id: OctetStr<'i>,
    /// Attention number
    pub number: AttentionNumber<'i>,
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
        let (input, number) = <OctetStr<'i>>::parse(input)?;
        let (input, msg) = <Option<OctetStr<'i>>>::parse(input)?;
        let (input, details) = <Option<Tree<'i>>>::parse(input)?;

        let val = Self {
            server_id,
            number: AttentionNumber::from(number),
            msg,
            details,
        };
        Ok((input, val))
    }
}

impl<'i> core::fmt::Debug for AttentionResponse<'i> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AttentionResponse")
            .field("server_id", &self.server_id)
            .field("number", &self.number)
            .field("msg", &self.msg)
            .field("details", &self.details)
            .finish()
    }
}
