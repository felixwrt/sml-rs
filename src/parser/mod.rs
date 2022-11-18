pub mod tlf;

use anyhow::bail;


type ResTy<'i, O> = anyhow::Result<(&'i [u8], O)>;
type ResTyComplete<'i, O> = anyhow::Result<O>;

pub trait SmlParse<'i>
where
    Self: Sized,
{
    fn parse(input: &[u8]) -> ResTy<Self>;
    
    fn parse_complete(input: &[u8]) -> ResTyComplete<Self> {
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