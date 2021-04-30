use nom::{bytes::complete::take, combinator::map, IResult};

use crate::{
    error,
    tlf::{Ty, TypeLengthField},
    SmlParse,
};

pub type OctetString = Vec<u8>;

impl SmlParse for OctetString {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, tlf) = TypeLengthField::parse(input)?;

        if !matches!(tlf.ty, Ty::OctetString) {
            return Err(error(input));
        }

        map(take(tlf.len), |bytes: &[u8]| bytes.to_vec())(input)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn test_octet_string() {
        // simple
        assert_eq!(
            OctetString::parse_complete(&hex!("0648656C6C6F")),
            Ok(b"Hello".to_vec())
        );

        // long
        assert_eq!(
            Vec::<u8>::parse_complete(b"\x81\x0Cqwertzuiopasdfghjklyxcvbnm"),
            Ok(b"qwertzuiopasdfghjklyxcvbnm".to_vec())
        );

        // optional
        assert_eq!(Option::<Vec<u8>>::parse_complete(b"\x01"), Ok(None));
    }
}
