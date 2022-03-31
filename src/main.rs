extern crate git2;
use git2::{Repository, RepositoryState};
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;

fn discover_git_repos(dir: &Path) -> Result<Vec<Repository>, io::Error> {
    let mut repos = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let inner_path = entry.path();
        if inner_path.is_dir() {
            if let Ok(repo) = Repository::open(inner_path) {
                //println!("Found git repo at {:?}", repo.path())
                repos.push(repo)
            }
        }
    }
    Ok(repos)
}

fn get_workdir_for_clean_repos(repo: &Repository) -> Result<Option<&Path>, Box<dyn Error>> {
    if let RepositoryState::Clean = repo.state() {
        if let Ok(statuses) = repo.statuses(None) {
            if statuses.iter().all(|status_entry| {
                status_entry.status().is_empty() || status_entry.status().is_ignored()
            }) {
                // println!("Clean git repo at {:?}", repo.path());
                Ok(repo.workdir())
            } else {
                println!("Skipping unclean git repo at {:?}", repo.path());
                Ok(None)
            }
        } else {
            Ok(None)
        }
    } else {
        // Ignore repos that are not clean
        println!("Skippuing unclean git repo at {:?}", repo.path());
        Ok(None)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let dir = match env::args().nth(1) {
        Some(dir) => dir,
        None => String::from("."),
    };
    let path = Path::new(&dir);
    if !path.is_dir() {
        panic!("Input {:?} is not a directory", path);
    }
    let repos = discover_git_repos(path)?;
    let workdirs: Vec<&Path> = repos
        .iter()
        .filter_map(|repo| get_workdir_for_clean_repos(repo).ok().flatten())
        .collect();
    for workdir in workdirs.iter() {
        println!("Workdir to pull: {}", workdir.display());
    }
    Ok(())
}
