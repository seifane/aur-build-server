use anyhow::{bail, Result};
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Output;
use log::{debug, info};
use tokio::fs::{create_dir_all, File, remove_dir_all};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use crate::models::config::Config;
use crate::utils::{copy_dir, copy_file_contents, get_package_dir_entries, rm_dir};

pub struct Bubblewrap {
    sandbox_path: PathBuf,
    pacman_config_path: PathBuf,
    pacman_mirrorlist_path: PathBuf,
}

impl Bubblewrap {
    pub fn from_config(config: &Config) -> Bubblewrap
    {
        Bubblewrap {
            sandbox_path: config.sandbox_path.clone(),
            pacman_config_path: config.pacman_config_path.clone(),
            pacman_mirrorlist_path: config.pacman_mirrorlist_path.clone(),
        }
    }

    #[allow(dead_code)] // Used in test cases
    pub fn new(sandbox_path: PathBuf, pacman_config_path: PathBuf, pacman_mirrorlist_path: PathBuf) -> Bubblewrap
    {
        Bubblewrap {
            sandbox_path,
            pacman_config_path,
            pacman_mirrorlist_path,
        }
    }

    pub fn namespace_path(&self, name: &str) -> PathBuf
    {
        self.sandbox_path.join(name)
    }

    pub async fn _delete(&self, name: &str) -> Result<()> {
        let path = self.sandbox_path.join(name);
        if path.exists() {
            remove_dir_all(&path).await?;
        }
        Ok(())
    }

    pub async fn create(&self, force: bool) -> Result<()>
    {
        if force {
            let _ = rm_dir(self.sandbox_path.join("base")).await;
        } else {
            if self.sandbox_path.join("base").exists() {
                info!("Base sandbox already present, not creating");
                return Ok(());
            }
        }

        info!("Creating new base sandbox");

        info!("Creating directories");
        create_dir_all(&self.sandbox_path).await?;
        create_dir_all(self.sandbox_path.join("base/var/lib/pacman")).await?;
        create_dir_all(self.sandbox_path.join("base/etc")).await?;
        create_dir_all(self.sandbox_path.join("base/etc/pacman.d")).await?;

        info!("Setting up configuration");
        copy_file_contents(&self.pacman_config_path, &self.sandbox_path.join("base/etc/pacman.conf")).await?;
        copy_file_contents(&self.pacman_mirrorlist_path, &self.sandbox_path.join("base/etc/pacman.d/mirrorlist")).await?;

        info!("Writing locale.gen");
        let mut file = File::create(self.sandbox_path.join("base/etc/locale.gen")).await?;
        file.write("en_US.UTF-8 UTF-8".as_bytes()).await?;
        drop(file);

        info!("Installing base packages");
        let out = self.run_fakechroot("pacman", vec![
            "-Syu", "--noconfirm", "--root", self.sandbox_path.join("base").to_str().unwrap(),
            "--dbpath", self.sandbox_path.join("base/var/lib/pacman").to_str().unwrap(),
            "--config", self.sandbox_path.join("base/etc/pacman.conf").to_str().unwrap(),
            "base", "fakeroot", "base-devel"
        ]).await?;
        info!("Base package installed finished with output code {:?}", out.status.code());
        debug!("stdout:\n{}stderr:\n{}", String::from_utf8(out.stdout.as_slice().to_vec()).unwrap(), String::from_utf8(out.stderr.as_slice().to_vec()).unwrap());

        info!("Generating locale");
        let out = self.run_sandbox("base","/", "locale-gen", vec![]).await?;
        info!("Locale generation finished with output code {:?}", out.status.code());
        debug!("stdout:\n{}stderr:\n{}", String::from_utf8(out.stdout.as_slice().to_vec()).unwrap(), String::from_utf8(out.stderr.as_slice().to_vec()).unwrap());

        info!("Initializing pacman key");
        let out = self.run_sandbox_fakeroot("base","/", "pacman-key", vec!["--init"]).await?;
        info!("Pacman key initialization finished with code: {:?}", out.status.code());
        debug!("stdout:\n{}stderr:\n{}", String::from_utf8(out.stdout.as_slice().to_vec()).unwrap(), String::from_utf8(out.stderr.as_slice().to_vec()).unwrap());

        info!("Populating pacman keyring");
        let out = self.run_sandbox_fakeroot("base","/", "pacman-key", vec!["--populate"]).await?;
        info!("Pacman keyring population finished with code: {:?}", out.status.code());
        debug!("stdout:\n{}stderr:\n{}", String::from_utf8(out.stdout.as_slice().to_vec()).unwrap(), String::from_utf8(out.stderr.as_slice().to_vec()).unwrap());

        Ok(())
    }

    pub async fn create_from_base(&self, name: &str) -> Result<PathBuf>
    {
        let out = self.run_sandbox_fakeroot("base","/", "pacman", vec!["-Syy"]).await?;
        if !out.status.success() {
            bail!("create_from_base({}): Failed update pacman", name);
        }

        let dest = self.sandbox_path.join(name);
        if dest.exists() {
            rm_dir(dest.clone()).await?;
        }

        copy_dir(self.sandbox_path.join("base"), dest.clone()).await?;

        Ok(dest)
    }

    pub async fn create_from_base_install_packages(&self, name: &str, packages: Vec<PathBuf>) -> Result<PathBuf> {
        let path = self.create_from_base(name).await?;
        if !packages.is_empty() {
            info!("Creating chroot with following packages {:?}", packages);
            let dep_path = path.join("dependencies");
            if !dep_path.exists() {
                create_dir_all(&dep_path).await?;
            }

            let mut args = vec![
                "--noconfirm",
                "-U"
            ];

            for i in packages.iter() {
                let file_name = i.file_name().unwrap();
                args.push(file_name.to_str().unwrap());
                tokio::fs::copy(i, &dep_path.join(file_name)).await?;
            }

            let out = self.run_sandbox_fakeroot(name, "/dependencies", "pacman", args).await?;
            debug!("Dependency install output code {:?}\nstdout:\n{}stderr:\n{}", out.status.code(), String::from_utf8(out.stdout.as_slice().to_vec()).unwrap(), String::from_utf8(out.stderr.as_slice().to_vec()).unwrap());
        }

        Ok(path)
    }

    fn get_sandbox_command<S: AsRef<OsStr>>(&self, sandbox_name: &str, dir: &str,  program: S, args: Vec<&str>) -> Command
    {
        let mut command = Command::new("bwrap");
        command.env("PACMAN_AUTH", "fakeroot");
        command.env("FAKEROOTDONTTRYCHOWN", "true");
        command.env("HOME", "/root");
        command.env("_JAVA_OPTIONS", "-Duser.home=/home/user"); // Fixes gradle issues: https://discuss.gradle.org/t/gradles-wrapper-is-creating-a-folder-called/10905/5

        command.args(vec![
            "--new-session",
            "--bind", self.sandbox_path.join(sandbox_name).to_str().unwrap(), "/",
            "--ro-bind", "/etc/resolv.conf", "/etc/resolv.conf",
            "--tmpfs", "/tmp",
            "--proc", "/proc",
            "--dev", "/dev",
            "--chdir", dir
        ]);
        command.arg(program);
        command.args(args);

        return command;
    }

    pub async fn run_sandbox<S: AsRef<OsStr>>(&self, sandbox_name: &str, dir: &str,  program: S, args: Vec<&str>) -> Result<Output>
    {
        let mut command = self.get_sandbox_command(sandbox_name, dir, program, args);
        let out = command.output().await?;
        Ok(out)
    }

   pub async fn run_sandbox_fakeroot(&self, sandbox_name: &str, dir: &str,  program: &str, args: Vec<&str>) -> Result<Output>
    {
        let mut new_args = vec![program];
        new_args.append(&mut args.clone());

        let mut command = self.get_sandbox_command(sandbox_name, dir, "fakeroot", new_args);

        let out = command.output().await?;
        Ok(out)
    }

    pub async fn copy_built_packages(&self, dest: PathBuf) -> Result<()>
    {
        let path = self.namespace_path("current").join("package");
        let artifacts = get_package_dir_entries(&path).await?;

        for entry in artifacts {
            info!("Copying built package {:?}", entry.path());
            tokio::fs::copy(entry.path(), dest.join(entry.file_name())).await?;
        }

        Ok(())
    }

    async fn run_fakechroot<S: AsRef<OsStr>>(&self, program: S, args: Vec<&str>) -> Result<Output>
    {
        let mut command = Command::new("fakechroot");
        command.arg("fakeroot");
        command.arg(program);
        command.args(args);

        let out = command.output().await?;

        Ok(out)
    }
}