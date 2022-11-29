//! An OctetString in SML is a sequence of bytes.

use super::{
    take_n,
    tlf::{Ty, TypeLengthField},
    ResTy, SmlParseTlf,
};

#[cfg(feature = "alloc")]
/// OctetString is the owned version of a sequence of bytes.
pub type OctetString = alloc::vec::Vec<u8>;

#[cfg(feature = "alloc")]
impl<'i> SmlParseTlf<'i> for OctetString {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        OctetStr::check_tlf(tlf)
    }

    fn parse_with_tlf(input: &'i [u8], tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let (input, octet_str) = OctetStr::parse_with_tlf(input, tlf)?;
        Ok((input, octet_str.to_vec()))
    }
}

/// OctetStr is the borrowed version of a sequence of bytes.
pub type OctetStr<'i> = &'i [u8];

impl<'i> SmlParseTlf<'i> for OctetStr<'i> {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        matches!(tlf.ty, Ty::OctetString)
    }

    fn parse_with_tlf(input: &'i [u8], tlf: &TypeLengthField) -> ResTy<'i, Self> {
        take_n(input, tlf.len as usize)
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::parser::SmlParse;
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
        assert_eq!(
            Option::<&[u8]>::parse_complete(b"\x01").expect("Decode Error"),
            None
        );
    }
}
