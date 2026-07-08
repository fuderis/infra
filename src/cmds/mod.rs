pub mod net;
pub mod secure;
pub mod ssh;
pub mod sync;
pub mod system;
pub mod user;

use crate::{RemoteHost, prelude::*};

pub struct SshConnection {
    pub target: String,
    pub args: Vec<String>,
}

pub fn get_remote_host(target: &Option<String>) -> Result<RemoteHost> {
    let cfg = Settings::get();
    let host_name = target.as_deref().unwrap_or(&cfg.remote.default);

    Ok(cfg
        .remote
        .hosts
        .get(host_name)
        .map(Clone::clone)
        .ok_or(Error::UnknownHost(host_name.into()))?)
}

pub fn get_ssh_conn(target: &Option<String>) -> Result<SshConnection> {
    let host = get_remote_host(target)?;

    let target = str!("{}@{}", host.user_name, host.ip_addr);
    let mut args = Vec::new();

    if let Some(ref key_path) = host.ssh_file {
        args.push("-i".to_string());
        args.push(key_path.to_string_lossy().into_owned());
    }

    Ok(SshConnection { target, args })
}

pub fn block() -> ColoredString {
    "==>".blue()
}
pub fn ok() -> ColoredString {
    " ->".green()
}
pub fn err() -> ColoredString {
    " ->".red()
}
pub fn warn() -> ColoredString {
    " ->".yellow()
}
