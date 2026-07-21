use super::{info, section, success};
use crate::prelude::*;
use std::path::PathBuf;
use tokio::process::Command;

/// Transports local files or directories to the remote host using rsync over SSH
pub async fn handle_upload(
    target: &Option<String>,
    ip: &Option<String>,
    local_path: &PathBuf,
    remote_path: &str,
) -> Result<()> {
    let conn = super::get_ssh_conn(target, ip)?;

    if !local_path.exists() {
        return Err(Error::Operational(format!(
            "Local path does not exist: {}",
            local_path.display()
        ))
        .into());
    }

    section("File Upload");
    info("Source     ", &local_path.to_string_lossy());
    info("Destination", &format!("{}:{}", conn.target, remote_path));

    println!();
    let mut rsync_cmd = Command::new("rsync");
    rsync_cmd.arg("-azh").arg("--info=progress2");

    if !conn.args.is_empty() {
        let ssh_env = format!("ssh {}", conn.args.join(" "));

        rsync_cmd.arg("-e").arg(ssh_env);
    }

    let remote_target = format!("{}:{}", conn.target, remote_path);

    let status = rsync_cmd
        .arg(local_path)
        .arg(&remote_target)
        .status()
        .await?;

    if !status.success() {
        return Err(Error::Operational(format!("Upload failed: {}", conn.target)).into());
    }

    println!();
    success("Upload completed");

    Ok(())
}

/// Downloads remote files or directories to the local host using rsync over SSH
pub async fn handle_download(
    target: &Option<String>,
    ip: &Option<String>,
    remote_path: &str,
    local_path: &PathBuf,
) -> Result<()> {
    let conn = super::get_ssh_conn(target, ip)?;

    if let Some(parent) = local_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Operational(format!("Cannot create {}: {}", parent.display(), e))
            })?;
        }
    }

    section("File Download");
    info("Source     ", &format!("{}:{}", conn.target, remote_path));
    info("Destination", &local_path.to_string_lossy());

    let mut rsync_cmd = Command::new("rsync");

    rsync_cmd.arg("-azh").arg("--info=progress2");

    if !conn.args.is_empty() {
        let ssh_env = format!("ssh {}", conn.args.join(" "));

        rsync_cmd.arg("-e").arg(ssh_env);
    }

    let remote_target = format!("{}:{}", conn.target, remote_path);

    let status = rsync_cmd
        .arg(remote_target)
        .arg(local_path)
        .status()
        .await?;

    if !status.success() {
        return Err(Error::Operational(format!("Download failed: {}", conn.target)).into());
    }

    println!();
    success("Download completed");

    Ok(())
}

/// Synchronizes local configuration files to the remote host by reusing `handle_send`
pub async fn handle_sync(
    target: &Option<String>,
    ip: &Option<String>,
    sync_config: &str,
) -> Result<()> {
    let settings = Settings::get();

    let conn = super::get_ssh_conn(target, ip)?;

    let local_home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| Error::Operational("Cannot resolve HOME directory".into()))?;

    section("Configuration Sync");
    info("Profile", &sync_config);

    let files_to_sync: Vec<PathBuf> = if sync_config == "@" {
        let mut files = Vec::new();

        for item in settings.sync.configs.values() {
            files.extend(item.files.clone());
        }

        if files.is_empty() {
            info("", "No files configured");
            return Ok(());
        }

        files
    } else {
        match settings.sync.configs.get(sync_config) {
            Some(item) => item.files.clone(),

            None => {
                return Err(
                    Error::Operational(format!("Profile not found: {}", sync_config)).into(),
                );
            }
        }
    };

    info("Files", &files_to_sync.len().to_string());

    let mut mkdir_dirs = Vec::new();

    for path in &files_to_sync {
        if let Ok(relative) = path.strip_prefix(&local_home) {
            if let Some(parent) = relative.parent() {
                if !parent.as_os_str().is_empty() {
                    mkdir_dirs.push(parent.to_string_lossy().into_owned());
                }
            }
        } else {
            if let Some(parent) = path.parent() {
                mkdir_dirs.push(parent.to_string_lossy().into_owned());
            }
        }
    }

    if !mkdir_dirs.is_empty() {
        mkdir_dirs.sort();
        mkdir_dirs.dedup();

        let mkdir_script = format!("mkdir -p {}", mkdir_dirs.join(" "));

        let status = Command::new("ssh")
            .args(&conn.args)
            .args([&conn.target, &mkdir_script])
            .status()
            .await?;

        if !status.success() {
            return Err(Error::Operational("Failed to create remote directories".into()).into());
        }
    }

    for local_path in files_to_sync {
        let remote_path = match local_path.strip_prefix(&local_home) {
            Ok(relative) => relative.to_string_lossy().into_owned(),

            Err(_) => local_path.to_string_lossy().into_owned(),
        };

        handle_upload(target, ip, &local_path, &remote_path).await?;
    }

    Ok(())
}
