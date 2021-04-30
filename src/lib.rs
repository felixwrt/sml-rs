use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::{all_consuming, map},
    error::{self, make_error, ErrorKind, ParseError},
    IResult,
};

mod num;
mod tlf;
mod octet_string;

pub type IResultComplete<I, O> = Result<O, nom::Err<error::Error<I>>>;

pub(crate) trait SmlParse
where
    Self: Sized,
{
    fn parse(input: &[u8]) -> IResult<&[u8], Self>;

    fn parse_complete(input: &[u8]) -> IResultComplete<&[u8], Self> {
        let res = all_consuming(Self::parse)(input);
        res.map(|(rest, value)| {
            assert!(rest.is_empty());
            value
        })
    }
}

impl<T: SmlParse> SmlParse for Option<T> {
    fn parse(input: &[u8]) -> IResult<&[u8], Self> {
        alt((map(tag(&[0x01u8]), |_| None), map(T::parse, |s| Some(s))))(input)
    }
}

pub fn error<I, E: ParseError<I>>(input: I) -> nom::Err<E> {
    nom::Err::Error(make_error(input, ErrorKind::Alt))
}
