#[derive(Clone, Debug)]
pub struct PackageBuildResult {
    pub built: bool,
    pub version: String,
}

impl PackageBuildResult {
    pub fn new(built: bool, version: String) -> PackageBuildResult {
        PackageBuildResult {
            built,
            version,
        }
    }
}

