mod decode;
mod encode;

use clap::{arg, command, value_parser, Command};
use decode::decompress_file;
use encode::compress_file;
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

fn run_compression(input_path: &Path, output_path: &Path, letter_size: u8) {
    let now = Instant::now();

    if let Err(e) = compress_file(input_path, output_path, letter_size) {
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
    let mut command = command!()
        .subcommand(
            Command::new("compress")
                .arg(
                    arg!(--input <FILE> "Input file")
                        .required(true)
                        .value_parser(value_parser!(PathBuf)),
                )
                .arg(
                    arg!(--output <FILE> "Output file")
                        .required(true)
                        .value_parser(value_parser!(PathBuf)),
                )
                .arg(
                    arg!(--size <SIZE> "Letter size")
                        .required(true)
                        .value_parser(value_parser!(u8)),
                ),
        )
        .subcommand(
            Command::new("decompress")
                .arg(
                    arg!(--input <FILE> "Input file")
                        .required(true)
                        .value_parser(value_parser!(PathBuf)),
                )
                .arg(
                    arg!(--output <FILE> "Output file")
                        .required(true)
                        .value_parser(value_parser!(PathBuf)),
                ),
        );

    let matches = command.clone().get_matches();

    match matches.subcommand() {
        Some(("compress", matches)) => {
            let input_path = matches.get_one::<PathBuf>("input").unwrap();
            let output_path = matches.get_one::<PathBuf>("output").unwrap();
            let letter_size = matches.get_one::<u8>("size").unwrap();

            run_compression(input_path, output_path, *letter_size);
        }
        Some(("decompress", matches)) => {
            let input_path = matches.get_one::<PathBuf>("input").unwrap();
            let output_path = matches.get_one::<PathBuf>("output").unwrap();

            run_decompression(Path::new(input_path), Path::new(output_path));
        }
        _ => {
            command.print_help().unwrap();
        }
    }
}
