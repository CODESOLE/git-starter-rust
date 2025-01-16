use anyhow;
use anyhow::Context;
use clap::{Parser, Subcommand};
use flate2;
use hex::{self};
use sha1::{Digest, Sha1};
use std::cmp::Ordering;
use std::fs;
use std::io::prelude::*;

#[derive(Parser)]
#[command(name = "git-starter-rust")]
#[command(version = "0.1")]
#[command(about = "toy git client", long_about = None)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init,
    CatFile {
        /// pretty-print <object> content
        #[clap(short = 'p')]
        object: String,
    },
    HashObject {
        /// write object to object database
        #[clap(short = 'w')]
        object: String,
    },
    LsTree {
        /// list only filenames
        #[clap(long = "name-only")]
        no: bool,
        tree_hash: String,
    },
}

#[derive(Debug)]
struct TreeElement {
    mode: String,
    name: String,
    sha1: [u8; 20],
}

fn decode_reader(bytes: &[u8]) -> anyhow::Result<String> {
    let mut z = flate2::read::ZlibDecoder::new(bytes);
    let mut s = String::new();
    z.read_to_string(&mut s)?;
    anyhow::Ok(s)
}
fn decode_reader_raw(bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut z = flate2::read::ZlibDecoder::new(bytes);
    let mut s = Vec::new();
    z.read_to_end(&mut s)?;
    anyhow::Ok(s)
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match &cli.commands {
        Commands::Init => {
            fs::create_dir(".git").unwrap();
            fs::create_dir(".git/objects").unwrap();
            fs::create_dir(".git/refs").unwrap();
            fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
            println!("Initialized git directory")
        }
        Commands::CatFile { object } => {
            let hash = object.as_str();
            assert!(hash.len() == 40, "Hash is not 40 characters long!!!");
            let blob = std::fs::read(format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))?;
            let decoded_str = decode_reader(&blob)?;
            let content = &decoded_str[decoded_str.find('\0').unwrap() + 1..];
            print!("{content}");
        }
        Commands::HashObject { object: write } => {
            let mut file = fs::File::open(write)?;
            let mut file_content = Vec::new();
            file.read_to_end(&mut file_content)?;
            let header_plus_content: Vec<u8> = format!("blob {}\0", file_content.len())
                .as_bytes()
                .iter()
                .chain(file_content.iter())
                .cloned()
                .collect();
            let mut hash = Sha1::new();
            hash.update(&file_content);
            let digest: &[u8] = &hash.finalize();
            let hash_str = hex::encode(digest);
            println!("{}", &hash_str);
            fs::create_dir_all(format!(".git/objects/{}", &hash_str[..2])).context("creat_dir")?;
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(format!(
                    ".git/objects/{}/{}",
                    &hash_str[..2],
                    &hash_str[2..]
                ))
                .context("File Open")?;
            let mut zlib =
                flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
            zlib.write_all(&header_plus_content)?;
            let compressed = zlib.finish()?;
            f.write_all(&compressed)?;
        }
        Commands::LsTree { no, tree_hash } => {
            let hash = tree_hash.as_str();
            assert!(hash.len() == 40, "Hash is not 40 characters long!!!");
            let tree_object = std::fs::read(format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))?;
            let decoded_str = decode_reader_raw(&tree_object)?;
            //dbg!(&decoded_str);
            if &decoded_str[..4] != b"tree" {
                anyhow::bail!("fatal: not a tree object");
            }
            let first_nul_byte = decoded_str
                .iter()
                .enumerate()
                .find(|&x| *x.1 == 0x0)
                .unwrap()
                .0;
            let tree_sz = &decoded_str[5..first_nul_byte];
            if tree_sz == b"0" {
                return anyhow::Ok(());
            }
            let mut rest_raw_u8 = &decoded_str[first_nul_byte + 1..]; // <mode> <name>\0<sha1_20b>...

            let mut vec_tree_elems: Vec<TreeElement> = vec![];
            loop {
                let first_space = rest_raw_u8
                    .iter()
                    .enumerate()
                    .find(|&x| *x.1 == 0x20)
                    .unwrap()
                    .0;
                let first_nul_byte = rest_raw_u8
                    .iter()
                    .enumerate()
                    .find(|&x| *x.1 == 0x0)
                    .unwrap()
                    .0;
                let mode = String::from_utf8_lossy(&rest_raw_u8[..first_space]).to_string();
                let name = String::from_utf8_lossy(&rest_raw_u8[first_space + 1..first_nul_byte])
                    .to_string();
                let sha1 = &rest_raw_u8[first_nul_byte + 1..first_nul_byte + 21];
                vec_tree_elems.push(TreeElement {
                    mode,
                    name,
                    sha1: sha1[..].try_into().unwrap(),
                });
                if first_nul_byte + 21 >= rest_raw_u8.len() {
                    break;
                }
                rest_raw_u8 = &rest_raw_u8[first_nul_byte + 21..];
            }
            vec_tree_elems.sort_by(|a, b| a.name.cmp(&b.name));
            if *no == true {
                for elm in vec_tree_elems.iter() {
                    print!("{}\n", elm.name);
                }
                return anyhow::Ok(());
            }
            for elm in vec_tree_elems.iter() {
                if elm.mode.as_str().cmp("40000") == Ordering::Equal {
                    print!(
                        "{:<6} tree {}    {}\n",
                        elm.mode,
                        hex::encode(elm.sha1),
                        elm.name
                    );
                } else {
                    print!(
                        "{:<6} blob {}    {}\n",
                        elm.mode,
                        hex::encode(elm.sha1),
                        elm.name
                    );
                }
            }
        }
    }

    anyhow::Ok(())
}
