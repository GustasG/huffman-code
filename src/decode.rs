use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, ErrorKind};
use std::path::Path;

use bitstream_io::{BigEndian, BitRead, BitReader, BitWrite, BitWriter};

enum Node {
    Leaf(u8),
    Joint(Box<Node>, Box<Node>),
}

fn create_table(tree: &Node) -> HashMap<Vec<bool>, u8> {
    fn walk(root: &Node, current_path: &mut Vec<bool>, table: &mut HashMap<Vec<bool>, u8>) {
        match &root {
            Node::Leaf(byte) => {
                table.insert(current_path.clone(), *byte);
            }
            Node::Joint(left, right) => {
                current_path.push(false);
                walk(left, current_path, table);
                current_path.pop();

                current_path.push(true);
                walk(right, current_path, table);
                current_path.pop();
            }
        }
    }

    let mut path = Vec::new();
    let mut table = HashMap::new();
    walk(&tree, &mut path, &mut table);

    table
}

fn read_header<R: BitRead>(reader: &mut R) -> Result<Node, std::io::Error> {
    match reader.read_bit()? {
        false => {
            let byte = reader.read::<u8>(8)?;
            Ok(Node::Leaf(byte))
        }
        true => {
            let left = read_header(reader)?;
            let right = read_header(reader)?;

            Ok(Node::Joint(Box::new(left), Box::new(right)))
        }
    }
}

fn decompress<R: BitRead, W: BitWrite>(
    reader: &mut R,
    writer: &mut W,
) -> Result<(), std::io::Error> {
    let table = match read_header(reader) {
        Err(e) => match e.kind() {
            ErrorKind::UnexpectedEof => return Ok(()),
            _ => return Err(e),
        },
        Ok(tree) => create_table(&tree),
    };

    let file_size = reader.read::<u64>(64)?;
    let mut buffer = Vec::new();
    let mut total_written = 0;

    while total_written < file_size {
        let bit = reader.read_bit()?;
        buffer.push(bit);

        if let Some(value) = table.get(&buffer) {
            writer.write(8, *value)?;
            total_written += 1;
            buffer.clear();
        }
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
