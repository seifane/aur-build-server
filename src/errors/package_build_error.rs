use std::error::Error;
use std::fmt;
use std::process::ExitStatus;

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