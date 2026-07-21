use super::{info, section};
use crate::prelude::*;
use tokio::process::Command;

/// Measures round-trip latency using icmp echo requests
pub async fn handle_ping(target: &Option<String>, ip: &Option<String>, count: usize) -> Result<()> {
    let host = super::get_remote_host(target, ip)?;

    section("ICMP Ping");
    info("Target", &format!("{} ({})", host, host.ip_addr));

    println!();

    Command::new("ping")
        .args(["-c", &count.to_string(), &host.ip_addr])
        .status()
        .await?;

    Ok(())
}

/// Traces the layer-3 network path to the remote host
pub async fn handle_trace(target: &Option<String>, ip: &Option<String>) -> Result<()> {
    let host = super::get_remote_host(target, ip)?;

    section("Traceroute");
    info("Target", &format!("{} ({})", host, host.ip_addr));

    println!();

    if Command::new("traceroute").arg("-V").output().await.is_err() {
        install_dependency("traceroute").await?;
    }

    Command::new("traceroute")
        .arg(&host.ip_addr)
        .status()
        .await?;

    Ok(())
}

/// Performs continuous network quality analysis using mtr
pub async fn handle_route(target: &Option<String>, ip: &Option<String>) -> Result<()> {
    let host = super::get_remote_host(target, ip)?;

    section("Network Route Quality (MTR)");
    info("Target", &format!("{} ({})", host, host.ip_addr));

    println!();

    if Command::new("mtr").arg("--version").output().await.is_err() {
        install_dependency("mtr").await?;
    }

    let mut mtr_cmd = Command::new("mtr");

    #[cfg(target_os = "linux")]
    {
        mtr_cmd.args(["-rwzc", "10", &host.ip_addr]);
    }

    #[cfg(target_os = "macos")]
    {
        mtr_cmd.args(["-rc", "10", &host.ip_addr]);
    }

    mtr_cmd.status().await?;

    Ok(())
}

async fn install_dependency(package: &str) -> Result<()> {
    section("Dependency");
    info("Missing package", package);

    #[cfg(target_os = "linux")]
    {
        println!("Installing via pacman...");

        Command::new("sudo")
            .args(["pacman", "-S", "--noconfirm", package])
            .status()
            .await?;
    }

    #[cfg(target_os = "macos")]
    {
        println!("Installing via Homebrew...");

        if Command::new("brew")
            .arg("--version")
            .output()
            .await
            .is_err()
        {
            return Err(Error::Operational(
                "Homebrew is required to install missing dependencies on macOS.",
            )
            .into());
        }

        Command::new("brew")
            .args(["install", package])
            .status()
            .await?;
    }

    Ok(())
}
