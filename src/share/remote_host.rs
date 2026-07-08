use crate::prelude::*;

/// The remote host options
#[derive(Debug, Display, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[display(fmt = "{user_name}@{ip_addr}")]
pub struct RemoteHost {
    pub user_name: String,
    pub ip_addr: String,
    pub ssh_file: Option<PathBuf>,
}

impl RemoteHost {
    /// Creates a new remote host options
    pub fn new(name: impl Into<String>, ip: impl Into<String>, ssh: Option<PathBuf>) -> Self {
        Self {
            user_name: name.into(),
            ip_addr: ip.into(),
            ssh_file: ssh,
        }
    }
}
