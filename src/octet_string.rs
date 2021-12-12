use nom::{bytes::complete::take, combinator::map, IResult};

use crate::{
    error,
    tlf::{Ty, TypeLengthField},
    SmlParse,
};

// pub type OctetString = Vec<u8>;

// impl<'i> SmlParse<'i> for OctetString {
//     fn parse(input: &'i [u8]) -> IResult<&[u8], Self> {
//         let (input, tlf) = TypeLengthField::parse(input)?;

//         if !matches!(tlf.ty, Ty::OctetString) {
//             return Err(error(input));
//         }

//         map(take(tlf.len), |bytes: &[u8]| bytes.to_vec())(input)
//     }
// }

pub type OctetStr<'a> = &'a[u8];

impl<'a> SmlParse<'a> for OctetStr<'a> {
    fn parse(input: &'a [u8]) -> IResult<&[u8], OctetStr<'a>> {
        let (input, tlf) = TypeLengthField::parse(input)?;

        if !matches!(tlf.ty, Ty::OctetString) {
            return Err(error(input));
        }

        map(take(tlf.len), |bytes: &[u8]| bytes)(input)
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
            OctetStr::parse_complete(&hex!("0648656C6C6F")),
            Ok(&b"Hello"[..])
        );

        // long
        assert_eq!(
            <&[u8]>::parse_complete(b"\x81\x0Cqwertzuiopasdfghjklyxcvbnm"),
            Ok(&b"qwertzuiopasdfghjklyxcvbnm"[..])
        );

        // optional
        assert_eq!(Option::<&[u8]>::parse_complete(b"\x01"), Ok(None));
    }
}
