use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use data_encoding::HEXUPPER;
use ring::digest::{Context, SHA256};
use std::fs::File;
use std::io::{BufReader, Read};

struct FileMeta {
    filename: String,
    path: PathBuf,
}

struct Deduper {
    files_by_size: HashMap<u64, Vec<FileMeta>>,
}

impl Deduper {
    pub fn new() -> Self {
        Deduper {
            files_by_size: HashMap::new(),
        }
    }

    pub fn add(&mut self, path: &str) {
        self.read_dir(PathBuf::from(path));
    }

    pub fn resolve(&self) {
        for (len, files) in self.files_by_size.iter() {
            if files.len() <= 1 {
                continue;
            }

            let mut shamap = HashMap::<String, Vec<&FileMeta>>::new();

            for fm in files.iter() {
                let sha = match Deduper::sha256_digest(fm.path.clone()) {
                    Err(e) => {
                        println!("error hashing {}: {:?}", fm.filename, e);
                        continue;
                    },
                    Ok(h) => h
                };

                match shamap.entry(sha) {
                    Entry::Vacant(e) => {
                        e.insert(vec![fm]);
                    }
                    Entry::Occupied(mut e) => {
                        let v = e.get_mut();
                        v.push(fm);
                    }
                };
            }

            for (hash, conflicts) in shamap.iter() {
                if conflicts.len() <= 1 {
                    continue;
                }

                println!();
                println!("These files are identical (size {}, hash {}) with {} similes:", len, hash, files.len() - conflicts.len());
                for fm in conflicts.iter() {
                    println!("\t{}", fm.filename);
                }
                println!();
            }
        }
    }

    fn read_file(&mut self, fsentry: fs::DirEntry, meta: fs::Metadata) {
        let fm = FileMeta {
            filename: format!("{}", fsentry.path().display()),
            path: fsentry.path(),
        };

        match self.files_by_size.entry(meta.len()) {
            Entry::Vacant(e) => {
                e.insert(vec![fm]);
            }
            Entry::Occupied(mut e) => {
                let v = e.get_mut();
                v.push(fm);
            }
        };
    }

    fn read_dir(&mut self, dir: PathBuf) {
        let paths = fs::read_dir(dir).unwrap();

        for path in paths {
            let path = match path {
                Ok(p) => p,
                Err(e) => {
                    println!("Error: {:?}", e);
                    continue;
                }
            };

            let meta = match path.metadata() {
                Ok(p) => p,
                Err(e) => {
                    println!("Error: {:?}", e);
                    continue;
                }
            };

            if meta.file_type().is_symlink() {
                println!("Ignoring symlink : {}", path.path().display());
            } else if meta.file_type().is_file() {
                self.read_file(path, meta);
            } else if meta.file_type().is_dir() {
                println!("Reading directory : {}", path.path().display());
                self.read_dir(path.path());
            }
        }
    }

    pub fn sha256_digest(path: PathBuf) -> Result<String, std::io::Error> {
        let input = File::open(path)?;
        let mut reader = BufReader::new(input);
        let mut context = Context::new(&SHA256);
        let mut buffer = [0; 1024];

        loop {
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            context.update(&buffer[..count]);
        }

        Ok(HEXUPPER.encode(context.finish().as_ref()))
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        args.push(String::from("."));
    }

    let mut dedup = Deduper::new();

    for p in args.iter() {
        dedup.add(&p);
    }

    dedup.resolve();
}
