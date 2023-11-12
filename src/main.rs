mod decode;
mod encode;

use std::{path::Path, time::Instant};
use decode::decompress_file;
use encode::compress_file;

fn run_compression(input_path: &Path, output_path: &Path) {
    let now = Instant::now();

    if let Err(e) = compress_file(input_path, output_path) {
        eprintln!("Error failed to compress: {}", e);
    } else {
        let input_size = input_path.metadata().unwrap().len();
        let output_size = output_path.metadata().unwrap().len();
        let compression_ratio = input_size as f32 / output_size as f32;
        let duration = now.elapsed();

        println!("-------------------------------------");
        println!("Compression finished");
        println!("Input file size: {} bytes", input_size);
        println!("Output file size: {} bytes", output_size);
        println!(
            "Compression ratio: {:.3} ({:.2} %)",
            compression_ratio,
            compression_ratio * 100.0
        );
        println!("Elapsed: {:.3} (s)", duration.as_secs_f32());
    }
}

fn run_decompression(input_path: &Path, output_path: &Path) {
    let now = Instant::now();

    if let Err(e) = decompress_file(input_path, output_path) {
        eprintln!("Error failed to decompress: {}", e);
    } else {
        let input_size = input_path.metadata().unwrap().len();
        let output_size = output_path.metadata().unwrap().len();
        let duration = now.elapsed();

        println!("-------------------------------------");
        println!("Decompression finished");
        println!("Input file size: {} bytes", input_size);
        println!("Output file size: {} bytes", output_size);
        println!("Elapsed: {:.3} (s)", duration.as_secs_f32());
    }
}

fn main() {
    // run_compression(Path::new("data/hello.txt"), Path::new("out.bin"));
    // run_decompression(Path::new("out.bin"), Path::new("text.txt"));

    run_compression(Path::new("data/big_bmp.bmp"), Path::new("out.bin"));
    run_decompression(Path::new("out.bin"), Path::new("image.bmp"));
}
