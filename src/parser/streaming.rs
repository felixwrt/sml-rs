#![allow(missing_docs)]
use sml_rs_macros::{CompactDebug, SmlParse};

use crate::util::CRC_X25;

use super::{
    domain::{CloseResponse, EndOfSmlMessage, ListEntry, OpenResponse, Signature, Time},
    octet_string::OctetStr,
    tlf::{Ty, TypeLengthField},
    ParseError, ResTy, SmlParse,
};

pub struct ParseState<'i> {
    input: &'i [u8],
    msg_input: &'i [u8],
    pending_list_entries: u32,
}

impl<'i> ParseState<'i> {
    pub fn new(input: &'i [u8]) -> Self {
        ParseState {
            input,
            msg_input: &[],
            pending_list_entries: 0,
        }
    }

    fn parse_next(&mut self) -> Result<Option<IterResult<'i>>, ParseError> {
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
                IterResult::MessageStart(msg)
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
                IterResult::GetListResponseEnd(glre)
            }
            x => {
                let (input, le) = ListEntry::parse(self.input)?;
                self.input = input;
                self.pending_list_entries = x - 1;
                IterResult::ListEntry(le)
            }
        }))
    }

    #[cfg(feature = "alloc")]
    pub fn collect(self) -> Result<super::domain::File<'i>, ParseError> {
        use super::domain;
        use crate::parser::domain::GetListResponse;

        let mut msgs = alloc::vec::Vec::new();

        for res in self {
            let res = dbg!(res)?;
            match res {
                IterResult::MessageStart(msg) => {
                    let body = match msg.message_body {
                        MessageBody::OpenResponse(x) => domain::MessageBody::OpenResponse(x),
                        MessageBody::CloseResponse(x) => domain::MessageBody::CloseResponse(x),
                        MessageBody::GetListResponse(x) => {
                            domain::MessageBody::GetListResponse(GetListResponse {
                                client_id: x.client_id,
                                server_id: x.server_id,
                                list_name: x.list_name,
                                act_sensor_time: x.act_sensor_time,
                                val_list: alloc::vec::Vec::with_capacity(x.num_vals as usize),
                                list_signature: None,
                                act_gateway_time: None,
                            })
                        }
                    };
                    let res = domain::Message {
                        transaction_id: msg.transaction_id,
                        group_no: msg.group_no,
                        abort_on_error: msg.abort_on_error,
                        message_body: body,
                    };
                    msgs.push(res);
                }
                IterResult::GetListResponseEnd(x) => match msgs.last_mut() {
                    Some(domain::Message {
                        message_body: domain::MessageBody::GetListResponse(glr),
                        ..
                    }) => {
                        glr.list_signature = x.list_signature;
                        glr.act_gateway_time = x.act_gateway_time;
                    }
                    _ => unreachable!(),
                },
                IterResult::ListEntry(x) => match msgs.last_mut() {
                    Some(domain::Message {
                        message_body: domain::MessageBody::GetListResponse(glr),
                        ..
                    }) => {
                        glr.val_list.push(x);
                    }
                    _ => unreachable!(),
                },
            }
        }

        Ok(domain::File { messages: msgs })
    }
}

impl<'i> Iterator for ParseState<'i> {
    type Item = Result<IterResult<'i>, ParseError>;

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

#[derive(Debug)]
pub enum IterResult<'i> {
    MessageStart(MessageStart<'i>),
    GetListResponseEnd(GetListResponseEnd<'i>),
    ListEntry(ListEntry<'i>),
}

#[derive(PartialEq, Eq, Clone, CompactDebug)]
pub struct MessageStart<'i> {
    pub transaction_id: OctetStr<'i>,
    pub group_no: u8,
    pub abort_on_error: u8, // this should probably be an enum
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

#[derive(PartialEq, Eq, Clone, SmlParse)]
pub enum MessageBody<'i> {
    #[tag(0x00000101)]
    OpenResponse(OpenResponse<'i>),
    #[tag(0x00000201)]
    CloseResponse(CloseResponse<'i>),
    #[tag(0x00000701)]
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

#[derive(PartialEq, Eq, Clone, CompactDebug)]
pub struct GetListResponseStart<'i> {
    pub client_id: Option<OctetStr<'i>>,
    pub server_id: OctetStr<'i>,
    pub list_name: Option<OctetStr<'i>>,
    pub act_sensor_time: Option<Time>,
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

#[derive(PartialEq, Eq, Clone, CompactDebug)]
pub struct GetListResponseEnd<'i> {
    pub list_signature: Option<Signature<'i>>,
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
