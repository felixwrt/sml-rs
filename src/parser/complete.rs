//! Simple to use SML parser that uses dynamic memory allocations (requires `alloc` feature)
//!
//! # Examples
//!
//! ```
//! use sml_rs::parser::{complete::{parse, File, Message, MessageBody}, common::CloseResponse};
//!
//! let bytes = [0x76, 0x5, 0xdd, 0x43, 0x44, 0x0, 0x62, 0x0, 0x62, 0x0, 0x72, 0x63, 0x2, 0x1, 0x71, 0x1, 0x63, 0xfd, 0x56, 0x0];
//!
//! // parse the input data
//! let result = parse(&bytes);
//!
//! let expected = File {
//!     messages: vec![
//!         Message {
//!             transaction_id: &[221, 67, 68, 0],
//!             group_no: 0,
//!             abort_on_error: 0,
//!             message_body: MessageBody::CloseResponse(CloseResponse {
//!                 global_signature: None
//!             })
//!         }
//!     ]
//! };
//! assert_eq!(result, Ok(expected))
//! ```

use alloc::vec::Vec;
use core::fmt::Debug;

use super::{
    common::{CloseResponse, EndOfSmlMessage, ListEntry, OpenResponse, Signature, Time},
    tlf::{Ty, TypeLengthField},
    OctetStr, OctetStrFormatter, ParseError, ResTy, SmlParse, SmlParseTlf,
};

#[derive(Debug, PartialEq, Eq, Clone)]
/// Top-level SML type. Holds multiple `Messages`.
pub struct File<'i> {
    /// Vector of `Messsages`
    pub messages: Vec<Message<'i>>,
}

impl<'i> SmlParse<'i> for File<'i> {
    fn parse(mut input: &'i [u8]) -> ResTy<Self> {
        let mut messages = Vec::new();
        while !input.is_empty() {
            let (new_input, msg) = Message::parse(input)?;
            messages.push(msg);
            input = new_input;
        }

        Ok((input, File { messages }))
    }
}

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

impl<'i> SmlParse<'i> for Message<'i> {
    fn parse(input: &'i [u8]) -> ResTy<Self> {
        let input_orig = input;
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

impl<'i> SmlParseTlf<'i> for MessageBody<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        tlf.ty == Ty::ListOf && tlf.len == 2
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

#[derive(PartialEq, Eq, Clone)]
/// `SML_GetList.Res` message
pub struct GetListResponse<'i> {
    /// identification of the client
    pub client_id: Option<OctetStr<'i>>,
    /// identification of the server
    pub server_id: OctetStr<'i>,
    /// name of the list
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

impl<'i> SmlParseTlf<'i> for GetListResponse<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        *tlf == TypeLengthField::new(Ty::ListOf, 7usize as u32)
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

/// Vector of SML list entries
pub type List<'i> = Vec<ListEntry<'i>>;

impl<'i> SmlParseTlf<'i> for List<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        matches!(tlf.ty, super::tlf::Ty::ListOf)
    }

    fn parse_with_tlf(mut input: &'i [u8], tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let mut v = Vec::with_capacity(tlf.len as usize);
        for _ in 0..tlf.len {
            let (new_input, x) = ListEntry::parse(input)?;
            v.push(x);
            input = new_input;
        }
        Ok((input, v))
    }
}

/// Parses a slice of bytes into an SML File.
///
/// *This function is available only if sml-rs is built with the `"alloc"` feature.*
pub fn parse(input: &[u8]) -> Result<File, ParseError> {
    File::parse_complete(input)
}
