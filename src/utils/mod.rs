use std::{env, fmt, fs, io};
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, ExitStatus};
use std::sync::{Arc, Condvar, Mutex};
use git2::{ObjectType, Repository};
use relative_path::RelativePath;
use crate::args::Args;
use clap::Parser;
use regex::Regex;
use simple_error::SimpleError;
use crate::package_manager::Package;
use std::string::String;


#[derive(Debug, Clone)]
pub struct PackageBuildError{
    pub exit_code: ExitStatus
}

impl PackageBuildError {
    pub fn new(exit_code: ExitStatus) -> PackageBuildError {
        PackageBuildError {
            exit_code
        }
    }
}

impl Error for PackageBuildError {

}

impl fmt::Display for PackageBuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Package build error with status code {:?}", self.exit_code)
    }
}

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

pub fn write_logs(repo_name: &str, data: &[u8], suffix: &str) -> Result<(), io::Error> {
    let log_dir_path = Path::new("logs");
    if !log_dir_path.exists() {
        fs::create_dir(log_dir_path).unwrap();
    }
    let path = format!("logs/{}_{}.log", repo_name, suffix).to_string();
    let log_path = Path::new(&path);
    if log_path.exists() {
        fs::remove_file(log_path)?;
    }
    let mut file = File::create(log_path).unwrap();
    file.write_all(data)?;
    Ok(())
}

pub fn read_file_to_string(path: &str) -> Result<String, io::Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn read_log(repo_name: &str, suffix: &str) -> Result<String, io::Error> {
    let path = format!("logs/{}_{}.log", repo_name, suffix).to_string();
    read_file_to_string(path.as_str())
}

pub fn read_dependencies(package: &Package, dependency_type: &str) -> Result<Vec<String>, io::Error> {
    let path = format!("data/{}/PKGBUILD", package.name);
    let pkgbuild = read_file_to_string(path.as_str()).unwrap();

    let mut deps = vec![];

    let search = format!("{}=(", dependency_type);
    let found_opt = pkgbuild.find(&search);
    if found_opt.is_none() {
        return Ok(deps);
    }
    let found = found_opt.unwrap();
    let found_end = pkgbuild.get(found..).unwrap().find(")").map(|i| i + found).unwrap();

    let depends = pkgbuild.get(found..found_end).unwrap();

    let re = Regex::new(r"'([^']+)'").unwrap();
    for cap in re.captures_iter(depends) {
        deps.push(String::from(cap.get(1).unwrap().as_str()));
    }
    Ok(deps)
}

pub fn build_package(package: &Package) -> std::result::Result<(), PackageBuildError> {
    let args: Args = Args::parse();

    let mut cmd_args: String = String::new();

    if args.sign {
        cmd_args += " --sign";
    }

    if package.run_before.is_some() {
        let pre_run_output = Command::new("sh")
            .arg("-c")
            .arg(package.run_before.as_ref().unwrap()).output();

        let out = pre_run_output.unwrap();

        write_logs(package.name.as_str(), out.stdout.as_slice(), "stdout_before").unwrap_or(());
        write_logs(package.name.as_str(), out.stderr.as_slice(), "stderr_before").unwrap_or(());

        let status_code = out.status;
        if status_code.code().unwrap() != 0 {
            println!("Failed run_before for {} with code {}", package.name, status_code.code().unwrap());
            return Err(PackageBuildError::new(status_code));
        }
    }

    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("cd data/{}; makepkg --syncdeps --clean --noconfirm{}", package.name, cmd_args)).output();

    let out = output.unwrap();

    write_logs(package.name.as_str(), out.stdout.as_slice(), "stdout").unwrap_or(());
    write_logs(package.name.as_str(), out.stderr.as_slice(), "stderr").unwrap_or(());

    let status_code = out.status;
    if status_code.code().unwrap() != 0 {
        println!("Failed makepkg for {} with code {}", package.name, status_code.code().unwrap());
        return Err(PackageBuildError::new(status_code));
    }

    Ok(())
}

pub fn parse_opt_deps(depends: Vec<String>) -> Vec<String> {
    let mut parsed: Vec<String> = Vec::new();

    for item in depends.iter() {
        let mut split = item.split(':');
        let package_name = split.next();
        if package_name.is_some()  {
            parsed.push(package_name.unwrap().to_string());
        }
    }

    parsed
}

pub fn copy_package_to_repo(repo_name: String) -> Result<(), Box<dyn Error>>{
    println!("Copying packages for {}", repo_name);

    let serve_path = Path::new("serve");
    if !serve_path.exists() {
        fs::create_dir(serve_path).unwrap();
    }

    let repo_data_str = format!("data/{}", repo_name).to_string();
    let repo_data_path = Path::new(repo_data_str.as_str());

    for file in fs::read_dir(repo_data_path)? {
        let file_res = file?;
        if file_res.file_name().to_str().unwrap().contains(".pkg.tar.zst") {
            let new_path = format!(
                "serve/{}",
                file_res.file_name().to_str().ok_or(SimpleError::new("Failed to format path"))?
            );
            let move_path = RelativePath::new(new_path.as_str());

            fs::rename(file_res.path(), move_path.to_path(env::current_dir()?))?;
        }
    }

    Ok(())
}

pub fn build_repo(repo_name: String) -> Result<(), Box<dyn Error>> {
    Command::new("sh")
        .arg("-c")
        .arg(format!("cd serve; repo-add {}.db.tar.gz *.pkg.tar.zst", repo_name)).output().unwrap();

    Ok(())
}

pub fn install_dependencies(package: &Package, dependency_lock: Arc<(Mutex<bool>, Condvar)>) {
    let &(ref lock, ref cvar) = &*dependency_lock;

    {
        let mut guard = lock.lock().unwrap();
        println!("Locked");
        while *guard {
            println!("Waiting for lock to install dependencies");
            guard = cvar.wait(guard).unwrap();
        }
        *guard = true;
    }

    let mut deps = read_dependencies(package, "depends").unwrap();
    deps.extend(read_dependencies(package, "makedepends").unwrap());
    deps.extend(read_dependencies(package, "checkdepends").unwrap());
    deps.extend(parse_opt_deps(
        read_dependencies(package, "optdepends").unwrap()
    ));

    println!("Getting dependencies : {}", deps.join(", "));

    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("sudo pacman -Sy --noconfirm {}", deps.join(" "))).output().unwrap();
    println!("{} {}", String::from_utf8(output.stdout).unwrap(), String::from_utf8(output.stderr).unwrap());

    *lock.lock().unwrap() = false;
    cvar.notify_one();
}

pub fn make_package(package: &Package, dependency_lock: Arc<(Mutex<bool>, Condvar)>, force: bool) -> Result<(), Box<dyn std::error::Error>>  {
    println!("Cloning {} ...", package.name);

    let changed = clone_repo(package.name.as_str())?;
    if changed || force {
        install_dependencies(package, dependency_lock);
        println!("Building {} ...", package.name);
        build_package(package)?;
    }
    Ok(())
}
