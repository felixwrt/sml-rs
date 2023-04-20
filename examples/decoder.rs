//! Reads bytes from stdin and prints decoded messages / errors
//!
//! Example usage:
//!
//! ```
//! cat tests/libsml-testing/dzg_dwsb20_2th_3byte.bin | cargo run --example decoder
//! ```

use sml_rs::parser::complete::File;

fn main() -> Result<(), std::io::Error> {
    let stdin = std::io::stdin().lock();

    let mut reader = sml_rs::SmlReader::from_reader(stdin);

    while let Some(res) = reader.next::<File>() {
        match res {
            Ok(file) => println!("{:#?}", file),
            Err(e) => println!("Err({:?})", e),
        }
    }

    println!("Done.");

    Ok(())
}
