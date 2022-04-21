use std::process::Command;

pub fn is_package_in_repo(package_name: &String) -> bool {
    let output = Command::new("/sbin/pacman")
        .arg("-Ss")
        .arg(format!("^{}$", package_name))
        .output()
        .unwrap();

    output.status.success()
}

pub fn split_repo_aur_packages(packages: Vec<String>) -> (Vec<String>, Vec<String>)
{
    let mut repo_packages = Vec::new();
    let mut aur_packages = Vec::new();

    for package in packages.iter() {
        if is_package_in_repo(package) {
            repo_packages.push(package.clone());
        } else {
            aur_packages.push(package.clone());
        }
    }

    (repo_packages, aur_packages)
}