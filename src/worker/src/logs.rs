use std::io::Read;
use std::path::PathBuf;
use anyhow::{Context, Result};
use log::{log_enabled, Level};
use os_pipe::PipeReader;
use tokio::fs::{create_dir, File, OpenOptions, remove_dir_all};
use tokio::io::{AsyncWriteExt};

pub enum LogSection {
    RunBefore,
    MakePkg(String)
}

impl LogSection {
    pub fn to_header_string(&self) -> String
    {
        let message = match self {
            LogSection::RunBefore => "run_before".to_string(),
            LogSection::MakePkg(package) => format!("MakePkg {}", package),
        };

        format!("----------------------------\n{message}\n----------------------------\n")
    }
}

pub async fn get_log_file_handle(path: &PathBuf) -> Result<File> {
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

pub async fn write_tail_logs(
    mut reader: PipeReader,
    path: Option<PathBuf>,
    section: Option<LogSection>
) -> Result<()> {
    let mut file = if let Some(path) = path {
        let mut file = get_log_file_handle(&path).await?;
        if let Some(section) = section {
            file.write(section.to_header_string().as_bytes()).await?;
        }
        Some(file)
    } else {
        None
    };

    let mut buffer = [0u8; 128];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => {
                return Ok(())
            },
            Ok(n) => {
                if log_enabled!(Level::Debug) {
                    print!("{}", String::from_utf8_lossy(&buffer[..n]));
                }
                if let Some(file) = file.as_mut() {
                    file.write(&buffer[..n]).await?;
                }
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }
}

pub async fn init_builder_logs(path: &PathBuf) -> Result<()>
{
    if path.exists() {
        remove_dir_all(&path).await?;
    }
    create_dir(&path).await?;
    Ok(())
}
