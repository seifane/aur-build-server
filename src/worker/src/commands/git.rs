use std::fs;
use std::path::Path;
use git2::{ObjectType, Repository};
use crate::errors::PackageBuildError;

fn get_current_commit_id(repo: &Repository) -> Result<String, git2::Error> {
    Ok(repo.head()?.resolve()?.peel(ObjectType::Commit)?
        .into_commit().map_err(|_| git2::Error::from_str("Couldn't find commit"))?.id().to_string())
}

pub fn clone_repo(repo_name: &String) -> Result<String, PackageBuildError> {
    let url = format!("https://aur.archlinux.org/{}.git", repo_name);
    let path = format!("data/{}", repo_name);
    if Path::new(path.as_str()).exists() {
        fs::remove_dir_all(Path::new(path.as_str())).map_err(
            |_e| PackageBuildError::new(String::from("Failed to clean repo. Check permissions"), None)
        )?;
    }
    let cloned = Repository::clone(url.as_str(), path.as_str())
        .map_err(
            |_e| PackageBuildError::new(String::from("Failed to clone"), None)
        )?;
    let cloned_commit = get_current_commit_id(&cloned);

    if cloned_commit.is_err() {
        return Err(PackageBuildError::new(String::from("Failed to get commit"),None));
    }

    return Ok(cloned_commit.unwrap());
}
