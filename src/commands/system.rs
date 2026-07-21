use super::{info, section};
use crate::prelude::*;

use std::process::Stdio;
use tokio::{io::AsyncWriteExt, process::Command};

/// Fetches and displays remote server resource usage metrics
pub async fn handle_usage(target: &Option<String>, ip: &Option<String>) -> Result<()> {
    let conn = super::get_ssh_conn(target, ip)?;

    section("Resource Usage");

    println!();
    let script = r#"
echo "LOAD"
cat /proc/loadavg

echo
echo "CPU"
nproc

echo
echo "MEMORY"
free -h

echo
echo "STORAGE"
df -h /

echo
echo "SWAP"
swapon --show
"#;

    let output = Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, script])
        .output()
        .await?;

    println!("{}", String::from_utf8_lossy(&output.stdout));

    Ok(())
}

/// Provisions and hardens the remote server infrastructure
pub async fn handle_setup(target: &Option<String>, ip: &Option<String>) -> Result<()> {
    let conn = super::get_ssh_conn(target, ip)?;

    section("Server Setup");
    info("", "Exporting SSH identity");

    let mut copy_id_cmd = Command::new("ssh-copy-id");

    if !conn.args.is_empty() {
        copy_id_cmd.args(&conn.args);
    }

    copy_id_cmd.arg(&conn.target).status().await?;

    info("", "Starting remote provisioning");

    let setup_script = r#"
set -e


echo "[1/7] Updating packages"
sudo apt update
sudo apt upgrade -y


echo "[2/7] Installing base packages"
sudo apt install -y \
    software-properties-common \
    curl \
    git \
    ufw \
    fail2ban \
    htop \
    unzip \
    at


echo "[3/7] Enabling services"

sudo systemctl enable --now atd


echo "[4/7] Installing Helix editor"

sudo add-apt-repository -y \
    ppa:maveonair/helix-editor

sudo apt update
sudo apt install -y helix


echo "[5/7] Configuring Fail2Ban"

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


echo "[6/7] Configuring firewall"

sudo ufw default deny incoming
sudo ufw default allow outgoing

sudo ufw allow 22/tcp
sudo ufw allow 2222/tcp
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

sudo ufw --force enable


echo "[7/7] Hardening SSH"


sudo sshd -t


echo "sudo systemctl restart ssh" |
sudo at now + 1 minute


sudo sed -i \
's/^#\?PasswordAuthentication.*/PasswordAuthentication no/' \
/etc/ssh/sshd_config


sudo sed -i \
's/^#\?PermitRootLogin.*/PermitRootLogin prohibit-password/' \
/etc/ssh/sshd_config


sudo sshd -t

sudo systemctl restart ssh


echo "DONE"
"#;

    let mut child = Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, "bash", "-s"])
        .stdin(Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(setup_script.as_bytes()).await?;
    }

    let status = child.wait().await?;

    if !status.success() {
        return Err(Error::Operational("Remote setup failed".into()).into());
    }

    info("Deployed", &conn.target);

    Ok(())
}
