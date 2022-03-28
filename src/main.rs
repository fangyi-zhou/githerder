extern crate git2;
use std::env;
use std::fs;
use std::io;
use std::path::Path;

fn discover_git_repos(dir: &Path) -> Result<Vec<git2::Repository>, io::Error> {
    let mut repos = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let inner_path = entry.path();
        if inner_path.is_dir() {
            if let Ok(repo) = git2::Repository::open(inner_path) {
                //println!("Found git repo at {:?}", repo.path())
                repos.push(repo)
            }
        }
    }
    Ok(repos)
}

fn process_repo(repo: git2::Repository) -> Result<(), io::Error> {
    if let git2::RepositoryState::Clean = repo.state() {
        if let Ok(statuses) = repo.statuses(None) {
            if statuses.iter().all(|status_entry| {
                status_entry.status().is_empty() || status_entry.status().is_ignored()
            }) {
                println!("Clean git repo at {:?}", repo.path());
            } else {
                println!("Unclean git repo at {:?}", repo.path());
            }
        }
        Ok(())
    } else {
        // Ignore repos that are not clean
        println!("Unclean git repo at {:?}", repo.path());
        Ok(())
    }
}

fn main() -> Result<(), io::Error> {
    let dir = match env::args().nth(1) {
        Some(dir) => dir,
        None => String::from("."),
    };
    let path = Path::new(&dir);
    if !path.is_dir() {
        panic!("Input {:?} is not a directory", path);
    }
    let repos = discover_git_repos(path)?;
    for repo in repos {
        process_repo(repo)?;
    }
    Ok(())
}
