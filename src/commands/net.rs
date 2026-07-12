use crate::prelude::*;
use tokio::process::Command;

/// Measures round-trip latency using icmp echo requests
pub async fn handle_ping(target: &Option<String>, ip: &Option<String>, count: usize) -> Result<()> {
    // resolve network details for the target host
    let host = super::get_remote_host(target, ip)?;

    println!("{} Monitoring ICMP latency to {host}", super::log());

    // execute system ping with a fixed count of 10 packets
    Command::new("ping")
        .args(["-c", &count.to_string(), &host.ip_addr])
        .status()
        .await?;

    Ok(())
}

/// Traces the layer-3 network path to the remote host
pub async fn handle_trace(target: &Option<String>, ip: &Option<String>) -> Result<()> {
    // resolve network details for the target host
    let host = super::get_remote_host(target, ip)?;

    println!(
        "{} Execution of layer-3 route tracking (traceroute) to {host}",
        super::log()
    );

    // ensure traceroute binary is available before proceeding
    if Command::new("traceroute").arg("-V").output().await.is_err() {
        install_dependency("traceroute").await?;
    }

    // execute path trace to the destination ip
    Command::new("traceroute")
        .arg(&host.ip_addr)
        .status()
        .await?;

    Ok(())
}

/// Performs continuous network quality analysis using mtr
pub async fn handle_route(target: &Option<String>, ip: &Option<String>) -> Result<()> {
    // resolve network details for the target host
    let host = super::get_remote_host(target, ip)?;

    println!(
        "{} Realtime continuous quality network analysis (MTR) to {host}",
        super::log()
    );

    // ensure mtr binary is available before proceeding
    if Command::new("mtr").arg("--version").output().await.is_err() {
        install_dependency("mtr").await?;
    }

    let mut mtr_cmd = Command::new("mtr");

    // apply platform-specific flags depending on the target os
    #[cfg(target_os = "linux")]
    {
        // use report, wide, as-lookup, and 10-cycle count flags for linux
        mtr_cmd.args(["-rwzc", "10", &host.ip_addr]);
    }

    #[cfg(target_os = "macos")]
    {
        // use safe report and 10-cycle count flags for macos to avoid missing libraries
        mtr_cmd.args(["-rc", "10", &host.ip_addr]);
    }

    // execute the diagnostic command
    mtr_cmd.status().await?;

    Ok(())
}

/// Installs a missing system package using the host platform package manager
async fn install_dependency(package: &str) -> Result<()> {
    // handle dependency provisioning for linux environments
    #[cfg(target_os = "linux")]
    {
        println!(
            ":: Local dependency '{}' absent. Resolving via pacman...",
            package
        );
        Command::new("sudo")
            .args(["pacman", "-S", "--noconfirm", package])
            .status()
            .await?;
    }

    // handle dependency provisioning for macos environments
    #[cfg(target_os = "macos")]
    {
        println!(
            ":: Local dependency '{}' absent. Resolving via Homebrew...",
            package
        );

        // ensure homebrew is installed on the host macos system
        if Command::new("brew")
            .arg("--version")
            .output()
            .await
            .is_err()
        {
            return Err(Error::Operational(
                "Homebrew is required to install missing dependencies on macOS. Please install it first.",
            ).into());
        }

        // fetch package through homebrew formulae
        Command::new("brew")
            .args(["install", package])
            .status()
            .await?;
    }

    Ok(())
}
