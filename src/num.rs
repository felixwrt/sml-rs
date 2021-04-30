use crate::error;
use crate::tlf::{Ty, TypeLengthField};
use crate::SmlParse;

use nom::combinator::map;
use nom::number::complete::*;
use nom::IResult;

macro_rules! impl_num {
    (($($t:ty),+), $int_ty:expr) => {
        $(
            impl SmlParse for $t {
                fn parse(input: &[u8]) -> IResult<&[u8], Self> {
                    // size of the number type (in bytes)
                    const SIZE: usize = std::mem::size_of::<$t>();

                    // parse and validate type-length field
                    let (input, tlf) = TypeLengthField::parse(input)?;
                    if tlf.ty != $int_ty || tlf.len > SIZE || tlf.len == 0 {
                        return Err(error(input))
                    }

                    // read bytes
                    let (input, bytes) = nom::bytes::complete::take(tlf.len)(input)?;

                    // determine fill bytes depending on the type and sign of the number
                    let fill_byte = match $int_ty {
                        Ty::Unsigned => 0x00,
                        Ty::Integer => {
                            let is_negative = bytes[0] > 0x7F;
                            if is_negative { 0xFF } else { 0x00 }
                        },
                        _ => panic!("impl_num used with invalid type argument. Only Ty::Unsigned and Ty::Integer are allowed")
                    };

                    // initialize buffer of Self's size with fill bytes
                    let mut buffer = [fill_byte; SIZE];

                    // copy read bytes into the buffer
                    let num_skipped_bytes = SIZE - tlf.len;
                    buffer[num_skipped_bytes..].copy_from_slice(bytes);

                    // turn the buffer into an actual number type
                    let num = Self::from_be_bytes(buffer);

                    Ok((input, num))
                }
            }
        )+
    };
}

impl_num!((u8, u16, u32, u64), Ty::Unsigned);
impl_num!((i8, i16, i32, i64), Ty::Integer);

// Boolean
impl SmlParse for bool {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        let (input, tlf) = TypeLengthField::parse(input)?;

        if tlf != TypeLengthField::new(Ty::Boolean, std::mem::size_of::<Self>()) {
            return Err(error(input));
        }

        map(be_u8, |x: u8| x > 0)(input)
    }
}

#[cfg(test)]
mod test {
    use super::SmlParse;

    #[test]
    fn parse_nums() {
        assert_eq!(u8::parse_complete(&[0x62, 0x05]), Ok(5));
        assert_eq!(u16::parse_complete(&[0x63, 0x01, 0x01]), Ok(257));
        assert_eq!(u32::parse_complete(&[0x65, 0x0, 0x0, 0x0, 0x1]), Ok(1));
        assert_eq!(
            u64::parse_complete(&[0x69, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1]),
            Ok(1)
        );

        assert_eq!(i8::parse_complete(&[0x52, 0xFF]), Ok(-1));
        assert_eq!(i16::parse_complete(&[0x53, 0xEC, 0x78]), Ok(-5000));
        assert_eq!(
            i32::parse_complete(&[0x55, 0xFF, 0xFF, 0xEC, 0x78]),
            Ok(-5000)
        );
        assert_eq!(
            i64::parse_complete(&[0x59, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]),
            Ok(-1)
        );
    }

    #[test]
    fn parse_fewer_bytes() {
        assert_eq!(u32::parse_complete(&[0x64, 0x01, 0x00, 0x01]), Ok(65537));
        assert_eq!(
            u64::parse_complete(&[0x67, 0x0, 0x0, 0x0, 0x0, 0x0, 0x1]),
            Ok(1)
        );
        assert_eq!(u64::parse_complete(&[0x65, 0x0, 0x0, 0x0, 0x1]), Ok(1));
        assert_eq!(u64::parse_complete(&[0x62, 0x1]), Ok(1));

        assert_eq!(i64::parse_complete(&[0x55, 0xFF, 0xFF, 0xFF, 0xFF]), Ok(-1));
        assert_eq!(i16::parse_complete(&[0x52, 0x01]), Ok(1))
    }

    #[test]
    fn parse_optional_num() {
        assert_eq!(Option::<u8>::parse_complete(&[0x01]), Ok(None));
        assert_eq!(Option::<u8>::parse_complete(&[0x62, 0x0F]), Ok(Some(15)));

        assert_eq!(Option::<bool>::parse_complete(&[0x01]), Ok(None));
    }

    #[test]
    fn parse_boolean() {
        assert_eq!(bool::parse_complete(&[0x42, 0x00]), Ok(false));
        for i in 0x01..=0xFF {
            assert_eq!(bool::parse_complete(&[0x42, i]), Ok(true));
        }
    }
}
