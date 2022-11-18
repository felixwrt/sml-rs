use super::{error, SmlParse};

use nom::{
    bits::{bits, complete::take as take_bits},
    combinator::map,
    error::{Error, ParseError, context, ContextError},
    sequence::tuple,
    IResult, bytes::complete::take,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct TypeLengthField {
    pub ty: Ty,
    pub len: usize,
}

impl TypeLengthField {
    pub fn new(ty: Ty, len: usize) -> TypeLengthField {
        TypeLengthField { ty, len }
    }
}

impl<'i> SmlParse<'i> for TypeLengthField {
    fn parsex<E>(input: &'i [u8]) -> IResult<&[u8], Self, E> 
    where E: ParseError<&'i [u8]> + ContextError<&'i [u8]> {
        let (mut input, (mut has_more_bytes, ty, mut len)) = context("parsing first byte", tlf_first_byte)(input)?;
        let mut tlf_len = 1;

        // reserved for future usages
        if matches!(ty, Ty::Boolean) && has_more_bytes {
            return Err(error(input)); /*reserved for future usage*/
        }

        while has_more_bytes {
            tlf_len += 1;

            let (input_new, (has_more_bytes_new, len_new)) = context("parsing next byte", tlf_next_byte)(input)?;
            input = input_new;
            has_more_bytes = has_more_bytes_new;

            len = match len.checked_shl(4) {
                Some(l) => l,
                None => {
                    return Err(error(input)); /*Overflow in length field of TLF*/
                }
            };
            len += (len_new & 0b1111) as usize;
        }

        // For some reason, the length of the tlf is part of `len` for primitive types.
        // Therefore, it has to be subtracted here
        if !matches!(ty, Ty::ListOf) {
            len = match len.checked_sub(tlf_len) {
                Some(l) => l,
                None => {
                    return Err(error(input)); /*Specified length is too small*/
                }
            }
        }

        Ok((input, TypeLengthField { ty, len }))
    }
}

fn tlf_byte<'i, E>(input: &'i [u8]) -> IResult<&'i [u8], (bool, u8, usize), E> 
where E: ParseError<&'i [u8]> {
    let (input, bytes) = take(1u16)(input)?;
    let byte = bytes[0];
    let has_more_bytes = (byte & 0x80) > 0;
    let ty = (byte >> 4) & 0x07;
    let len = byte & 0x0f;

    Ok((input, (has_more_bytes, ty, len as usize)))
}

fn tlf_first_byte<'i, E>(input: &'i [u8]) -> IResult<&'i [u8], (bool, Ty, usize), E> 
where E: ParseError<&'i [u8]> {
    let (input, (has_more_bytes, ty, len)) = tlf_byte(input)?;
    let ty = Ty::from_byte(ty).map_err(|_| error(input))?;
    Ok((input, (has_more_bytes, ty, len)))
}

fn tlf_next_byte<'i, E>(input: &'i [u8]) -> IResult<&'i [u8], (bool, usize), E> 
where E: ParseError<&'i [u8]> {
    let (input, (has_more_bytes, ty, len)) = tlf_byte(input)?;
    if ty != 0x00 {
        return Err(error(input));
    }
    Ok((input, (has_more_bytes, len)))
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Ty {
    OctetString,
    Boolean,
    Integer,
    Unsigned,
    ListOf,
}

impl Ty {
    fn from_byte(ty_num: u8) -> Result<Ty, ()> {
        Ok(match ty_num {
            0b000 => Ty::OctetString,
            0b100 => Ty::Boolean,
            0b101 => Ty::Integer,
            0b110 => Ty::Unsigned,
            0b111 => Ty::ListOf,
            _ => {
                return Err(()); /*invalid type bit*/
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_types() {
        let test_cases = [
            (&[0b0000_0001], TypeLengthField::new(Ty::OctetString, 0)),
            (&[0b0100_0001], TypeLengthField::new(Ty::Boolean, 0)),
            (&[0b0101_0001], TypeLengthField::new(Ty::Integer, 0)),
            (&[0b0110_0001], TypeLengthField::new(Ty::Unsigned, 0)),
            (&[0b0111_0000], TypeLengthField::new(Ty::ListOf, 0)),
        ];

        test_cases.iter().for_each(|(input, exp)| {
            assert_eq!(
                &TypeLengthField::parse_complete(*input).expect("Decode error"),
                exp
            )
        });
    }

    #[test]
    fn reserved() {
        // single-byte
        assert!(TypeLengthField::parse(&[0b1100_0000]).is_err());
        assert!(TypeLengthField::parse(&[0b0001_0000]).is_err());
        assert!(TypeLengthField::parse(&[0b0010_0000]).is_err());
        assert!(TypeLengthField::parse(&[0b0011_0000]).is_err());
        assert!(TypeLengthField::parse(&[0b1001_0000]).is_err());
        assert!(TypeLengthField::parse(&[0b1010_0000]).is_err());
        assert!(TypeLengthField::parse(&[0b1011_0000]).is_err());

        // multi-byte
        assert!(TypeLengthField::parse(&[0b1000_0010, 0b0001_0000]).is_err());
        assert!(TypeLengthField::parse(&[0b1000_0010, 0b0010_0000]).is_err());
        assert!(TypeLengthField::parse(&[0b1000_0010, 0b0011_0000]).is_err());
        assert!(TypeLengthField::parse(&[0b1000_0010, 0b0101_0000]).is_err());
        assert!(TypeLengthField::parse(&[0b1000_0010, 0b0110_0000]).is_err());
        assert!(TypeLengthField::parse(&[0b1000_0010, 0b0111_0000]).is_err());
    }

    #[test]
    fn len_single_byte() {
        // for primitive data types, the tlf length is part of the length field.
        // for complex data types, it is not.

        // single-byte tlf for primitive type
        assert_eq!(
            TypeLengthField::parse_complete(&[0b0000_0001]).expect("Decode error"),
            TypeLengthField::new(Ty::OctetString, 0)
        );
        assert_eq!(
            TypeLengthField::parse_complete(&[0b0000_1000]).expect("Decode error"),
            TypeLengthField::new(Ty::OctetString, 7)
        );
        assert_eq!(
            TypeLengthField::parse_complete(&[0b0000_1111]).expect("Decode error"),
            TypeLengthField::new(Ty::OctetString, 14)
        );
        // length 0 for primitive types is an error
        assert!(TypeLengthField::parse(&[0b0000_0000]).is_err());

        // single-byte tlf for complex type
        assert_eq!(
            TypeLengthField::parse_complete(&[0b0111_0000]).expect("Decode error"),
            TypeLengthField::new(Ty::ListOf, 0)
        );
        assert_eq!(
            TypeLengthField::parse_complete(&[0b0111_1000]).expect("Decode error"),
            TypeLengthField::new(Ty::ListOf, 8)
        );
        assert_eq!(
            TypeLengthField::parse_complete(&[0b0111_1111]).expect("Decode error"),
            TypeLengthField::new(Ty::ListOf, 15)
        );
    }

    #[test]
    fn len_multi_byte() {
        // for primitive data types, the tlf length is part of the length field.
        // for complex data types, it is not.

        // multi-byte tlf for primitive type
        assert_eq!(
            TypeLengthField::parse_complete(&[0b1000_0010, 0b0000_0011]).expect("Decode error"),
            TypeLengthField::new(Ty::OctetString, 0b0010_0011 - 2)
        );
        assert_eq!(
            TypeLengthField::parse_complete(&[0b1000_0010, 0b1000_0011, 0b0000_1111])
                .expect("Decode error"),
            TypeLengthField::new(Ty::OctetString, 0b0010_0011_1111 - 3)
        );

        // multi-byte tlf for complex type
        assert_eq!(
            TypeLengthField::parse_complete(&[0b1111_0010, 0b0000_0011]).expect("Decode error"),
            TypeLengthField::new(Ty::ListOf, 0b0010_0011)
        );
        assert_eq!(
            TypeLengthField::parse_complete(&[0b1111_0010, 0b1000_0011, 0b0000_1111])
                .expect("Decode error"),
            TypeLengthField::new(Ty::ListOf, 0b0010_0011_1111)
        );

        TypeLengthField::parse_complete(&[0b1111_0010, 0b1000_0011, 0b1000_1111]);
    }
}
