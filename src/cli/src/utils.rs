use colored::{ColoredString, Colorize};
use common::models::PackageStatus;

pub fn package_status_to_colored_string(status: &PackageStatus) -> ColoredString
{
    match status {
        PackageStatus::PENDING => {
            "Pending".to_string().magenta().bold()
        }
        PackageStatus::BUILDING => {
            "Building".to_string().yellow().bold()
        }
        PackageStatus::BUILT => {
            "Built".to_string().green().bold()
        }
        PackageStatus::FAILED => {
            "Failed".to_string().red().bold()
        }
    }
}