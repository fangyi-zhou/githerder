extern crate easy_parallel;
extern crate git2;
use easy_parallel::Parallel;
use git2::build::CheckoutBuilder;
use git2::{Cred, FetchOptions, RemoteCallbacks, Repository};
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

fn process_repository(repo: &Repository) -> Result<(), Box<dyn Error + Send + Sync>> {
    // See the git pull example
    // https://github.com/rust-lang/git2-rs/blob/master/examples/pull.rs
    let path = repo.path();
    let path_str = path.to_string_lossy();
    if let Ok(mut head) = repo.head() {
        let head_name = head.name().unwrap();
        // println!("HEAD is {:?}", head_name);
        if head.is_branch() {
            // HEAD is pointing to a branch

            if let (Ok(remote_name_buf), Ok(remote_ref_buf)) = (
                repo.branch_upstream_remote(head_name),
                repo.branch_upstream_name(head_name),
            ) {
                let remote_name = remote_name_buf.as_str().unwrap();
                let remote_ref = remote_ref_buf.as_str().unwrap();

                let mut remote = repo.find_remote(remote_name).unwrap();

                // Set authentication callback
                // https://docs.rs/git2/latest/git2/struct.RemoteCallbacks.html
                let mut callbacks = RemoteCallbacks::new();
                callbacks.credentials(|_url, username_from_url, _allowed_types| {
                    Cred::ssh_key(
                        username_from_url.unwrap(),
                        None,
                        std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
                        None,
                    )
                });
                let mut fetch_options = FetchOptions::new();
                fetch_options.remote_callbacks(callbacks);

                println!("{}: fetching", path_str);
                remote.fetch(&[remote_ref], Some(&mut fetch_options), None)?;

                if let Ok(fetched) = repo.find_reference(remote_ref) {
                    let commit = repo.reference_to_annotated_commit(&fetched)?;

                    // Perform a merge analysis, and only fast forward
                    let (analysis_result, _) = repo.merge_analysis(&[&commit])?;
                    if analysis_result.is_fast_forward() {
                        println!("{}: fast forwarding", path_str);
                        let reflog = format!(
                            "Fast-Forward by githerder: Setting {} to id: {}",
                            head_name,
                            commit.id()
                        );
                        head.set_target(commit.id(), &reflog)?;
                        repo.set_head(head.name().unwrap())?;

                        let mut checkout_builder = CheckoutBuilder::new();
                        checkout_builder.force();
                        repo.checkout_head(Some(&mut checkout_builder))?;
                    } else if analysis_result.is_up_to_date() {
                        println!("{}: already up to date", path_str);
                    } else if analysis_result.is_normal() {
                        println!("{}: ATTENTION: merging is necessary", path_str);
                    }
                }
            } else {
                println!("{}: no remote tracking branch, skipping", path_str);
            }
        } else {
            println!("{}: HEAD not point to a branch, skipping", path_str);
        }
    } else {
        println!("{}: cannot find HEAD, skipping", path_str);
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
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
    let tasks = Parallel::new()
        .each(repos.into_iter(), |repo| process_repository(&repo))
        .run();
    tasks.into_iter().reduce(Result::or).unwrap()
}
