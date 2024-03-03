use std::error::Error;
use std::fs;
use std::path::Path;
use git2::{Diff, ObjectType, Repository};
use log::{error, info};
use reqwest::Client;
use sha2::{Digest, Sha512};
use common::models::{Package, PackagePatch};
use crate::errors::PackageBuildError;

fn get_current_commit_id(repo: &Repository) -> Result<String, git2::Error> {
    Ok(repo.head()?.resolve()?.peel(ObjectType::Commit)?
        .into_commit().map_err(|_| git2::Error::from_str("Couldn't find commit"))?.id().to_string())
}

pub async fn fetch_patch(patch: &PackagePatch) -> Result<String, Box<dyn Error + Send + Sync>>
{
    let content = Client::new().get(&patch.url)
        .send()
        .await?
        .text()
        .await?;

    if let Some(expected_hash) = patch.sha512.as_ref() {
        let mut hasher = Sha512::new();
        hasher.update(content.as_bytes());
        let actual_hash = hasher.finalize();
        let actual_hash = base16ct::lower::encode_string(&actual_hash);
        if &actual_hash != expected_hash {
            error!("Patch {} hash not matching expected '{}' got '{}'", patch.url, expected_hash, actual_hash);
            return Err(PackageBuildError::new("Patch hash is not matching".to_string(), None).into());
        }
    }

    Ok(content)
}

pub async fn apply_patches(package: &Package, repository: Repository) -> Result<(), Box<dyn Error + Send + Sync>>
{
    for patch in package.patches.iter() {
        info!("Applying patch {} on {} ...", patch.url, package.name);
        let patch_content = fetch_patch(patch).await?;
        info!("Patch content : '{}'", patch_content);
        apply_patch(&repository, &patch_content)?;
        info!("Patch is applied !");
    }
    Ok(())
}

pub fn apply_patch(repository: &Repository, content: &String) -> Result<(), Box<dyn Error + Send + Sync>>
{
    let diff = Diff::from_buffer(content.as_bytes())?;
    let mut apply_opts = git2::ApplyOptions::new();
    apply_opts.check(false);
    repository.apply(
        &diff,
        git2::ApplyLocation::WorkDir,
        Some(&mut apply_opts),
    )?;
    Ok(())
}

pub fn clone_repo(repo_name: &String) -> Result<Repository, PackageBuildError> {
    let url = format!("https://aur.archlinux.org/{}.git", repo_name);
    let path = format!("data/{}", repo_name);
    if Path::new(path.as_str()).exists() {
        fs::remove_dir_all(Path::new(path.as_str())).map_err(
            |_e| PackageBuildError::new(String::from("Failed to clean repo. Check permissions"), None)
        )?;
    }
    let repository = Repository::clone(url.as_str(), path.as_str())
        .map_err(
            |_e| PackageBuildError::new(String::from("Failed to clone"), None)
        )?;
    let cloned_commit = get_current_commit_id(&repository);

    if cloned_commit.is_err() {
        return Err(PackageBuildError::new(String::from("Failed to get commit"),None));
    }
    return Ok(repository);
}

#[cfg(test)]
mod tests {
    use tokio::fs::read_to_string;
    use common::models::{Package, PackagePatch};
    use crate::commands::git::{apply_patches, clone_repo};

    #[tokio::test]
    async fn clone_and_patch() {
        let repo = clone_repo(&"google-chrome".to_string()).unwrap();

        let package = Package {
            name: "google-chrome".to_string(),
            run_before: None,
            patches: vec![
                PackagePatch {
                    url: "https://gist.githubusercontent.com/seifane/9c6343a086392db6bc40f98138542e5e/raw/22a7ac55dcf378bde3bd63f9a68eb1799253ba67/gistfile1.txt".to_string(),
                    sha512: Some("80a819fa1caa77660c1e5970cc6783a217364551fc331cce72e662d9d2c2e6b28712e278b3a9089822255f3506d07b9a05be28986d3a04bf72713ff0689eeb0a".to_string()),
                }
            ],
            last_built_version: None,
        };

        apply_patches(&package, repo).await.unwrap();
        let contents = read_to_string("data/google-chrome/PKGBUILD").await.unwrap();
        assert_eq!(true, contents.contains("The popular web browser by Google (Stable Channel) test"));
    }
}