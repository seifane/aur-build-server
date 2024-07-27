use std::path::{Path, PathBuf};
use async_recursion::async_recursion;
use log::debug;
use tokio::fs::{create_dir_all, DirEntry, File, read_dir, read_to_string};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use anyhow::{bail, Result};

/// Removes version requirements on dependency strings.
/// This is used to get the name of the dependency to determine if it is available on repository or has to be fetched on AUR.
///
/// # Arguments
///
/// * `dep`: The dependency requirement as a String
///
/// returns: String
pub fn sanitize_dependency(dep: &str) -> String {
    let mut char_index = 0;
    for c in vec![">", "<", "=", ":"] {
        let found = dep.find(c).unwrap_or(0);
        if char_index == 0 || (found > 0 && found < char_index) {
            char_index = found;
        }
    }
    if char_index > 0 {
        return dep[..char_index].to_string();
    }
    dep.to_string()
}

pub async fn get_package_dir_entries(path: impl AsRef<Path>) -> Result<Vec<DirEntry>>
{
    let mut packages = Vec::new();

    let mut dir = read_dir(path).await?;

    while let Some(entry) = dir.next_entry().await? {
        if entry.file_name().to_str().unwrap().ends_with(".pkg.tar.zst") {
            packages.push(entry);
        }
    }
    Ok(packages)
}

#[async_recursion]
pub async fn copy_dir_all(src: PathBuf, dst: PathBuf) -> Result<()> {
    create_dir_all(&dst).await?;
    let mut dir = read_dir(src).await?;
    while let Some(entry) = dir.next_entry().await? {
        if entry.file_type().await?.is_dir() {
            copy_dir_all(entry.path(), dst.join(entry.file_name())).await?;
        } else {
            tokio::fs::copy(entry.path(), dst.join(entry.file_name())).await?;
        }
    }
    Ok(())
}

// Temporary workaround while fixing copy_dir_all for some types of files
pub async fn copy_dir(src: PathBuf, dst: PathBuf) -> Result<()>
{
    let mut command = Command::new("cp");
    command.arg("-r");
    command.arg(src);
    command.arg(dst);
    let out = command.output().await?;
    if !out.status.success() {
        bail!("Failed to cp, with status code {:?}", out.status.code());
    }
    Ok(())
}

// Temporary fix for https://github.com/rust-lang/rust/issues/127576
pub async fn rm_dir(dir: PathBuf) -> Result<()> {
    let mut command = Command::new("rm");
    command.arg("-rf");
    command.arg(dir);
    let out = command.output().await?;
    if !out.status.success() {
        bail!("Failed to rm dir with status code {:?}", out.status.code());
    }
    Ok(())
}

pub async fn copy_file_contents(src: &PathBuf, dest: &PathBuf) -> Result<()>
{
    debug!("Copy file content from {:?} to {:?}", src, dest);
    let src_contents = read_to_string(src).await?;

    let mut dest_file = File::create(dest).await?;
    dest_file.write_all(src_contents.as_bytes()).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::utils::sanitize_dependency;

    #[test]
    fn test_sanitize_dependency() {
        assert_eq!(
            sanitize_dependency("glibc>=2.28-4").as_str(),
            "glibc"
        );
        assert_eq!(
            sanitize_dependency("jre-runtime=17").as_str(),
            "jre-runtime"
        );
    }
}