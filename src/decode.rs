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
            return Ok(Node::Leaf(byte));
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
    let tree = match read_header(reader) {
        Err(e) => match e.kind() {
            ErrorKind::UnexpectedEof => return Ok(()),
            _ => return Err(e),
        },
        Ok(tree) => tree,
    };

    let table = create_table(&tree);
    let mut buffer = Vec::new();

    loop {
        match reader.read_bit() {
            Err(e) => match e.kind() {
                std::io::ErrorKind::UnexpectedEof => break,
                _ => return Err(e),
            },
            Ok(bit) => {
                buffer.push(bit);

                if let Some(value) = table.get(&buffer) {
                    writer.write(8, *value)?;
                    buffer.clear();
                }
            }
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

    match decompress(&mut reader, &mut writer) {
        Ok(()) => Ok(()),
        Err(e) => {
            std::fs::remove_file(output_path).ok();
            Err(e)
        }
    }
}
