use crate::prelude::*;
use std::path::PathBuf;
use tokio::process::Command;

/// Transports local files or directories to the remote host using rsync over SSH
pub async fn handle_upload(
    target: &Option<String>,
    local_path: &PathBuf,
    remote_path: &str,
) -> Result<()> {
    // resolve SSH connection configuration details
    let conn = super::get_ssh_conn(target)?;

    // early exit if the source payload does not exist locally
    if !local_path.exists() {
        return Err(Error::Operational(format!(
            "Local path does not exist: {}",
            local_path.display()
        ))
        .into());
    }

    println!(
        "{} Transporting {} to {}:{}",
        super::log(),
        local_path.display(),
        conn.target,
        remote_path
    );

    // initialize rsync:
    // -a: archive mode (recursive, preserves symlinks, permissions, times)
    // -z: compress data during data transfer
    // -h: human-readable output metrics
    // --info=progress2: structured real-time single-line transfer progress
    let mut rsync_cmd = Command::new("rsync");
    rsync_cmd.arg("-azh").arg("--info=progress2");

    // inject custom SSH transport arguments (e.g. non-standard ports, key identity paths)
    if !conn.args.is_empty() {
        let ssh_env = format!("ssh {}", conn.args.join(" "));
        rsync_cmd.arg("-e").arg(ssh_env);
    }

    // format target destination descriptor
    let remote_target = format!("{}:{}", conn.target, remote_path);

    // execute the infrastructure file transfer
    let status = rsync_cmd
        .arg(local_path)
        .arg(&remote_target)
        .status()
        .await?;

    if !status.success() {
        return Err(Error::Operational(format!(
            "Failed to transfer files via rsync to {}",
            conn.target
        ))
        .into());
    }

    println!(
        "{} Successfully transferred to {}.",
        super::ok(),
        conn.target
    );

    Ok(())
}

/// Downloads remote files or directories to the local host using rsync over SSH
pub async fn handle_download(
    target: &Option<String>,
    remote_path: &str,
    local_path: &PathBuf,
) -> Result<()> {
    // resolve SSH connection configuration details
    let conn = super::get_ssh_conn(target)?;

    // pre-create local parent directories if they don't exist
    if let Some(parent) = local_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Operational(format!(
                    "Failed to create local destination directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
    }

    println!(
        "{} Downloading {}:{} to {}",
        super::log(),
        conn.target,
        remote_path,
        local_path.display()
    );

    // initialize rsync flags (matches your upload configuration)
    let mut rsync_cmd = Command::new("rsync");
    rsync_cmd.arg("-azh").arg("--info=progress2");

    // inject custom SSH transport arguments (e.g. non-standard ports, key identity paths)
    if !conn.args.is_empty() {
        let ssh_env = format!("ssh {}", conn.args.join(" "));
        rsync_cmd.arg("-e").arg(ssh_env);
    }

    // format target source descriptor
    let remote_target = format!("{}:{}", conn.target, remote_path);

    // execute the infrastructure file transfer (Remote -> Local)
    let status = rsync_cmd
        .arg(&remote_target) // source is remote
        .arg(local_path) // destination is local
        .status()
        .await?;

    if !status.success() {
        return Err(Error::Operational(format!(
            "Failed to download files via rsync from {}",
            conn.target
        ))
        .into());
    }

    println!(
        "{} Successfully downloaded to {}.",
        super::ok(),
        local_path.display()
    );

    Ok(())
}

/// Synchronizes local configuration files to the remote host by reusing `handle_send`
pub async fn handle_sync(target: &Option<String>, sync_config: &str) -> Result<()> {
    let settings = Settings::get();
    let conn = super::get_ssh_conn(target)?;

    let local_home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| Error::Operational("Could not resolve local $HOME directory".into()))?;

    // resolve and aggregate target file arrays depending on the sync_config argument
    let files_to_sync: Vec<PathBuf> = if sync_config == "@" {
        let mut all_files = Vec::new();
        for item in settings.sync.configs.values() {
            all_files.extend(item.files.clone());
        }
        if all_files.is_empty() {
            println!("{} No configurations defined for sync.", super::warn());
            return Ok(());
        }
        all_files
    } else {
        if let Some(config_item) = settings.sync.configs.get(sync_config) {
            config_item.files.clone()
        } else {
            return Err(Error::Operational(format!(
                "Configuration profile '{sync_config}' not found in settings.toml"
            ))
            .into());
        }
    };

    println!(
        "{} Initializing smart dotfile synchronization via rsync",
        super::log()
    );

    // pre-generate target directory structures on the remote host
    let mut mkdir_dirs = Vec::new();
    for path in &files_to_sync {
        if let Ok(relative_path) = path.strip_prefix(&local_home) {
            if let Some(parent) = relative_path.parent() {
                let parent_str = parent.to_string_lossy();
                if !parent_str.is_empty() {
                    mkdir_dirs.push(parent_str.into_owned());
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
        let mkdir_status = Command::new("ssh")
            .args(&conn.args)
            .args([&conn.target, &mkdir_script])
            .status()
            .await?;

        if !mkdir_status.success() {
            return Err(Error::Operational(
                "Failed to verify or create remote destination directories".into(),
            )
            .into());
        }
    }

    // sequentially process and stream files via the centralized rsync pipeline
    for local_path in files_to_sync {
        let remote_path_str = match local_path.strip_prefix(&local_home) {
            Ok(rel) => rel.to_string_lossy().into_owned(),
            Err(_) => local_path.to_string_lossy().into_owned(),
        };

        handle_upload(target, &local_path, &remote_path_str).await?;
    }

    Ok(())
}
