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
