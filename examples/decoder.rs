use std::io::Read;

/// Reads bytes from stdin and prints decoded messages / errors
///
/// Example usage:
///
/// ```
/// cat tests/libsml-testing/dzg_dwsb20_2th_3byte.bin | cargo run --example decoder
/// ```

fn main() -> Result<(), std::io::Error> {
    let stdin = std::io::stdin().lock();

    let mut decoder = sml_rs::transport::Decoder::<heapless::Vec<u8, 2048>>::new();

    for res in stdin.bytes() {
        let b = res?;

        match decoder.push_byte(b) {
            Ok(None) => {}
            Ok(Some(decoded_bytes)) => {
                #[cfg(feature = "alloc")]
                {
                    println!(
                        "{:#?}",
                        sml_rs::parser::parse(decoded_bytes)
                    );
                }

                #[cfg(not(feature = "alloc"))]
                {
                    let mut parser = sml_rs::parser::streaming::ParseState::new(decoded_bytes);
                    while let Some(x) = parser.next() {
                        println!("{:#?}", x);
                    }
                }
            }
            Err(e) => {
                println!("Err({:?})", e);
            }
        }
    }

    if let Some(e) = decoder.finalize() {
        println!("Err({:?})", e);
    }

    Ok(())
}
