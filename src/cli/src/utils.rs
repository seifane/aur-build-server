use common::models::{PackageStatus, WorkerStatus};

pub enum UnifiedColor {
    Red,
    Yellow,
    Green,
    Magenta,
    Cyan,
    Blue,
    White
}

impl Into<cli_table::Color> for UnifiedColor {
    fn into(self) -> cli_table::Color {
        match self {
            UnifiedColor::Red => cli_table::Color::Red,
            UnifiedColor::Yellow => cli_table::Color::Yellow,
            UnifiedColor::Green => cli_table::Color::Green,
            UnifiedColor::Magenta => cli_table::Color::Magenta,
            UnifiedColor::Cyan => cli_table::Color::Cyan,
            UnifiedColor::Blue => cli_table::Color::Blue,
            UnifiedColor::White => cli_table::Color::White,
        }
    }
}

impl Into<colored::Color> for UnifiedColor {
    fn into(self) -> colored::Color {
        match self {
            UnifiedColor::Red => colored::Color::Red,
            UnifiedColor::Yellow => colored::Color::Yellow,
            UnifiedColor::Green => colored::Color::Green,
            UnifiedColor::Magenta => colored::Color::Magenta,
            UnifiedColor::Cyan => colored::Color::Cyan,
            UnifiedColor::Blue => colored::Color::Blue,
            UnifiedColor::White => colored::Color::White,
        }
    }
}

pub fn get_color_from_worker_status(status: &WorkerStatus) -> UnifiedColor
{
    match status {
        WorkerStatus::UNKNOWN => UnifiedColor::Red,
        WorkerStatus::INIT => UnifiedColor::Yellow,
        WorkerStatus::STANDBY => UnifiedColor::Green,
        WorkerStatus::DISPATCHED => UnifiedColor::Magenta,
        WorkerStatus::UPDATING => UnifiedColor::Magenta,
        WorkerStatus::WORKING => UnifiedColor::Cyan,
        WorkerStatus::UPLOADING => UnifiedColor::Blue,
        WorkerStatus::CLEANING => UnifiedColor::Yellow,
    }
}

pub fn get_color_from_package_status(status: &PackageStatus) -> UnifiedColor
{
    match status {
        PackageStatus::UNKNOWN => UnifiedColor::White,
        PackageStatus::PENDING => UnifiedColor::Magenta,
        PackageStatus::BUILDING => UnifiedColor::Yellow,
        PackageStatus::BUILT => UnifiedColor::Green,
        PackageStatus::FAILED => UnifiedColor::Red,
    }
}