use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;
use std::io::{BufReader, BufRead};
use std::fs::{self, File};
use clap::Parser;
use sha2::{Sha512, Digest};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Recursively check all files in any specified directory.
    #[clap(short, long)]
    recursive: bool,

    /// The files (and/or directories if in recursive mode) to check for duplicates
    paths: Vec<PathBuf>,
}

fn main() {
    let args = Args::parse();
    let mut paths: VecDeque<PathBuf> = args.paths.into_iter().collect();
    let mut buckets: BTreeMap<Vec<u8>, BTreeSet<PathBuf>> = Default::default();
    let mut total_files: usize = 0;

    while let Some(path) = paths.pop_front() {
        let meta = match fs::metadata(&path) {
            Ok(x) => x,
            Err(_) => {
                eprintln!("Failed to stat {:?}", path);
                continue;
            }
        };
        if meta.is_dir() {
            if !args.recursive {
                eprintln!("{:?} is a directory (not in recursive mode)", path);
                continue;
            }
            let content = match fs::read_dir(&path) {
                Ok(x) => x,
                Err(_) => {
                    eprintln!("Failed to read directory contents of {:?}", path);
                    continue;
                }
            };
            for sub in content {
                match sub {
                    Ok(x) => paths.push_back(x.path()),
                    Err(_) => eprintln!("Failed to read child of {:?}", path),
                }
            }
        }
        else if meta.is_symlink() {
            eprintln!("{:?} is a symlink", path);
            continue;
        }
        else if meta.is_file() {
            let mut file = match File::open(&path) {
                Ok(x) => BufReader::new(x),
                Err(_) => {
                    eprintln!("Failed to read file {:?}", path);
                    continue;
                }
            };

            let mut hasher = Sha512::new();
            loop {
                let buf = match file.fill_buf() {
                    Ok(x) => x,
                    Err(_) => {
                        eprintln!("Error while reading file {:?}", path);
                        break;
                    }
                };
                let len = buf.len();
                if len == 0 { break }
                hasher.update(buf);
                file.consume(len);
            }
            let hash = hasher.finalize().as_slice().to_owned();
            buckets.entry(hash).or_default().insert(path);
            total_files += 1;
        }
        else {
            eprintln!("{:?} is an unrecognized fs entry", path);
            continue;
        }
    }

    let mut dupe_count = 0;
    for (hash, files) in buckets.iter() {
        if files.len() == 1 { continue }
        dupe_count += files.len() - 1;

        for b in hash {
            print!("{:02x}", b);
        }
        println!(":");
        for file in files {
            println!("{:?}", file);
        }
        println!();
    }

    println!("found {} duplicates of {} total files", dupe_count, total_files);
}
