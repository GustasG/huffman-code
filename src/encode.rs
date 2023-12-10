use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io::{BufReader, BufWriter, ErrorKind, Read, Seek, SeekFrom};
use std::path::Path;

use bitstream_io::{BigEndian, BitQueue, BitRead, BitReader, BitWrite, BitWriter, Endianness};

#[derive(Debug, Eq, PartialEq)]
enum NodePayload {
    Leaf(u32),
    Joint(Box<Node>, Box<Node>),
}

#[derive(Debug, Eq, PartialEq)]
struct Node {
    freq: usize,
    payload: NodePayload,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.freq.cmp(&self.freq)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn count_frequency<P: AsRef<Path>>(
    input_path: P,
    letter_size: u8,
) -> Result<Vec<Node>, std::io::Error> {
    let file = File::open(input_path)?;
    let reader = BufReader::with_capacity(32 * 1024, file);
    let mut reader = BitReader::endian(reader, BigEndian);
    let mut nodes = HashMap::new();

    loop {
        match reader.read::<u32>(letter_size as u32) {
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => break,
                _ => return Err(e),
            },
            Ok(code) => {
                let node = nodes.entry(code).or_insert(Node {
                    freq: 0,
                    payload: NodePayload::Leaf(code),
                });

                node.freq += 1;
            }
        }
    }

    Ok(nodes.into_values().collect())
}

fn create_tree(nodes: Vec<Node>) -> Option<Node> {
    let mut nodes = BinaryHeap::from(nodes);

    while nodes.len() > 1 {
        let left = nodes.pop().unwrap();
        let right = nodes.pop().unwrap();

        nodes.push(Node {
            freq: left.freq + right.freq,
            payload: NodePayload::Joint(Box::new(left), Box::new(right)),
        });
    }

    nodes.pop()
}

fn create_table(tree: &Node) -> HashMap<u32, (u32, u32)> {
    fn walk(root: &Node, table: &mut HashMap<u32, (u32, u32)>, current_path: &mut Vec<bool>) {
        match &root.payload {
            NodePayload::Leaf(code) => {
                let mut queue = BitQueue::<BigEndian, u32>::new();

                for bit in current_path {
                    match *bit {
                        false => queue.push(1, 0),
                        true => queue.push(1, 1),
                    }
                }

                table.insert(*code, (queue.len(), queue.value()));
            }
            NodePayload::Joint(left, right) => {
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

fn write_header<W: BitWrite>(
    writer: &mut W,
    tree: &Node,
    letter_size: u8,
) -> Result<(), std::io::Error> {
    match &tree.payload {
        NodePayload::Leaf(code) => {
            writer.write_bit(false)?;
            writer.write(letter_size as u32, *code)?;
        }
        NodePayload::Joint(left, right) => {
            writer.write_bit(true)?;
            write_header(writer, left, letter_size)?;
            write_header(writer, right, letter_size)?;
        }
    }

    Ok(())
}

fn compress<R: Read + Seek, W: BitWrite, E: Endianness>(
    mut reader: BitReader<R, E>,
    mut writer: W,
    nodes: Vec<Node>,
    file_size: u64,
    letter_size: u8,
) -> Result<(), std::io::Error> {
    if !(2..=16).contains(&letter_size) {
        return Err(std::io::Error::new(
            ErrorKind::InvalidInput,
            "letter size must be between 2 and 16",
        ));
    }

    writer.write(8, letter_size)?;
    writer.write(64, file_size)?;

    let tree = match create_tree(nodes) {
        Some(root) => root,
        _ => return Ok(()),
    };

    let table = create_table(&tree);
    let mut written = 0;

    write_header(&mut writer, &tree, letter_size)?;

    loop {
        match reader.read::<u32>(letter_size as u32) {
            Err(e) => match e.kind() {
                ErrorKind::UnexpectedEof => break,
                _ => return Err(e),
            },
            Ok(code) => {
                let (length, value) = table[&code];
                writer.write(length, value)?;
                written += letter_size as usize;
            }
        }
    }

    let remaining = file_size * 8 - written as u64;

    if remaining != 0 {
        reader.seek_bits(SeekFrom::Current(-(remaining as i64)))?;
        let value = reader.read::<u32>(remaining as u32)?;

        writer.write(remaining as u32, value)?;
    }

    writer.byte_align()?;

    Ok(())
}

pub fn compress_file<P: AsRef<Path>>(
    input_path: P,
    output_path: P,
    letter_size: u8,
) -> Result<(), std::io::Error> {
    let fin = File::open(&input_path)?;
    let reader = BufReader::with_capacity(32 * 1024, fin);
    let reader = BitReader::endian(reader, BigEndian);

    let fout = File::create(&output_path)?;
    let writer = BufWriter::with_capacity(32 * 1024, fout);
    let writer = BitWriter::endian(writer, BigEndian);

    let nodes = count_frequency(&input_path, letter_size)?;
    let file_size = std::fs::metadata(input_path)?.len();
    compress(reader, writer, nodes, file_size, letter_size)
}
