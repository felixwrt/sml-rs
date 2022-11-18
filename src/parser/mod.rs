pub mod tlf;

use nom::{
    combinator::all_consuming,
    error::{self, ParseError, make_error, ErrorKind, VerboseError, ContextError},
    IResult,
};

pub type IResultComplete<I, O> = Result<O, nom::Err<error::Error<I>>>;

pub trait SmlParse<'i>
where
    Self: Sized + std::fmt::Debug,
{
    fn parse(input: &'i [u8]) -> IResult<&'i [u8], Self>;

    fn parse_complete(input: &'i [u8]) -> IResultComplete<&[u8], Self> {
        let res = all_consuming(Self::parse)(input);
        res.map(|(rest, value)| {
            assert!(rest.is_empty());
            value
        })
    }
}

pub fn error<I, E: ParseError<I>>(input: I) -> nom::Err<E> {
    nom::Err::Error(make_error(input, ErrorKind::Alt))
}