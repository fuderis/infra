use crate::prelude::*;

/// The synchronization config
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct SyncConfig {
    pub files: Vec<PathBuf>,
}

impl SyncConfig {
    /// Creates a new sync config
    pub fn new(files: Vec<PathBuf>) -> Self {
        Self { files }
    }
}
