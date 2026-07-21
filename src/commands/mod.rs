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

pub fn get_remote_host_ext(target: &Option<String>, ip: &Option<String>) -> Result<RemoteHost> {
    let cfg = Settings::get();

    // 1. Specific IP [--ip]
    if let Some(explicit_ip) = ip {
        // search in config similarity ip
        if let Some(found_host) = cfg
            .remote
            .hosts
            .values()
            .find(|h| &h.ip_addr == explicit_ip)
        {
            return Ok(found_host.clone());
        }

        // or use default options
        return Ok(RemoteHost {
            ip_addr: explicit_ip.clone(),
            user_name: "root".to_string(),
            ssh_file: None,
        });
    }

    // 2. Search by target name
    let host_name = target.as_deref().unwrap_or(&cfg.remote.default);
    cfg.remote
        .hosts
        .get(host_name)
        .cloned()
        .ok_or_else(|| Error::UnknownHost(host_name.to_string()).into())
}

pub fn get_ssh_conn(target: &Option<String>, ip: &Option<String>) -> Result<SshConnection> {
    let host = get_remote_host_ext(target, ip)?;
    let target_str = format!("{}@{}", host.user_name, host.ip_addr);
    let mut args = Vec::new();

    if let Some(ref key_path) = host.ssh_file {
        args.push("-i".to_string());
        args.push(key_path.to_string_lossy().into_owned());
    }

    Ok(SshConnection {
        target: target_str,
        args,
    })
}

pub fn get_remote_host(
    target: &Option<String>,
    explicit_ip: &Option<String>,
) -> Result<RemoteHost> {
    let cfg = Settings::get();

    // 1. If an explicit IP is passed via --ip
    if let Some(ip) = explicit_ip {
        // looking for a host in the config that has the same IP (to pull up its user and key, if it is there)
        if let Some(found_host) = cfg.remote.hosts.values().find(|h| &h.ip_addr == ip) {
            return Ok(found_host.clone());
        }

        // if there is nothing with such an IP in the configuration, we take the default host as a template.
        // (to copy the default user_name and ssh_file), but change the IP to the specified one
        if let Some(default_host) = cfg.remote.hosts.get(&cfg.remote.default) {
            let mut host = default_host.clone();
            host.ip_addr = ip.clone(); // Подменяем IP на лету
            return Ok(host);
        }

        // even if there is no default host in the config, we build a minimal RemoteHost.
        return Ok(RemoteHost {
            ip_addr: ip.clone(),
            user_name: "root".to_string(),
            ssh_file: None,
        });
    }

    // 2. If --ip is not specified, we work according to the old logic (we search by host name)
    let host_name = target.as_deref().unwrap_or(&cfg.remote.default);

    cfg.remote
        .hosts
        .get(host_name)
        .cloned()
        .ok_or_else(|| Error::UnknownHost(host_name.to_string()).into())
}

pub fn section(title: &str) {
    println!();
    println!("{} {}", "▶".cyan().bold(), title.bold());
}

pub fn info(label: &str, message: &str) {
    if !label.is_empty() {
        println!(
            "  {} {}{} {}",
            "•".cyan(),
            label.bold(),
            ":".bold(),
            message
        );
    } else {
        println!("  {} {}", "•".cyan(), message);
    }
}

pub fn success(message: &str) {
    println!("  {} {}", "✓".green(), message);
}

pub fn error(e: crate::DynError) {
    if let Some((prefix, tail)) = crate::str!(e).split_once(": ") {
        println!("\n  {} {}{} {tail}", "✗".red(), prefix.red(), ":".red());
    } else {
        println!("\n  {} {e}", "✗".red());
    }
}
