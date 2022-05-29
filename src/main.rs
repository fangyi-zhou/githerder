extern crate async_executor;
extern crate async_process;
extern crate futures;
extern crate git2;
use async_executor::{Executor, Task};
use async_process::{Command, Output, Stdio};
use futures::executor::block_on;
use futures::future::join_all;
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

enum Action<'a> {
    Pull(&'a Path),
    Fetch(&'a Path),
}

impl<'a> Action<'a> {
    fn workdir(&self) -> &Path {
        match self {
            Action::Pull(path) => path,
            Action::Fetch(path) => path,
        }
    }

    fn verb(&self) -> &str {
        match self {
            Action::Pull(_) => "pull",
            Action::Fetch(_) => "fetch",
        }
    }
}

fn get_action(repo: &Repository) -> Result<Option<Action>, Box<dyn Error>> {
    if let RepositoryState::Clean = repo.state() {
        if let Ok(statuses) = repo.statuses(None) {
            if statuses.iter().all(|status_entry| {
                status_entry.status().is_empty() || status_entry.status().is_ignored()
            }) {
                // println!("Clean git repo at {:?}", repo.path());
                if let Some(path) = repo.workdir() {
                    return Ok(Some(Action::Pull(path)));
                }
            }
        }
    }
    Ok(None)
}

fn execute_action(exe: &Executor, action: &Action) -> Task<Result<Output, io::Error>> {
    let workdir = action.workdir().to_owned();
    let verb = action.verb().to_owned();
    exe.spawn(async move {
        println!("Pulling {}", workdir.display());
        Command::new("git")
            .arg("-C")
            .arg(workdir)
            .arg(verb)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .await
    })
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
    let workdirs: Vec<Action> = repos
        .iter()
        .filter_map(|repo| get_action(repo).ok().flatten())
        .collect();
    let exe = Executor::new();
    let tasks = workdirs.iter().map(|action| execute_action(&exe, action));
    block_on(exe.run(join_all(tasks)));
    Ok(())
}
