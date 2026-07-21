use super::{SshConnection, info, section, success};
use crate::{TunnelAction, prelude::*};

use std::{fs, path::Path};
use tokio::{
    process::Command,
    time::{Duration, sleep},
};

/// Defining the local runtime tunnel pid path
fn get_pid_file(port: u16) -> String {
    str!("/tmp/infra-tunnel-{port}.pid")
}

/// Lists all configured infrastructure hosts from settings
pub async fn handle_list() -> Result<()> {
    let settings = Settings::get();

    section("Configured Hosts");
    println!();
    println!("{:<20} {:<16} {}", "HOST", "USER", "ADDRESS");
    println!("{}", "─".repeat(60));

    for (name, host) in &settings.remote.hosts {
        println!("{:<20} {:<16} {}", name, host.user_name, host.ip_addr);
    }

    Ok(())
}

/// Spawns an interactive ssh terminal session
pub async fn handle_connect(target: &Option<String>, ip: &Option<String>) -> Result<()> {
    let conn = super::get_ssh_conn(target, ip)?;

    section("SSH Connection");
    info("Target", &conn.target);

    println!();

    Command::new("ssh")
        .args(&conn.args)
        .arg(&conn.target)
        .status()
        .await?;

    Ok(())
}

/// Manages the lifecycle of a persistent background socks5 proxy tunnel
pub async fn handle_tunnel(
    target: &Option<String>,
    ip: &Option<String>,
    action: TunnelAction,
    port: Option<u16>,
) -> Result<()> {
    let conn = super::get_ssh_conn(target, ip)?;
    let port = port.unwrap_or(1080);

    match action {
        TunnelAction::Start { gateway } => handle_tunnel_start(&conn, gateway, port).await?,
        TunnelAction::Stop => handle_tunnel_stop(port).await?,
        TunnelAction::Restart { gateway } => handle_tunnel_restart(&conn, gateway, port).await?,
        TunnelAction::Status => handle_tunnel_status(port).await?,
    }

    Ok(())
}

async fn handle_tunnel_start(conn: &SshConnection, gateway: bool, port: u16) -> Result<()> {
    section("Starting SHH tunnel");
    info("Target", &conn.target);
    info("Port  ", &port.to_string());

    let pid_file = get_pid_file(port);

    if Path::new(&pid_file).exists() {
        let pid = fs::read_to_string(&pid_file)?.trim().to_string();

        if Command::new("kill")
            .args(["-0", &pid])
            .status()
            .await?
            .success()
        {
            return Err(
                Error::Operational(format!("Tunnel already running (PID: {})", pid)).into(),
            );
        }
    }

    if Command::new("fuser")
        .args(["-n", "tcp", &port.to_string()])
        .output()
        .await?
        .status
        .success()
    {
        return Err(Error::Operational(format!("Port {} already in use", port)).into());
    }

    let gateway_flag = if gateway { "-g " } else { "" };
    let daemon_script = format!(
        "setsid nohup bash -c 'while true; do ssh {} -D {port} -C -N {} -o ServerAliveInterval=30 -o ServerAliveCountMax=3 -o ExitOnForwardFailure=yes {}; sleep 1; done' >/dev/null 2>&1 & echo $! > {}",
        conn.args.join(" "),
        gateway_flag,
        conn.target,
        pid_file
    );

    Command::new("bash")
        .arg("-c")
        .arg(&daemon_script)
        .status()
        .await?;

    println!();
    success("Tunnel active");

    if gateway {
        info("", "Gateway mode: enabled");
    }

    Ok(())
}

async fn handle_tunnel_stop(port: u16) -> Result<()> {
    section("Stopping SSH tunnel");
    info("Port  ", &port.to_string());

    let pid_file = get_pid_file(port);

    if Path::new(&pid_file).exists() {
        let pid = fs::read_to_string(&pid_file)?.trim().to_string();

        let _ = Command::new("pkill")
            .args(["-9", "-g", &pid])
            .output()
            .await;

        let _ = Command::new("kill").args(["-9", &pid]).output().await;

        let _ = fs::remove_file(&pid_file);
    }

    let _ = Command::new("fuser")
        .args(["-k", "-9"])
        .arg(str!("{port}/tcp"))
        .output()
        .await;

    let _ = Command::new("pkill")
        .args(["-9", "-f"])
        .arg(str!("ssh.*-D {port}"))
        .output()
        .await;

    println!();
    success("Tunnel stopped");

    Ok(())
}

async fn handle_tunnel_restart(conn: &SshConnection, gateway: bool, port: u16) -> Result<()> {
    handle_tunnel_stop(port).await?;
    sleep(Duration::from_secs(1)).await;
    handle_tunnel_start(conn, gateway, port).await?;
    Ok(())
}

async fn handle_tunnel_status(port: u16) -> Result<()> {
    section("SSH tunnel status");

    let status = Command::new("pgrep")
        .args(["-fl"])
        .arg(str!("ssh.*-D {port}"))
        .output()
        .await?;

    if status.status.success() && !status.stdout.is_empty() {
        info("Status", &"ACTIVE".green().to_string());
    } else {
        info("Status", &"INACTIVE".red().to_string());
    }

    Ok(())
}
