pub mod tlf;

use nom::{
    combinator::all_consuming,
    error::{self, ParseError, make_error, ErrorKind, VerboseError, ContextError},
    IResult,
};

pub type IResultComplete<I, O> = Result<O, nom::Err<VerboseError<I>>>;

pub trait SmlParse<'i>
where
    Self: Sized + std::fmt::Debug,
{
    fn parsex<E>(input: &'i [u8]) -> IResult<&[u8], Self, E> 
    where E: ParseError<&'i [u8]> + ContextError<&'i [u8]>;

    fn parse(input: &'i [u8]) -> IResult<&[u8], Self, VerboseError<&[u8]>> {
        let res = Self::parsex(input);
        if let Err(x) = &res {
            println!("{:?}", x);
            // panic!();
        }
        res
    }

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