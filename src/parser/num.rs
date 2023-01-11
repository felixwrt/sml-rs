//! Parsers for number types and booleans

use super::{
    map, take_byte, take_n,
    tlf::{Ty, TypeLengthField},
    ResTy, SmlParseTlf,
};

fn parse_num<'i, const SIZE: usize, const IS_SIGNED: bool>(
    input: &'i [u8],
    tlf: &TypeLengthField,
) -> ResTy<'i, [u8; SIZE]> {
    // read bytes
    let (input, bytes) = take_n(input, tlf.len as usize)?;

    // determine fill bytes depending on the type and sign of the number
    let fill_byte = if IS_SIGNED {
        let is_negative = bytes[0] > 0x7F;
        if is_negative {
            0xFF
        } else {
            0x00
        }
    } else {
        0x00
    };

    // initialize buffer of Self's size with fill bytes
    let mut buffer = [fill_byte; SIZE];

    // copy read bytes into the buffer
    let num_skipped_bytes = SIZE - tlf.len as usize;
    buffer[num_skipped_bytes..].copy_from_slice(bytes);

    Ok((input, buffer))
}

macro_rules! impl_num {
    (($($t:ty),+), $int_ty:expr) => {
        $(
            // compile time check to ensure that the second argument to `impl_num` can only be `Ty::Unsigned` or `Ty::Integer`
            const _: () = if !matches!($int_ty, Ty::Unsigned | Ty::Integer) {
                panic!("impl_num used with invalid type argument. Only Ty::Unsigned and Ty::Integer are allowed");
            };

            impl<'i> SmlParseTlf<'i> for $t {
                fn check_tlf(tlf: &TypeLengthField) -> bool {
                    // size of the number type (in bytes)
                    const SIZE: usize = core::mem::size_of::<$t>();

                    tlf.ty == $int_ty && tlf.len as usize <= SIZE && tlf.len != 0
                }

                fn parse_with_tlf(input: &'i [u8], tlf: &TypeLengthField) -> ResTy<'i, Self> {
                    // size of the number type (in bytes)
                    const SIZE: usize = core::mem::size_of::<$t>();
                    const IS_SIGNED: bool = matches!($int_ty, Ty::Integer);

                    map(parse_num::<SIZE, IS_SIGNED>(input, tlf), Self::from_be_bytes)
                }
            }
        )+
    };
}

impl_num!((u8, u16, u32, u64), Ty::Unsigned);
impl_num!((i8, i16, i32, i64), Ty::Integer);

// Boolean
impl<'i> SmlParseTlf<'i> for bool {
    fn check_tlf(tlf: &TypeLengthField) -> bool {
        *tlf == TypeLengthField::new(Ty::Boolean, core::mem::size_of::<Self>() as u32)
    }

    fn parse_with_tlf(input: &'i [u8], _tlf: &TypeLengthField) -> ResTy<'i, Self> {
        let (input, b) = take_byte(input)?;
        Ok((input, b > 0))
    }
}

#[cfg(test)]
mod test {
    use crate::parser::SmlParse;

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
    fn parse_nums_error() {
        // tlf if for u32, but we're trying to parse as u8
        assert!(u8::parse_complete(&[0x65, 0x0, 0x0, 0x0, 0x1]).is_err());
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
