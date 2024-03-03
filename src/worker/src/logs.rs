use std::path::Path;
use tokio::fs::{create_dir, File, OpenOptions};
use tokio::io;
use tokio::io::AsyncWriteExt;

async fn get_log_file_handle(package_name: &str) -> Result<File, io::Error> {
    let log_dir_base_path = Path::new("worker_logs");
    if !log_dir_base_path.exists() {
        create_dir(log_dir_base_path).await?;
    }
    let log_file_path = log_dir_base_path.join(format!("{package_name}.log"));
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_file_path).await?;
    Ok(file)
}

pub enum LogSection {
    RunBeforeOut,
    RunBeforeErr,
    DepsOut(String),
    DepsErr(String),
    MakePkgOut,
    MakePkgErr,
    BuildError,
}

impl LogSection {
    pub fn to_header_string(&self) -> String
    {
        let message = match self {
            LogSection::RunBeforeOut => "run_before StdOut".to_string(),
            LogSection::RunBeforeErr => "run_before StdErr".to_string(),
            LogSection::DepsOut(name) => format!("Dependency {name} StdOut"),
            LogSection::DepsErr(name) => format!("Dependency {name} StdErr"),
            LogSection::MakePkgOut => "MakePkg StdOut".to_string(),
            LogSection::MakePkgErr => "MakePkg StdErr".to_string(),
            LogSection::BuildError => "Build Error".to_string()
        };

        format!("----------------------------\n{message}\n----------------------------\n")
    }
}

pub async fn write_section_header(package_name: &str, section: LogSection) -> Result<(), io::Error> {
    let mut file = get_log_file_handle(package_name).await?;
    file.write(section.to_header_string().as_bytes()).await?;
    Ok(())
}

pub async fn write_log(package_name: &str, data: &[u8]) -> Result<(), io::Error> {
    let mut file = get_log_file_handle(package_name).await?;
    file.write(data).await?;
    Ok(())
}

pub async fn write_log_section(package_name: &str, section: LogSection, data: &[u8]) -> Result<(), io::Error>
{
    write_section_header(package_name, section).await?;
    let mut file = get_log_file_handle(package_name).await?;
    file.write_all(data).await?;
    Ok(())
}

