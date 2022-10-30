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

fn process_repository(repo: &Repository) -> Result<(), Box<dyn Error>> {
    let path = repo.path();
    println!("Processing {:?}", path);
    if !repo.head_detached()? {
        let head = repo.head()?;
        let head_name = head.name().unwrap();
        // println!("HEAD is {:?}", head_name);
        if head.is_branch() {
            // Fetch the remote branch
            let remote_name = repo.branch_upstream_remote(head_name)?;
            let branch_name = repo.branch_upstream_name(head_name)?;
            // println!("remote is {:?}", remote?.as_str());
            // println!("branch is {:?}", branch?.as_str());
            let mut remote = repo.find_remote(remote_name.as_str().unwrap()).unwrap();
            println!("remote is found {:?}", remote.name());
            remote.fetch(&[branch_name.as_str().unwrap()], None, None);
            println!("Fetched");
        } else {
            println!("{:?}: no remote tracking branch, skipping", path);
        }
        Ok(())
    } else {
        // Detached head, do nothing
        println!("{:?}: detached HEAD, skipping", path);
        Ok(())
    }
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

    fn additional_options(&self) -> Vec<&'static str> {
        match self {
            Action::Pull(_) => vec!["--ff-only"],
            Action::Fetch(_) => vec![],
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
    if let Ok(remotes) = repo.remotes() {
        if !remotes.is_empty() {
            if let Some(path) = repo.workdir() {
                return Ok(Some(Action::Fetch(path)));
            }
        }
    }
    Ok(None)
}

fn execute_action(exe: &Executor, action: &Action) -> Task<Result<Output, io::Error>> {
    let workdir = action.workdir().to_owned();
    let verb = action.verb().to_owned();
    let opts = action.additional_options().to_owned();
    exe.spawn(async move {
        println!("{}ing {}", verb, workdir.display());
        Command::new("git")
            .arg("-C")
            .arg(workdir)
            .arg(verb)
            .args(opts)
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
    // let workdirs: Vec<Action> = repos
    //     .iter()
    //     .filter_map(|repo| get_action(repo).ok().flatten())
    //     .collect();
    let exe = Executor::new();
    repos.iter().try_for_each(process_repository)?;
    // block_on(exe.run(join_all(tasks)));
    Ok(())
}
