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

    let mut decoder = sml_rs::transport::Decoder::<heapless::Vec<u8, 1024>>::new();

    for res in stdin.bytes() {
        let b = res?;

        match decoder.push_byte(b) {
            Ok(None) => {}
            Ok(Some(decoded_bytes)) => {
                println!("Decoded {} bytes. Parsing SML:", decoded_bytes.len());
                let parser = sml_rs::parser::streaming::Parser::new(decoded_bytes);
                for item in parser {
                    println!("{:#?}", item);
                }
                println!("\n\n")
            }
            Err(e) => {
                println!("Error decoding transmission: {:?}", e);
            }
        }
    }

    if let Some(e) = decoder.finalize() {
        println!("Error decoding transmission: {:?}", e);
    }

    Ok(())
}
