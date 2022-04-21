use std::env::current_dir;
use std::process::Command;

use clap::Parser;
use srcinfo::Srcinfo;

use crate::args::Args;
use crate::errors::package_build_error::PackageBuildError;
use crate::utils::log::write_logs;
use crate::utils::pkgbuild::sanitize_dependency;

pub fn build(package_name: &String, install: bool) -> Result<(), PackageBuildError> {
    let args: Args = Args::parse();

    let mut cmd_args: String = String::new();

    if args.sign {
        cmd_args += " --sign";
    }

    if install {
        cmd_args += " --install";
    }

    debug!("Running makepkg for {}", package_name);

    let output = Command::new("sh")
        .arg("-c")
        .arg(format!("cd data/{}; makepkg --syncdeps --clean --noconfirm{}", package_name, cmd_args)).output();

    let out = output.unwrap();

    write_logs(package_name.as_str(), out.stdout.as_slice(), "stdout").unwrap_or(());
    write_logs(package_name.as_str(), out.stderr.as_slice(), "stderr").unwrap_or(());

    let status_code = out.status;
    if status_code.code().unwrap() != 0 {
        error!("Failed makepkg for {} with code {}", package_name, status_code.code().unwrap());
        return Err(PackageBuildError::new(status_code));
    }

    Ok(())
}

pub fn get_src_info(package_name: &String) -> Option<Srcinfo> {
    let mut path = current_dir().unwrap();
    path.push("data/");
    path.push(package_name);

    let output = Command::new("/sbin/makepkg")
        .arg("--printsrcinfo")
        .current_dir(path)
        .output();

    if output.is_err() {
        return None;
    }

    let srcinfo = Srcinfo::parse_buf(output.unwrap().stdout.as_slice());

    if srcinfo.is_ok() {
        return Some(srcinfo.unwrap());
    }
    None
}

pub fn get_dependencies(package_name: &String) -> Vec<String> {
    let mut deps = Vec::new();

    let src_info = get_src_info(package_name);
    if src_info.is_none() {
        return deps;
    }

    let src_deps = src_info.unwrap().pkgs[0].clone();

    if src_deps.depends.len() > 0 {
        for dep in src_deps.depends[0].vec.iter() {
            deps.push(sanitize_dependency(dep.as_str()))
        }
    }

    deps
}