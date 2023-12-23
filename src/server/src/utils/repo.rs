use std::error::Error;
use log::{debug};
use tokio::process::Command;

pub async fn add_packages_to_repo(repo_name: &String, package_files: Vec<String>, sign: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
    if sign {
        for file in package_files.iter() {
            let out = Command::new("gpg")
                .arg("--yes")
                .arg("--output")
                .arg(format!("serve/{}.sig", file))
                .arg("--detach-sig")
                .arg(format!("serve/{}", file))
                .output().await?;
            debug!("GPG output for {} exit code : {:?} {:?} {:?}", file, out.status.code(), String::from_utf8(out.stdout), String::from_utf8(out.stderr))
        }
    } else {
        debug!("Skipping signature ...")
    }

    return build_repo(repo_name, Some(package_files), sign).await;
}

async fn build_repo(repo_name: &String, package_files: Option<Vec<String>>, sign: bool) -> Result<(), Box<dyn Error + Send + Sync>> {
    debug!("Building repo");

    let joined_packages = match package_files {
        None => "*.pkg.tar.zst".to_string(),
        Some(package) => package.join(" ")
    };

    let mut args = Vec::new();
    if sign {
        args.push("--verify".to_string());
        args.push("--sign".to_string());
    }
    args.push(format!("{}.db.tar.gz", repo_name));
    args.push(format!("{}", joined_packages));

    let out = Command::new("repo-add")
        .current_dir("serve/")
        .args(args)
        .output().await?;

    debug!("repo-add output exit code : {:?} {:?} {:?}", out.status.code() ,String::from_utf8(out.stdout), String::from_utf8(out.stderr));

    Ok(())
}