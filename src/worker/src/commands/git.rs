use anyhow::{bail, Context, Result};
use std::fs;
use std::path::PathBuf;
use git2::{Diff, ObjectType, Repository};
use log::{debug, info};
use reqwest::Client;
use sha2::{Digest, Sha512};
use common::models::{PackageJob, PackagePatchDefinition};

async fn fetch_patch(patch: &PackagePatchDefinition) -> Result<String>
{
    let content = Client::new().get(&patch.url)
        .send()
        .await.with_context(|| format!("Failed to retrieve patch {}", patch.url))?
        .text()
        .await.with_context(|| format!("Failed to parse patch request content {}", patch.url))?;

    if let Some(expected_hash) = patch.sha512.as_ref() {
        let mut hasher = Sha512::new();
        hasher.update(content.as_bytes());
        let actual_hash = hasher.finalize();
        let actual_hash = base16ct::lower::encode_string(&actual_hash);
        if &actual_hash != expected_hash {
            bail!("Patch {} hash not matching expected '{}' got '{}'", patch.url, expected_hash, actual_hash);
        }
    }

    Ok(content)
}

fn apply_patch(repository: &Repository, content: &String) -> Result<()>
{
    let diff = Diff::from_buffer(content.as_bytes()).with_context(|| "Failed to create diff from buffer")?;
    let mut apply_opts = git2::ApplyOptions::new();
    apply_opts.check(false);

    repository.apply(
        &diff,
        git2::ApplyLocation::WorkDir,
        Some(&mut apply_opts),
    ).with_context(|| "Failed to apply patch")?;
    Ok(())
}

pub async fn apply_patches(package: &PackageJob, repository: Repository) -> Result<()>
{
    for patch in package.definition.patches.iter() {
        info!("Applying patch {} on {} ...", patch.url, package.definition.name);
        let patch_content = fetch_patch(patch).await?;
        info!("Patch content : '{}'", patch_content);
        apply_patch(&repository, &patch_content)?;
        info!("Patch is applied !");
    }

    Ok(())
}

fn get_current_commit_id(repo: &Repository) -> Result<String> {
    Ok(repo.head()?.resolve()?.peel(ObjectType::Commit)?
        .into_commit().map_err(|_| git2::Error::from_str("Couldn't find commit"))?.id().to_string())
}

pub fn clone_repo(data_path: &PathBuf, repo_name: &String) -> Result<Repository> {
    let path = data_path.join(repo_name);
    let url = format!("https://aur.archlinux.org/{}.git", repo_name);

    if path.exists() {
        fs::remove_dir_all(&path).with_context(|| "Failed to clean repository. Check permissions")?;
    }

    let repository = Repository::clone(url.as_str(), path)
        .with_context(|| format!("Failed to clone for url {}", url))?;

    let cloned_commit = get_current_commit_id(&repository)
        .with_context(|| format!("Failed to get commit for url {}", url))?;
    debug!("Cloned commit for {}: {}", url, cloned_commit);

    Ok(repository)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use tokio::fs::{read_to_string, remove_dir_all};
    use common::models::{PackageDefinition, PackageJob, PackagePatchDefinition};
    use crate::commands::git::{apply_patches, clone_repo};

    #[tokio::test]
    async fn clone_and_patch() {
        let repo = clone_repo(&PathBuf::from("data"), &"google-chrome".to_string()).unwrap();

        let package = PackageJob {
            definition: PackageDefinition {
                package_id: 1,
                name: "google-chrome".to_string(),
                run_before: None,
                patches: vec![
                    PackagePatchDefinition {
                        url: "https://gist.githubusercontent.com/seifane/d1b04045a02452ada1fe894d18e2c2aa/raw/bc01f21fc579164d69dff0191685647d81d4b27e/gistfile1.txt".to_string(),
                        sha512: Some("cb8e7696fb1ff4fd6ed0d5200b2665c470aaf1ed2f67e0b73762b242327bdde34512afcf728151656d3442579e655465fc6d6fb89ff4412fad16357eb9c7632a".to_string()),
                    }
                ],
            },
            last_built_version: None,
        };

        apply_patches(&package, repo).await.unwrap();
        let contents = read_to_string("data/google-chrome/PKGBUILD").await.unwrap();
        assert_eq!(true, contents.contains("The popular web browser by Google (Stable Channel) test"));
        remove_dir_all("data").await.unwrap();
    }
}