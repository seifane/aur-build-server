use std::path::PathBuf;
use anyhow::{Context, Result};

use tokio::fs::{create_dir, File, OpenOptions, remove_dir_all};
use tokio::io::AsyncWriteExt;

pub enum LogSection {
    RunBeforeOut,
    RunBeforeErr,
    MakePkgOut(String),
    MakePkgErr(String),
}

impl LogSection {
    pub fn to_header_string(&self) -> String
    {
        let message = match self {
            LogSection::RunBeforeOut => "run_before StdOut".to_string(),
            LogSection::RunBeforeErr => "run_before StdErr".to_string(),
            LogSection::MakePkgOut(package) => format!("MakePkg {} StdOut", package),
            LogSection::MakePkgErr(package) => format!("MakePkg {} StdErr", package),
        };

        format!("----------------------------\n{message}\n----------------------------\n")
    }
}

async fn get_log_file_handle(path: &PathBuf) -> Result<File> {
    let parent = path.parent()
        .with_context(|| format!("Failed to get parent for path {:?}", path))?;
    if !parent.exists() {
        create_dir(parent).await?;
    }
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path).await?;
    Ok(file)
}

async fn write_section_header(path: &PathBuf, section: LogSection) -> Result<()> {
    let mut file = get_log_file_handle(path).await?;
    file.write(section.to_header_string().as_bytes()).await?;
    Ok(())
}

async fn write_log(path: &PathBuf, data: &[u8]) -> Result<()> {
    let mut file = get_log_file_handle(path).await?;
    file.write(data).await?;
    Ok(())
}

pub async fn write_log_section(path: &PathBuf, package_name: &String, section: LogSection, data: &[u8]) -> Result<()>
{
    let path = path.join(format!("{}.log", package_name));
    write_section_header(&path, section).await?;
    write_log(&path, data).await?;
    Ok(())
}

pub async fn init_builder_logs(path: &PathBuf) -> Result<()>
{
    if path.exists() {
        remove_dir_all(&path).await?;
    }
    create_dir(&path).await?;
    Ok(())
}
