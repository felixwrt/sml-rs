//! A Type-Length-Field is a building block for many SML data structures.

use crate::parser::ParseError;

use super::{take_byte, SmlParse};

use super::ResTy;

/// Error type used when parsing a `TypeLengthField`
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TlfParseError {
    /// The length field of a TLF overflowed
    TlfLengthOverflow,
    /// The TLF uses values reserved for future usage
    TlfReserved,
    /// The length field of a TLF underflowed
    TlfLengthUnderflow,
    /// The type field of a byte following the first TLF byte isn't set to `000`
    TlfNextByteTypeMismatch,
    /// The TLF's type field contains an invalid value
    TlfInvalidTy,
}

impl From<TlfParseError> for ParseError {
    fn from(x: TlfParseError) -> Self {
        ParseError::InvalidTlf(x)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(crate) struct TypeLengthField {
    pub ty: Ty,
    pub len: u32,
}

impl TypeLengthField {
    #[allow(unused)]
    pub(crate) fn new(ty: Ty, len: u32) -> TypeLengthField {
        TypeLengthField { ty, len }
    }
}

impl<'i> SmlParse<'i> for TypeLengthField {
    fn parse(input: &[u8]) -> ResTy<Self> {
        let (mut input, (mut has_more_bytes, ty, mut len)) = tlf_first_byte(input)?;
        let mut tlf_len = 1;

        // reserved for future usages
        if matches!(ty, Ty::Boolean) && has_more_bytes {
            return Err(TlfParseError::TlfReserved.into());
        }

        while has_more_bytes {
            tlf_len += 1;

            let (input_new, (has_more_bytes_new, len_new)) = tlf_next_byte(input)?;
            input = input_new;
            has_more_bytes = has_more_bytes_new;

            len = match len.checked_shl(4) {
                Some(l) => l,
                None => {
                    return Err(TlfParseError::TlfLengthOverflow.into());
                }
            };
            len += len_new & 0b1111;
        }

        // For some reason, the length of the tlf is part of `len` for primitive types.
        // Therefore, it has to be subtracted here
        if !matches!(ty, Ty::ListOf) {
            len = match len.checked_sub(tlf_len) {
                Some(l) => l,
                None => {
                    return Err(TlfParseError::TlfLengthUnderflow.into());
                }
            }
        }

        Ok((input, TypeLengthField { ty, len }))
    }
}

fn tlf_byte(input: &[u8]) -> ResTy<(bool, u8, u32)> {
    let (input, b) = take_byte(input)?;
    let len = b & 0x0F;
    let ty = (b >> 4) & 0x07;
    let has_more_bytes = (b & 0x80) != 0;
    Ok((input, (has_more_bytes, ty, len as u32)))
}

fn tlf_first_byte(input: &[u8]) -> ResTy<(bool, Ty, u32)> {
    let (input, (has_more_bytes, ty, len)) = tlf_byte(input)?;
    let ty = Ty::from_byte(ty)?;
    Ok((input, (has_more_bytes, ty, len)))
}

fn tlf_next_byte(input: &[u8]) -> ResTy<(bool, u32)> {
    let (input, (has_more_bytes, ty, len)) = tlf_byte(input)?;
    if ty != 0x00 {
        return Err(TlfParseError::TlfNextByteTypeMismatch.into());
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
    fn from_byte(ty_num: u8) -> Result<Ty, ParseError> {
        Ok(match ty_num {
            0b000 => Ty::OctetString,
            0b100 => Ty::Boolean,
            0b101 => Ty::Integer,
            0b110 => Ty::Unsigned,
            0b111 => Ty::ListOf,
            _ => {
                return Err(TlfParseError::TlfInvalidTy.into());
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
    }
}
