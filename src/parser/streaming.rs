//! Flexible parser that doesn't require dynamic memory allocations.
//!
//!

use crate::util::CRC_X25;

use super::{
    common::{CloseResponse, EndOfSmlMessage, ListEntry, OpenResponse, Signature, Time},
    octet_string::OctetStr,
    tlf::{self, Ty, TypeLengthField},
    OctetStrFormatter, ParseError, ResTy, SmlParse, SmlParseTlf,
};

/// Incremental parser for SML messages.
///
/// See the `parser` module for a discussion of the differences between the different parsers.
pub struct Parser<'i> {
    input: &'i [u8],
    msg_input: &'i [u8],
    pending_list_entries: u32,
}

impl<'i> Parser<'i> {
    /// Create a new Parser from a slice of bytes.
    pub fn new(input: &'i [u8]) -> Self {
        Parser {
            input,
            msg_input: &[],
            pending_list_entries: 0,
        }
    }

    fn parse_next(&mut self) -> Result<Option<ParseEvent<'i>>, ParseError> {
        if self.input.is_empty() && self.pending_list_entries == 0 {
            return Ok(None);
        }

        Ok(Some(match self.pending_list_entries {
            0 => {
                self.msg_input = self.input;
                let (input, msg) = MessageStart::parse(self.input)?;
                self.input = input;
                if let MessageBody::GetListResponse(glr) = &msg.message_body {
                    self.pending_list_entries = glr.num_vals + 2;
                } else {
                    self.pending_list_entries = 1;
                }
                ParseEvent::MessageStart(msg)
            }
            1 => {
                let num_bytes_read = self.msg_input.len() - self.input.len();

                let (input, crc) = u16::parse(self.input)?;
                let (input, _) = EndOfSmlMessage::parse(input)?;
                self.input = input;

                // validate crc16
                let digest = CRC_X25
                    .checksum(&self.msg_input[0..num_bytes_read])
                    .swap_bytes();
                if digest != crc {
                    return Err(ParseError::CrcMismatch);
                }

                self.pending_list_entries = 0;
                return self.parse_next();
            }
            2 => {
                let (input, glre) = GetListResponseEnd::parse(self.input)?;
                self.input = input;
                self.pending_list_entries = 1;
                ParseEvent::GetListResponseEnd(glre)
            }
            x => {
                let (input, le) = ListEntry::parse(self.input)?;
                self.input = input;
                self.pending_list_entries = x - 1;
                ParseEvent::ListEntry(le)
            }
        }))
    }
}

impl<'i> Iterator for Parser<'i> {
    type Item = Result<ParseEvent<'i>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.parse_next();
        if res.is_err() {
            self.input = &[];
        }
        match res {
            Ok(None) => None,
            Ok(Some(x)) => Some(Ok(x)),
            Err(e) => Some(Err(e)),
        }
    }
}

/// Event data structure produced by the streaming parser.
#[derive(Debug)]
pub enum ParseEvent<'i> {
    /// Start of an SML Message.
    MessageStart(MessageStart<'i>),
    /// End of a GetListResponse message.
    GetListResponseEnd(GetListResponseEnd<'i>),
    /// A single data value.
    ListEntry(ListEntry<'i>),
}

/// Contains the start of an SML message.
///
/// For message types that have a known size (e.g. `OpenResponse`), the `MessageStart` type
/// contains the whole message. For messages with dynamic size (e.g. `GetListResponse`), the
/// `MessageStart` type only contains the data read until the start of the dynamically-sized
/// data. The dynamically-sized elements (`ListEntry` in the case of `GetListResponse`) are
/// returned as separate events by the parser. For some message types (e.g. `GetListResponse`),
/// there's a separate event produced when the message has been parsed completely
/// (`GetListResponseEnd` in case of `GetListResponse`).
#[derive(PartialEq, Eq, Clone)]
pub struct MessageStart<'i> {
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

impl<'i> SmlParse<'i> for MessageStart<'i> {
    fn parse(input: &'i [u8]) -> ResTy<Self> {
        let (input, tlf) = TypeLengthField::parse(input)?;
        if tlf.ty != Ty::ListOf || tlf.len != 6 {
            return Err(ParseError::TlfMismatch("Message"));
        }
        let (input, transaction_id) = OctetStr::parse(input)?;
        let (input, group_no) = u8::parse(input)?;
        let (input, abort_on_error) = u8::parse(input)?;
        let (input, message_body) = MessageBody::parse(input)?;

        let val = MessageStart {
            transaction_id,
            group_no,
            abort_on_error,
            message_body,
        };
        Ok((input, val))
    }
}

impl<'i> core::fmt::Debug for MessageStart<'i> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut x = f.debug_struct("MessageStart");
        x.field("transaction_id", &OctetStrFormatter(self.transaction_id));
        x.field("group_no", &self.group_no);
        x.field("abort_on_error", &self.abort_on_error);
        x.field("message_body", &self.message_body);
        x.finish()
    }
}

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
    /// Start of the `SML_GetList.Res` message
    GetListResponse(GetListResponseStart<'i>),
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
                let (input, x) = <GetListResponseStart<'i>>::parse(input)?;
                Ok((input, MessageBody::GetListResponse(x)))
            }
            _ => Err(ParseError::UnexpectedVariant),
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
/// Start event of a `GetListResponse` message.
pub struct GetListResponseStart<'i> {
    /// identification of the client
    pub client_id: Option<OctetStr<'i>>,
    /// identification of the server
    pub server_id: OctetStr<'i>,
    /// name of the list
    pub list_name: Option<OctetStr<'i>>,
    /// optional sensor time information
    pub act_sensor_time: Option<Time>,
    /// number of data values
    pub num_vals: u32,
}

impl<'i> crate::parser::SmlParseTlf<'i> for GetListResponseStart<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        *tlf == crate::parser::tlf::TypeLengthField::new(
            crate::parser::tlf::Ty::ListOf,
            7usize as u32,
        )
    }
    fn parse_with_tlf(input: &'i [u8], _tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let (input, client_id) = <Option<OctetStr<'i>>>::parse(input)?;
        let (input, server_id) = <OctetStr<'i>>::parse(input)?;
        let (input, list_name) = <Option<OctetStr<'i>>>::parse(input)?;
        let (input, act_sensor_time) = <Option<Time>>::parse(input)?;
        let (input, tlf) = TypeLengthField::parse(input)?;
        if !matches!(tlf.ty, Ty::ListOf) {
            return Err(ParseError::TlfMismatch(core::any::type_name::<Self>()));
        }
        let val = GetListResponseStart {
            client_id,
            server_id,
            list_name,
            act_sensor_time,
            num_vals: tlf.len,
        };
        Ok((input, val))
    }
}

impl<'i> core::fmt::Debug for GetListResponseStart<'i> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut x = f.debug_struct("GetListResponseStart");
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
        x.field("num_values", &self.num_vals);
        x.finish()
    }
}

/// End event of a `GetListResponse` message.
#[derive(PartialEq, Eq, Clone)]
pub struct GetListResponseEnd<'i> {
    /// signature of the list - whatever that means?!
    pub list_signature: Option<Signature<'i>>,
    /// optional gateway time information
    pub act_gateway_time: Option<Time>,
}

impl<'i> crate::parser::SmlParse<'i> for GetListResponseEnd<'i> {
    fn parse(input: &'i [u8]) -> ResTy<'i, Self> {
        let (input, list_signature) = <Option<Signature<'i>>>::parse(input)?;
        let (input, act_gateway_time) = <Option<Time>>::parse(input)?;
        let val = GetListResponseEnd {
            list_signature,
            act_gateway_time,
        };
        Ok((input, val))
    }
}

impl<'i> core::fmt::Debug for GetListResponseEnd<'i> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        let mut x = f.debug_struct("GetListResponseEnd");
        if let Some(e) = &self.list_signature {
            x.field("list_signature", &e);
        }
        if let Some(e) = &self.act_gateway_time {
            x.field("act_gateway_time", &e);
        }
        x.finish()
    }
}
