use cli_table::Color;
use common::models::{PackageStatus, WorkerStatus};

pub fn get_color_from_worker_status(status: &WorkerStatus) -> Option<Color>
{
    match status {
        WorkerStatus::UNKNOWN => Some(Color::Red),
        WorkerStatus::STANDBY => Some(Color::Green),
        WorkerStatus::DISPATCHED => Some(Color::Magenta),
        WorkerStatus::UPDATING => Some(Color::Magenta),
        WorkerStatus::WORKING => Some(Color::Cyan),
        WorkerStatus::UPLOADING => Some(Color::Blue),
        WorkerStatus::CLEANING => Some(Color::Yellow)
    }
}

pub fn get_color_from_package_status(status: &PackageStatus) -> Option<Color>
{
    match status {
        PackageStatus::PENDING => Some(Color::Magenta),
        PackageStatus::BUILDING => Some(Color::Yellow),
        PackageStatus::BUILT => Some(Color::Green),
        PackageStatus::FAILED => Some(Color::Red)
    }
}