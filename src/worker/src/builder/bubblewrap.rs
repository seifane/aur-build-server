use anyhow::{Result};
use std::path::PathBuf;
use std::process::{Output};
use log::{debug, error, info};
use tokio::fs::{create_dir_all, remove_dir_all};
use tokio::process::Command;
use crate::models::config::Config;
use crate::utils::{copy_dir, get_package_dir_entries, set_recursive_permissions};

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

    pub async fn delete(&self, name: &str) -> Result<()> {
        let path = self.sandbox_path.join(name);
        if path.exists() {
            info!("Deleting {}", name);
            set_recursive_permissions(&path, "777").await?;
            remove_dir_all(&path).await?;
        }
        Ok(())
    }

    pub async fn create(&self, force: bool) -> Result<()>
    {
        if force {
            let _ = remove_dir_all(self.sandbox_path.join("base")).await;
        } else {
            if self.sandbox_path.join("base").exists() {
                info!("Base sandbox already present, not creating");
                return Ok(());
            }
        }

        info!("Creating new base sandbox");

        debug!("Creating sandbox folders");
        create_dir_all(&self.sandbox_path.join("base/etc")).await?;
        create_dir_all(&self.sandbox_path.join("base/var/lib/pacman")).await?;
        create_dir_all(&self.sandbox_path.join("base/etc/pacman.d")).await?;

        debug!("Copying pacman.conf");
        tokio::fs::copy(&self.pacman_config_path, &self.sandbox_path.join("base/etc/pacman.conf")).await?;
        debug!("Copying mirrorlist");
        tokio::fs::copy(&self.pacman_mirrorlist_path, &self.sandbox_path.join("base/etc/pacman.d/mirrorlist")).await?;
        debug!("Copying locale.gen");
        tokio::fs::copy("/etc/locale.gen", &self.sandbox_path.join("base/etc/locale.gen")).await?;

        let res = Command::new("fakechroot")
            .args(vec![
                "fakeroot",
                "pacman",
                "-Syu",
                "--noconfirm",
                "--root", &self.sandbox_path.join("base/").canonicalize()?.to_str().unwrap(),
                "--dbpath", &self.sandbox_path.join("base/var/lib/pacman").canonicalize()?.to_str().unwrap(),
                "--config", &self.sandbox_path.join("base/etc/pacman.conf").canonicalize()?.to_str().unwrap(),
                "base", "fakeroot", "base-devel"
            ])
            .output().await?;

        debug!(
            "pacman init command command output: {:?}, stdout: {:?}, stderr: {:?}",
            res.status.code(),
            String::from_utf8(res.stdout.clone()),
            String::from_utf8(res.stderr.clone())
        );

        self.run_sandbox(true, "base", "/", "locale-gen", vec![]).await?;
        self.run_sandbox(true, "base", "/", "pacman-key", vec!["--init"]).await?;
        self.run_sandbox(true, "base", "/", "pacman-key", vec!["--populate"]).await?;

        // let (mut reader, writer) = os_pipe::pipe()?;
        //
        // let res = Command::new("pacstrap")
        //     .arg("-N")
        //     .arg(&self.sandbox_path.join("base").canonicalize()?)
        //     .arg("base")
        //     .arg("base-devel")
        //     .stderr(writer.try_clone()?)
        //     .stdout(writer)
        //     .spawn()?;
        //
        // tokio::task::spawn(async move {
        //     let mut buffer = [0u8; 1024];
        //     loop {
        //         match reader.read(&mut buffer) {
        //             Ok(0) => break,
        //             Ok(n) => {
        //               debug!("{}", String::from_utf8_lossy(&buffer[..n]));
        //             }
        //             Err(e) => {
        //                 error!("{}", e);
        //                 break;
        //             }
        //         }
        //     }
        // });
        //
        // let res = res.wait_with_output().await?;
        // if !res.status.success() {
        //     bail!("Failed to create base sandbox");
        // }
        //
        // tokio::fs::remove_file(
        //     self.sandbox_path.join("base")
        //         .join("usr")
        //         .join("lib")
        //         .join("dbus-daemon-launch-helper")
        // ).await?;

        Ok(())
    }

    pub async fn create_from_base(&self, name: &str) -> Result<PathBuf>
    {
        self.delete(name).await?;

        let dest = self.sandbox_path.join(name);
        copy_dir(self.sandbox_path.join("base"), dest.clone()).await?;

        Ok(dest)
    }

    pub async fn run_sandbox(
        &self,
        as_root: bool,
        env: &str,
        chdir: &str,
        program: &str,
        mut program_args: Vec<&str>
    ) -> Result<Output>
    {
        let as_user = if as_root {
            "-r"
        } else {
            "-c"
        };

        let env_path = self.sandbox_path.join(env).canonicalize()?;

        let mut args = vec![
            "--map-auto",
            as_user,
            "--",
            "bwrap",
            "--bind", env_path.to_str().unwrap(), "/",
            "--ro-bind", "/etc/resolv.conf", "/etc/resolv.conf",
            "--perms", "1777",
            "--tmpfs", "/tmp",
            "--proc", "/proc",
            "--dev", "/dev",
            "--chdir", chdir,
        ];
        args.push(program);
        args.append(&mut program_args);


        let res = Command::new("unshare")
            .env("FAKEROOTDONTTRYCHOWN", "true")
            .args(&args)
            .output().await?;


        debug!(
            "unshare command {:?} output: {:?}, stdout: {:?}, stderr: {:?}",
            args,
            res.status.code(),
            String::from_utf8(res.stdout.clone()),
            String::from_utf8(res.stderr.clone())
        );
        Ok(res)
    }

    pub async fn create_from_base_install_packages(&self, name: &str, packages: Vec<PathBuf>) -> Result<PathBuf> {
        let path = self.create_from_base(name).await?;
        if !packages.is_empty() {
            info!("Creating chroot with following packages {:?}", packages);
            let dep_path = path.join("dependencies");
            if !dep_path.exists() {
                create_dir_all(&dep_path).await?;
            }

            for i in packages.iter() {
                let file_name = i.file_name().unwrap();
                tokio::fs::copy(i, &dep_path.join(file_name)).await?;
                debug!("Installing built dep {:?}", file_name);
                let out = self.run_sandbox(true, name, "/dependencies", "pacman", vec![
                    "--noconfirm",
                    "-U",
                    file_name.to_str().unwrap(),
                ]).await?;
                debug!("Dependency install output code {:?}\nstdout:\n{}stderr:\n{}", out.status.code(), String::from_utf8(out.stdout.as_slice().to_vec()).unwrap(), String::from_utf8(out.stderr.as_slice().to_vec()).unwrap());
            }
        }

        Ok(path)
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
}