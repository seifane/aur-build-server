use std::process::{ExitStatus, Output};

pub mod git;
pub mod makepkg;
pub mod pacman;

pub struct CommandResult {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    pub fn from_output(output: Output) -> Self {
        CommandResult {
            status: output.status.clone(),
            stdout: String::from_utf8(output.stdout).unwrap(),
            stderr: String::from_utf8(output.stderr).unwrap(),
        }
    }

    pub fn success(&self) -> bool {
        self.status.success()
    }
}