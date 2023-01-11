//! This module contains trivial Debug / SmlParseTlf implementations for several domain types.
//!
//! The code has been generated using procedural macros that are no longer part of the codebase.
//! See branch `proc-macro-codegen` for the macros. The output of the macros has been formatted
//! to match the code style of the rest of the codebase.
//!
//! The procedural macros were removed because of the following reasons:
//! - They require a separate crate `sml-rs-macros`, which means more maintenance effort
//! - They significantly increase compile times (`syn` etc.)
//! - The macro code was as long (~300 loc) as the generated code
//! - Debugging / understanding macro-generated code is difficult

use super::*;

impl<'i> SmlParseTlf<'i> for Time {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        (tlf.ty == tlf::Ty::ListOf && tlf.len == 2)
            || *tlf == tlf::TypeLengthField::new(tlf::Ty::Unsigned, 4)
    }

    fn parse_with_tlf(input: &'i [u8], tlf: &TypeLengthField) -> ResTy<'i, Self> {
        // Workaround for Holley DTZ541:
        // For the `Time` type, this meter doesn't respect the spec.
        // Intead of a TLF of type ListOf and length 2, it directly sends an u32 integer,
        // which is encoded by a TLF of Unsigned and length 4 followed by four bytes containing
        // the data.
        if *tlf == tlf::TypeLengthField::new(tlf::Ty::Unsigned, 4) {
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

impl<'i> Debug for Message<'i> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut x = f.debug_struct("Message");
        x.field("transaction_id", &OctetStrFormatter(self.transaction_id));
        x.field("group_no", &self.group_no);
        x.field("abort_on_error", &self.abort_on_error);
        x.field("message_body", &self.message_body);
        x.finish()
    }
}

impl<'i> SmlParseTlf<'i> for MessageBody<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        tlf.ty == tlf::Ty::ListOf && tlf.len == 2
    }

    fn parse_with_tlf(input: &'i [u8], _tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let (input, tag) = u32::parse(input)?;
        match tag {
            0x00000101 => {
                let (input, x) = <OpenResponse<'i>>::parse(input)?;
                Ok((input, MessageBody::OpenResponse(x)))
            }
            0x00000201 => {
                let (input, x) = <CloseResponse<'i>>::parse(input)?;
                Ok((input, MessageBody::CloseResponse(x)))
            }
            0x00000701 => {
                let (input, x) = <GetListResponse<'i>>::parse(input)?;
                Ok((input, MessageBody::GetListResponse(x)))
            }
            _ => Err(ParseError::UnexpectedVariant),
        }
    }
}
impl<'i> SmlParseTlf<'i> for OpenResponse<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        *tlf == tlf::TypeLengthField::new(tlf::Ty::ListOf, 6usize as u32)
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

impl<'i> SmlParseTlf<'i> for CloseResponse<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        *tlf == tlf::TypeLengthField::new(tlf::Ty::ListOf, 1usize as u32)
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

impl<'i> SmlParseTlf<'i> for GetListResponse<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        *tlf == tlf::TypeLengthField::new(tlf::Ty::ListOf, 7usize as u32)
    }

    fn parse_with_tlf(input: &'i [u8], _tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let (input, client_id) = <Option<OctetStr<'i>>>::parse(input)?;
        let (input, server_id) = <OctetStr<'i>>::parse(input)?;
        let (input, list_name) = <Option<OctetStr<'i>>>::parse(input)?;
        let (input, act_sensor_time) = <Option<Time>>::parse(input)?;
        let (input, val_list) = <List<'i>>::parse(input)?;
        let (input, list_signature) = <Option<Signature<'i>>>::parse(input)?;
        let (input, act_gateway_time) = <Option<Time>>::parse(input)?;
        let val = GetListResponse {
            client_id,
            server_id,
            list_name,
            act_sensor_time,
            val_list,
            list_signature,
            act_gateway_time,
        };
        Ok((input, val))
    }
}

impl<'i> core::fmt::Debug for GetListResponse<'i> {
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
            x.field("act_sensor_time", &e);
        }
        x.field("val_list", &self.val_list);
        if let Some(e) = &self.list_signature {
            x.field("list_signature", &e);
        }
        if let Some(e) = &self.act_gateway_time {
            x.field("act_gateway_time", &e);
        }
        x.finish()
    }
}

impl<'i> SmlParseTlf<'i> for ListEntry<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        *tlf == tlf::TypeLengthField::new(tlf::Ty::ListOf, 7usize as u32)
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

impl<'i> SmlParseTlf<'i> for ListType {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        tlf.ty == tlf::Ty::ListOf && tlf.len == 2
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
