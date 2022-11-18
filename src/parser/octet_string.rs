//! An OctetString in SML is a sequence of bytes.

use crate::parser::ParseError;

use super::{
    tlf::{Ty, TypeLengthField},
    SmlParse, ResTy, take_n,
};


#[cfg(feature = "alloc")]
/// OctetString is the owned version of a sequence of bytes.
pub type OctetString = alloc::vec::Vec<u8>;

#[cfg(feature = "alloc")]
impl<'i> SmlParse<'i> for OctetString {
    fn parse(input: &'i [u8]) -> ResTy<Self> {
        let (input, octet_str) = OctetStr::parse(input)?;
        Ok((input, octet_str.to_vec()))
    }
}

/// OctetStr is the borrowed version of a sequence of bytes.
pub type OctetStr<'i> = &'i [u8];

impl<'i> SmlParse<'i> for OctetStr<'i> {
    fn parse(input: &'i [u8]) -> ResTy<OctetStr<'i>> {
        let (input, tlf) = TypeLengthField::parse(input)?;

        if !matches!(tlf.ty, Ty::OctetString) {
            return Err(ParseError::OctetStrTlfTypeMismatch);
        }

        take_n(input, tlf.len as usize)
    }
}


#[cfg(test)]
mod test {

    use super::*;
    use hex_literal::hex;

    #[test]
    fn test_octet_str() {
        // simple
        assert_eq!(
            OctetStr::parse_complete(&hex!("0648656C6C6F")).expect("Decode Error"),
            &b"Hello"[..]
        );

        // long
        assert_eq!(
            <&[u8]>::parse_complete(b"\x81\x0Cqwertzuiopasdfghjklyxcvbnm").expect("Decode Error"),
            &b"qwertzuiopasdfghjklyxcvbnm"[..]
        );

        // optional
        assert_eq!(Option::<&[u8]>::parse_complete(b"\x01").expect("Decode Error"), None);
    }
}
