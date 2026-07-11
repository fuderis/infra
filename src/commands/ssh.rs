use super::SshConnection;
use crate::{TunnelAction, prelude::*};

use std::fs;
use std::path::Path;
use tokio::process::Command;

/// Defining the local runtime tunnel pid path
fn get_pid_file(port: u16) -> String {
    str!("/tmp/infra-tunnel-{port}.pid")
}

/// Lists all configured infrastructure hosts from settings
pub async fn handle_list() -> Result<()> {
    // load global configurations
    let settings = Settings::get();
    println!(":: Available hosts from settings.toml:");

    // iterate and display each registered server
    for (name, host) in &settings.remote.hosts {
        println!(
            "  {} {} {}@{}",
            name.bold(),
            "->".blue(),
            host.user_name,
            host.ip_addr
        );
    }
    Ok(())
}

/// Spawns an interactive ssh terminal session
pub async fn handle_connect(target: &Option<String>) -> Result<()> {
    // resolve ssh connection details for the target host
    let conn = super::get_ssh_conn(target)?;

    println!(
        ":: Establishing interactive SSH session to {}...",
        conn.target
    );

    // execute native ssh passing through configuration arguments
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
    action: TunnelAction,
    port: Option<u16>,
) -> Result<()> {
    // resolve ssh connection details for the target host
    let conn = super::get_ssh_conn(target)?;
    let port = port.unwrap_or(1080);

    match action {
        TunnelAction::Start { gateway } => handle_tunnel_start(&conn, gateway, port).await?,
        TunnelAction::Stop => handle_tunnel_stop(port).await?,
        TunnelAction::Restart { gateway } => handle_tunnel_restart(target, gateway, port).await?,
        TunnelAction::Status => handle_tunnel_status(port).await?,
    }
    Ok(())
}

/// Allocates resources and boots the background monitoring proxy daemon loop
async fn handle_tunnel_start(conn: &SshConnection, gateway: bool, port: u16) -> Result<()> {
    println!(":: Starting persistent SOCKS5 SSH tunnel on port {port}...");
    let pid_file = get_pid_file(port);

    // check if a daemon process lockfile already exists
    if Path::new(&pid_file).exists() {
        let pid = fs::read_to_string(&pid_file)?.trim().to_string();

        // verify if the process linked to the pid is actually running
        if Command::new("kill")
            .args(["-0", &pid])
            .status()
            .await?
            .success()
        {
            return Err(Error::Operational(format!(
                "Tunnel daemon is already active (PID: {}).",
                pid
            ))
            .into());
        }
    }

    // verify that the local socket port is not allocated by another daemon
    if Command::new("fuser")
        .args([str!("port/tcp")])
        .output()
        .await?
        .status
        .success()
    {
        return Err(Error::Operational(str!("Port {port} already is busy.")).into());
    }

    let gateway_flag = if gateway { "-g " } else { "" };

    // construct a script running inside a clean session group to trap children PIDs
    let daemon_script = format!(
        "setsid nohup bash -c 'while true; do ssh {} -D {port} -C -N {} -o ServerAliveInterval=30 -o ServerAliveCountMax=3 -o ExitOnForwardFailure=yes {}; sleep 1; done' >/dev/null 2>&1 & echo $! > {}",
        conn.args.join(" "),
        gateway_flag,
        conn.target,
        &pid_file
    );

    // trigger the background worker script via local bash instance
    Command::new("bash")
        .arg("-c")
        .arg(&daemon_script)
        .status()
        .await?;

    println!(
        "{} SSH tunnel spawned successfully for: {} {}",
        super::ok(),
        conn.target,
        if gateway {
            "(Gateway mode enabled)"
        } else {
            ""
        }
    );
    Ok(())
}

/// Disconnects and terminates running tunnel session groups safely
async fn handle_tunnel_stop(port: u16) -> Result<()> {
    println!(":: Disconnecting SOCKS5 tunnel sessions...");
    let pid_file = get_pid_file(port);

    // process termination layer using pgid to catch orphaned child processes
    if Path::new(&pid_file).exists() {
        let pid = fs::read_to_string(&pid_file)?.trim().to_string();

        // try to kill the process group via pkill using the saved group id
        let _ = Command::new("pkill")
            .args(["-9", "-g", &pid])
            .output() // swallows stdout/stderr automatically
            .await;

        // fallback: kill the main process directly if group tracking missed it
        let _ = Command::new("kill").args(["-9", &pid]).output().await;

        let _ = fs::remove_file(&pid_file);
    }

    // forcibly release any processes binding the forwarding port
    let _ = Command::new("fuser")
        .args(["-k", "-9"])
        .arg(str!("{port}/tcp"))
        .output()
        .await;

    // terminate residual background ssh forwarding sessions matching the pattern
    let _ = Command::new("pkill")
        .args(["-9", "-f"])
        .arg(str!("ssh.*-D {port}"))
        .output()
        .await;

    println!("{} SOCKS5 SSH tunnel closed", super::ok());
    Ok(())
}

/// Cycles the active network daemon offline and online
async fn handle_tunnel_restart(target: &Option<String>, gateway: bool, port: u16) -> Result<()> {
    handle_tunnel_stop(port).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // re-resolve configuration tree mappings for the startup pass
    let conn = super::get_ssh_conn(target)?;
    handle_tunnel_start(&conn, gateway, port).await?;
    Ok(())
}

/// Audits current process status matrices for active tunnel allocations
async fn handle_tunnel_status(port: u16) -> Result<()> {
    // scan process snapshot tables for alive background forwarding instances
    let status = Command::new("pgrep")
        .args(["-fl"])
        .arg(str!("ssh.*-D {port}"))
        .output()
        .await?;

    // display operational state back to stdout
    if status.status.success() && !status.stdout.is_empty() {
        println!(":: Tunnel status: {}", "ACTIVE".green());
        std::io::Write::write_all(&mut std::io::stdout(), &status.stdout)?;
    } else {
        println!(":: Tunnel status: {}", "INACTIVE".red());
    }
    Ok(())
}
