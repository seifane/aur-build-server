use std::error::Error;
use std::fmt;
use std::process::ExitStatus;

#[derive(Debug, Clone)]
pub struct PackageBuildError{
    pub message: String,
    pub exit_code: Option<ExitStatus>
}

impl PackageBuildError {
    pub fn new(message: String, exit_code: Option<ExitStatus>) -> PackageBuildError {
        PackageBuildError {
            message,
            exit_code
        }
    }
}

impl Error for PackageBuildError {}

impl fmt::Display for PackageBuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Package build error with status code {:?}", self.exit_code)
    }
}