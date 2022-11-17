use std::io::Read;

/// Reads bytes from stdin and prints decoded messages / errors
///
/// Example usage:
///
/// ```
/// cat tests/libsml-testing/dzg_dwsb20_2th_3byte.bin | cargo run --all-features --example decoder
/// ```

fn main() -> Result<(), std::io::Error> {
    let stdin = std::io::stdin().lock();

    let mut decoder = sml_rs::transport::Decoder::<heapless::Vec<u8, 2048>>::new();

    for res in stdin.bytes() {
        let b = res?;

        match decoder.push_byte(b) {
            Ok(None) => {}
            Ok(Some(decoded_bytes)) => {
                println!("Ok({})", decoded_bytes.len());
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
