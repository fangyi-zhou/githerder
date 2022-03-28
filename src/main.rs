extern crate git2;
use std::env;
use std::fs;
use std::io;
use std::path::Path;

fn main() -> Result<(), io::Error> {
    let dir = match env::args().nth(1) {
        Some(dir) => dir,
        None => String::from("."),
    };
    let path = Path::new(&dir);
    if !path.is_dir() {
        panic!("Input {:?} is not a directory", path);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let inner_path = entry.path();
        if inner_path.is_dir() {
            if let Ok(repo) = git2::Repository::open(inner_path) {
                println!("Found git repo at {:?}", repo.path())
            }
        }
    }
    Ok(())
}
