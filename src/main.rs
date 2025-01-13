use anyhow;
use anyhow::Context;
use flate2;
use hex;
use sha1::{Digest, Sha1};
use std::env;
use std::fs;
use std::io::prelude::*;

fn decode_reader(bytes: Vec<u8>) -> anyhow::Result<String> {
    let mut z = flate2::read::ZlibDecoder::new(&bytes[..]);
    let mut s = String::new();
    z.read_to_string(&mut s)?;
    anyhow::Ok(s)
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args[1] == "init" {
        fs::create_dir(".git").unwrap();
        fs::create_dir(".git/objects").unwrap();
        fs::create_dir(".git/refs").unwrap();
        fs::write(".git/HEAD", "ref: refs/heads/main\n").unwrap();
        println!("Initialized git directory")
    } else if args[1] == "cat-file" && args[2] == "-p" {
        let hash = args[3].clone();
        assert!(hash.len() == 40, "Hash is not 40 characters long!!!");
        let blob = std::fs::read(format!(".git/objects/{}/{}", &hash[..2], &hash[2..]))?;
        let decoded_str = decode_reader(blob)?;
        let content = &decoded_str[decoded_str.find('\0').unwrap() + 1..];
        print!("{content}");
    } else if args[1] == "hash-object" && args[2] == "-w" {
        let mut file = fs::File::open(args[3].clone())?;
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
        let mut zlib = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        zlib.write_all(&header_plus_content)?;
        let compressed = zlib.finish()?;
        f.write_all(&compressed)?;
    } else {
        println!("unknown command: {}", args[1])
    }

    anyhow::Ok(())
}
