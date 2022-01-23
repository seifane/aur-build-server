
struct App {
    pub package_manager: PackageManager
}

impl App {
    pub fn new() {
        App {
            package_manager: PackageManager::new()
        }
    }
}