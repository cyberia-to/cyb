//! Extract pretokenized TinyStories data from zip
//! Usage: cargo run --bin tokenize -- ~/tiny_stories_data_pretokenized.zip [output.bin]

use std::io::{Read, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let zip_path = args.get(1).unwrap_or_else(|| {
        eprintln!("Usage: tokenize <zip_path> [output.bin]");
        std::process::exit(1);
    });
    let output = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or("tinystories_data00.bin");

    if std::path::Path::new(output).exists() {
        let size = std::fs::metadata(output).unwrap().len();
        eprintln!(
            "{} already exists ({} tokens, {:.1} MB)",
            output,
            size / 2,
            size as f64 / 1e6
        );
        return;
    }

    eprintln!("Extracting data00.bin from {}...", zip_path);
    let file = std::fs::File::open(zip_path).expect("Cannot open zip");
    let mut archive = zip::ZipArchive::new(file).expect("Invalid zip");
    let mut entry = archive
        .by_name("data00.bin")
        .expect("data00.bin not found in zip");

    let mut out = std::fs::File::create(output).expect("Cannot create output");
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = entry.read(&mut buf).expect("Read error");
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).expect("Write error");
    }

    let size = std::fs::metadata(output).unwrap().len();
    eprintln!(
        "{}: {} tokens, {:.1} MB",
        output,
        size / 2,
        size as f64 / 1e6
    );

    // Sanity check
    let data = std::fs::read(output).unwrap();
    let tokens: Vec<u16> = data
        .chunks(2)
        .take(10)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    eprintln!("First 10 tokens: {:?}", tokens);
}
