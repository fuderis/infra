use crate::prelude::*;
use std::process::Stdio;
use tokio::{io::AsyncWriteExt, process::Command};

/// Fetches and displays remote server resource usage metrics.
pub async fn handle_usage(target: &Option<String>) -> Result<()> {
    // resolve ssh connection details for the target host
    let conn = super::get_ssh_conn(target)?;

    // inline bash script to gather system metrics
    let script = r#"
echo "==================== LOAD AVERAGE ====================" && cat /proc/loadavg
echo
echo "==================== CPU CORES ====================" && nproc
echo
echo "==================== MEMORY METRICS ====================" && free -h
echo
echo "==================== FILE STORAGE STATUS ====================" && df -h /
echo
echo "==================== SWAP VOLUME ====================" && swapon --show
"#;

    // execute the metrics script via ssh
    Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, script])
        .status()
        .await?;

    Ok(())
}

/// Provisions and hardens the remote server infrastructure.
pub async fn handle_setup(target: &Option<String>) -> Result<()> {
    // resolve ssh connection details for the target host
    let conn = super::get_ssh_conn(target)?;

    println!("{} EXPORTING CURRENT SSH IDENTITY", super::block());

    // initialize ssh-copy-id command to transfer public keys
    let mut copy_id_cmd = Command::new("ssh-copy-id");
    if !conn.args.is_empty() {
        copy_id_cmd.args(&conn.args);
    }
    copy_id_cmd.arg(&conn.target).status().await?;

    println!(
        "{} REMOTELY COMPILING INFRASTRUCTURE CONFIGURATION",
        super::block()
    );

    // main provisioning script containing package setup, firewall, and hardening
    let setup_script = r#"
set -e

echo "::: Updating package registries & modernizing kernel trees..."
sudo apt update && sudo apt upgrade -y

echo "::: Provisioning fundamental runtime packages..."
sudo apt install -y software-properties-common curl git ufw fail2ban htop unzip at

echo "::: Stabilizing atd job queue scheduler..."
sudo systemctl enable --now atd

echo "::: Preparing Helix text editor repository PPA..."
sudo add-apt-repository -y ppa:maveonair/helix-editor
sudo apt update && sudo apt install -y helix

echo "::: Injecting hardened profiles into Fail2Ban jails..."
sudo systemctl enable fail2ban
sudo bash -c 'cat > /etc/fail2ban/jail.local <<F2B
[sshd]
enabled = true
port = 22,2222,443
maxretry = 1
findtime = 1d
bantime = -1
F2B'
sudo systemctl restart fail2ban

echo "::: Configuring UFW (Uncomplicated Firewall)..."
# set default firewall policies
sudo ufw default deny incoming
sudo ufw default allow outgoing

# critical: allow ssh access before enabling firewall to avoid lockout
sudo ufw allow 22/tcp
sudo ufw allow 2222/tcp

# optional: open standard web ports
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# enable firewall non-interactively
sudo ufw --force enable

echo "::: Hardening SSH configuration using the safety at-rollback-timer..."
sudo sshd -t
echo "sudo systemctl restart ssh" | sudo at now + 1 minute
sudo sed -i 's/^#\?PasswordAuthentication.*/PasswordAuthentication no/' /etc/ssh/sshd_config
sudo sed -i 's/^#\?PermitRootLogin.*/PermitRootLogin prohibit-password/' /etc/ssh/sshd_config
sudo sshd -t
sudo systemctl restart ssh

echo "::: Host bootstrap provisioning complete! :::"
"#;

    // spawn remote bash process expecting script via stdin
    let mut child = Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, "bash", "-s"])
        .stdin(Stdio::piped())
        .spawn()?;

    // pipe the setup script into the remote shell session
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(setup_script.as_bytes()).await?;
    }

    // wait for the setup script to complete execution
    let status = child.wait().await?;

    // handle unexpected remote execution failures
    if !status.success() {
        return Err(Error::Operational("Remote setup script execution failed".into()).into());
    }

    println!(
        "{} Infrastructure context deployed on {}.",
        super::ok(),
        conn.target
    );
    Ok(())
}
