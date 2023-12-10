use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, ErrorKind, Read};
use std::path::Path;

use bitstream_io::{BigEndian, BitQueue, BitRead, BitReader, BitWrite, BitWriter, Endianness};

#[derive(Debug)]
enum Node {
    Leaf(u32),
    Joint(Box<Node>, Box<Node>),
}

fn create_table(tree: &Node) -> HashMap<(u32, u32), u32> {
    fn walk(root: &Node, table: &mut HashMap<(u32, u32), u32>, current_path: &mut Vec<bool>) {
        match &root {
            Node::Leaf(code) => {
                let mut queue = BitQueue::<BigEndian, u32>::new();

                for bit in current_path {
                    match *bit {
                        false => queue.push(1, 0),
                        true => queue.push(1, 1),
                    }
                }

                let len = queue.len();
                table.insert((len, queue.value()), *code);
            }
            Node::Joint(left, right) => {
                current_path.push(false);
                walk(left, table, current_path);
                current_path.pop();

                current_path.push(true);
                walk(right, table, current_path);
                current_path.pop();
            }
        }
    }

    let mut path = Vec::new();
    let mut table = HashMap::new();
    walk(tree, &mut table, &mut path);

    table
}

fn read_header<R: BitRead>(reader: &mut R, letter_size: u8) -> Result<Node, std::io::Error> {
    match reader.read_bit()? {
        false => {
            let code = reader.read::<u32>(letter_size as u32)?;
            Ok(Node::Leaf(code))
        }
        true => {
            let left = read_header(reader, letter_size)?;
            let right = read_header(reader, letter_size)?;

            Ok(Node::Joint(Box::new(left), Box::new(right)))
        }
    }
}

fn decompress<R: Read, W: BitWrite, E: Endianness>(
    reader: &mut BitReader<R, E>,
    writer: &mut W,
) -> Result<(), std::io::Error> {
    let letter_size = reader.read::<u8>(8)?;
    let file_size = reader.read::<u64>(64)? * 8;
    let target_size = letter_size as u64 * (file_size / letter_size as u64);
    let remaining_size = file_size - target_size;

    let mut written = 0;

    let table = match read_header(reader, letter_size) {
        Err(e) => match e.kind() {
            ErrorKind::UnexpectedEof => return Ok(()),
            _ => return Err(e),
        },
        Ok(tree) => create_table(&tree),
    };

    let mut buffer = 0;
    let mut iteration = 0;

    while written < target_size {
        let bit = reader.read_bit()?;
        buffer = (buffer << 1) | bit as u32;
        iteration += 1;

        if let Some(value) = table.get(&(iteration, buffer)) {
            writer.write(letter_size as u32, *value)?;
            written += letter_size as u64;
            buffer = 0;
            iteration = 0;
        }
    }

    if remaining_size != 0 {
        let value = reader.read::<u32>(remaining_size as u32)?;
        writer.write(remaining_size as u32, value)?;
    }

    Ok(())
}

pub fn decompress_file<P: AsRef<Path>>(
    input_path: P,
    output_path: P,
) -> Result<(), std::io::Error> {
    let fin = File::open(input_path)?;
    let reader = BufReader::with_capacity(32 * 1024, fin);
    let mut reader = BitReader::endian(reader, BigEndian);

    let fout = File::create(&output_path)?;
    let writer = BufWriter::with_capacity(32 * 1024, fout);
    let mut writer = BitWriter::endian(writer, BigEndian);

    decompress(&mut reader, &mut writer)
}
