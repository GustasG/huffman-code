use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::Path;

use bitstream_io::{BigEndian, BitQueue, BitWrite, BitWriter};

#[derive(Debug, Eq, PartialEq)]
enum NodePayload {
    Leaf(u8),
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

fn count_frequency<P: AsRef<Path>>(input_path: P) -> Result<Vec<Node>, std::io::Error> {
    let file = File::open(input_path)?;
    let reader = BufReader::with_capacity(32 * 1024, file);

    let mut nodes = (0..=255)
        .map(|i| Node {
            freq: 0,
            payload: NodePayload::Leaf(i),
        })
        .collect::<Vec<Node>>();

    for byte in reader.bytes() {
        let byte = byte?;
        nodes[byte as usize].freq += 1;
    }

    nodes.retain(|node| node.freq != 0);

    Ok(nodes)
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

fn create_table(tree: &Node) -> HashMap<u8, (u32, u32)> {
    fn walk(root: &Node, current_path: &mut Vec<bool>, table: &mut HashMap<u8, (u32, u32)>) {
        match &root.payload {
            NodePayload::Leaf(byte) => {
                let mut queue = BitQueue::<BigEndian, u32>::new();

                for bit in current_path {
                    match *bit {
                        false => queue.push(1, 0),
                        true => queue.push(1, 1),
                    }
                }

                table.insert(*byte, (queue.len(), queue.value()));
            }
            NodePayload::Joint(left, right) => {
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

fn write_header<W: BitWrite>(writer: &mut W, tree: &Node) -> Result<(), std::io::Error> {
    match &tree.payload {
        NodePayload::Leaf(byte) => {
            writer.write_bit(false)?;
            writer.write(8, *byte)?;
        }
        NodePayload::Joint(left, right) => {
            writer.write_bit(true)?;
            write_header(writer, &left)?;
            write_header(writer, &right)?;
        }
    }

    Ok(())
}

fn compress<R: Read, W: BitWrite>(
    reader: &mut R,
    writer: &mut W,
    nodes: Vec<Node>,
    file_size: u64,
) -> Result<(), std::io::Error> {
    let tree = match create_tree(nodes) {
        Some(root) => root,
        _ => return Ok(()),
    };

    let table = create_table(&tree);

    write_header(writer, &tree)?;
    writer.write(64, file_size)?;

    for byte in reader.bytes() {
        let byte = byte?;
        let (length, value) = table[&byte];
        writer.write(length, value)?;
    }

    writer.byte_align()?;

    Ok(())
}

pub fn compress_file<P: AsRef<Path>>(input_path: P, output_path: P) -> Result<(), std::io::Error> {
    let fin = File::open(&input_path)?;
    let mut reader = BufReader::with_capacity(32 * 1024, fin);
    let nodes = count_frequency(&input_path)?;
    let file_size = std::fs::metadata(&input_path)?.len();

    let fout = File::create(&output_path)?;
    let writer = BufWriter::with_capacity(32 * 1024, fout);
    let mut writer = BitWriter::endian(writer, BigEndian);

    compress(&mut reader, &mut writer, nodes, file_size)
}
