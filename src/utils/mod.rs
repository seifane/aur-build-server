use std::{env, fs};
use std::path::Path;
use std::process::{Command, Output};
use git2::{ObjectType, Repository};
use relative_path::RelativePath;
use crate::args::Args;
use clap::Parser;

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

    return Ok(true);
}

pub fn build_package(repo_name: &str) -> std::io::Result<Output> {
    let args: Args = Args::parse();

    let mut cmd_args: String = String::new();

    if args.sign {
        cmd_args += " --sign";
    }

    Command::new("sh")
        .arg("-c")
        .arg(format!("cd data/{}; makepkg --syncdeps --clean --noconfirm{}", repo_name, cmd_args)).output()
}

pub fn copy_packages() {
    let serve_path = Path::new("./serve");
    if serve_path.exists() {
        fs::remove_dir_all(serve_path).unwrap();
    }
    fs::create_dir(serve_path).unwrap();

    let data_path = Path::new("./data");
    for path in fs::read_dir(data_path).unwrap() {
        let res = path.unwrap();
        if !res.path().is_dir() {
            continue
        }
        println!("Moving packages in {}", res.path().to_str().unwrap());
        for file in fs::read_dir(res.path()).unwrap() {
            let file_res = file.unwrap();
            if file_res.file_name().to_str().unwrap().contains(".pkg.tar.zst") {
                let new_path = format!("./serve/{}", file_res.file_name().to_str().unwrap());
                let move_path = RelativePath::new(new_path.as_str());
                fs::rename(file_res.path(), move_path.to_path(env::current_dir().unwrap())).unwrap();
                println!("Moved {} to {}", file_res.path().to_str().unwrap(), move_path.to_path(env::current_dir().unwrap()).to_str().unwrap());
            }
        }
    }

    Command::new("sh")
        .arg("-c")
        .arg("cd serve; repo-add tiemajor.db.tar.gz *.pkg.tar.zst").output().unwrap();
}

pub fn make_package(package_name: String) -> Result<(), Box<dyn std::error::Error>>  {
    println!("Cloning {} ...", package_name);

    let changed = clone_repo(package_name.as_str())?;
    if changed {
        println!("Building {} ...", package_name);
        build_package(package_name.as_str())?;
    }
    Ok(())
}
