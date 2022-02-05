use std::fs;
use std::path::Path;
use git2::{ObjectType, Repository};

pub fn get_current_commit_id(repo: &Repository) -> Result<String, git2::Error> {
    Ok(repo.head()?.resolve()?.peel(ObjectType::Commit)?
        .into_commit().map_err(|_| git2::Error::from_str("Couldn't find commit"))?.id().to_string())
}

pub fn clone_repo(repo_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let url = format!("https://aur.archlinux.org/{}.git", repo_name);
    let path = format!("data/{}", repo_name);
    if Path::new(path.as_str()).exists() {
        let repo = Repository::open(path.clone())?;
        let current_commit = get_current_commit_id(&repo);

        fs::remove_dir_all(Path::new(path.as_str()))?;

        let cloned = Repository::clone(url.as_str(), path.as_str())?;
        let cloned_commit = get_current_commit_id(&cloned);

        return Ok(current_commit != cloned_commit);
    }

    Repository::clone(url.as_str(), path)?;

    Ok(true)
}