//! This module implements the SML parser

pub mod tlf;
pub mod octet_string;

use anyhow::bail;


type ResTy<'i, O> = anyhow::Result<(&'i [u8], O)>;
type ResTyComplete<'i, O> = anyhow::Result<O>;

/// SmlParse is the main trait used to parse bytes into SML data structures.
pub trait SmlParse<'i>
where
    Self: Sized,
{
    /// Tries to parse an instance of `Self` from a byte slice.
    /// 
    /// On success, returns the remaining input and the parsed instance of `Self`.
    fn parse(input: &'i [u8]) -> ResTy<Self>;
    
    /// Tries to parse an instance of `Self` from a byte slice and returns an error if there are leftover bytes.
    /// 
    /// On success, returns the parsed instance of `Self`.
    fn parse_complete(input: &'i [u8]) -> ResTyComplete<Self> {
        let (input, x) = Self::parse(input)?;
        if !input.is_empty() {
            bail!("Leftover input");
        }
        Ok(x)
    }
}

fn take_byte(input: &[u8]) -> ResTy<u8> {
    if input.is_empty() {
        bail!("Unexpected EOF");
    }
    Ok((&input[1..], input[0]))
}

// fn take<const N: usize>(input: &[u8]) -> IResult<&[u8], &[u8; N]> {
//     if input.len() < N {
//         return Err(nom::Err::Failure(error::Error::new(input, error::ErrorKind::Eof)));
//     }
//     Ok((&input[N..], input[..N].try_into().unwrap()))
// }

fn take_n(input: &[u8], n: usize) -> ResTy<&[u8]> {
    if input.len() < n {
        bail!("Unexpected EOF");
    }
    Ok((&input[n..], &input[..n]))
}