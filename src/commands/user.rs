use super::{SshConnection, info, section};
use crate::{UserAction, UserKeyOp, prelude::*};
use std::fs;
use tokio::process::Command;

/// Dispatches account management routines based on the specified user action
pub async fn handle_user(
    target: &Option<String>,
    ip: &Option<String>,
    username: String,
    action: UserAction,
) -> Result<()> {
    let conn = super::get_ssh_conn(&target, ip)?;

    match action {
        UserAction::New => handle_new(&conn, username).await?,
        UserAction::GrantSudo => handle_grant_sudo(&conn, username).await?,
        UserAction::RevokeSudo => handle_revoke_sudo(&conn, username).await?,
        UserAction::Status => handle_status(&conn, username).await?,
        UserAction::Remove => handle_remove(&conn, username).await?,
        UserAction::Key { op } => handle_key_operations(&conn, username, op).await?,
    }

    Ok(())
}

/// Creates a new unprivileged system user with an initialized ssh directory
async fn handle_new(conn: &SshConnection, username: String) -> Result<()> {
    section("Create User");
    info(&format!("User : {}", username));

    let script = format!(
        r#"
if sudo id {0} >/dev/null 2>&1; then
    echo "User already exists"
    exit 0
fi

sudo useradd -m -s /bin/bash {0}

sudo mkdir -p /home/{0}/.ssh
sudo chown -R {0}:{0} /home/{0}/.ssh
sudo chmod 700 /home/{0}/.ssh
"#,
        username
    );

    Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    info("User created");

    Ok(())
}

/// Appends the targeted user account to the secondary administrative sudo group
async fn handle_grant_sudo(conn: &SshConnection, username: String) -> Result<()> {
    section("Grant Sudo");
    info(&format!("User : {}", username));

    let script = format!(
        r#"
if ! sudo id {0} >/dev/null 2>&1; then
    echo "User does not exist"
    exit 1
fi

sudo usermod -aG sudo {0}
"#,
        username
    );

    let status = Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    if status.success() {
        info("Sudo privileges granted");
        Ok(())
    } else {
        Err(Error::Operational("Failed to grant sudo privileges".into()).into())
    }
}

/// Removes the targeted user account from the secondary administrative sudo group
async fn handle_revoke_sudo(conn: &SshConnection, username: String) -> Result<()> {
    section("Revoke Sudo");

    info(&format!("User : {}", username));

    let script = format!(
        r#"
if ! sudo id {0} >/dev/null 2>&1; then
    echo "User does not exist"
    exit 1
fi

sudo deluser {0} sudo
"#,
        username
    );

    let status = Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    if status.success() {
        info("Sudo privileges revoked");

        Ok(())
    } else {
        Err(Error::Operational("Failed to revoke sudo privileges".into()).into())
    }
}

/// Audits detailed structural and session metrics for a single host account
async fn handle_status(conn: &SshConnection, username: String) -> Result<()> {
    section("User Status");

    let script = format!(
        r#"
if ! id {0} >/dev/null 2>&1; then
    echo "ERROR|User does not exist"
    exit 1
fi

echo "USER|{0}"
echo "UID|$(id -u {0})"
echo "HOME|$(eval echo ~{0})"
echo "SHELL|$(getent passwd {0} | cut -d: -f7)"
echo "GROUPS|$(groups {0} | cut -d: -f2- | xargs)"

if who | grep -q "^{0}"; then
    echo "SESSION|ONLINE"
else
    echo "SESSION|OFFLINE"
fi
"#,
        username
    );

    let output = Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("{:<12} {:<40}", "FIELD", "VALUE");
    println!(
        "{:<12} {:<40}",
        "------------", "----------------------------------------"
    );

    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once('|') {
            println!("{:<12} {}", key, value);
        }
    }

    Ok(())
}

/// Destroys a user identity along with all attached files and home folders
async fn handle_remove(conn: &SshConnection, username: String) -> Result<()> {
    section("Remove User");
    info(&format!("Removing : {}", username));

    let script = format!("sudo deluser --remove-home {}", username);

    Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    info("User removed");

    Ok(())
}

/// Routes specific cryptographic key mutations.
async fn handle_key_operations(
    conn: &SshConnection,
    username: String,
    op: UserKeyOp,
) -> Result<()> {
    match op {
        UserKeyOp::Add { pubkey } => handle_key_add(conn, username, pubkey).await?,
        UserKeyOp::Clear => handle_key_clear(conn, username).await?,
        UserKeyOp::Gen { output_file } => handle_key_gen(conn, username, output_file).await?,
    }
    Ok(())
}

/// Manually appends a structured public signature string to authorized keys
async fn handle_key_add(conn: &SshConnection, username: String, pubkey: String) -> Result<()> {
    section("SSH Key");

    info(&format!("Action : Add public key"));

    info(&format!("User   : {}", username));

    let script = format!(
        r#"
sudo mkdir -p /home/{0}/.ssh

echo '{1}' |
sudo tee -a /home/{0}/.ssh/authorized_keys >/dev/null

sudo chown -R {0}:{0} /home/{0}/.ssh
sudo chmod 700 /home/{0}/.ssh
sudo chmod 600 /home/{0}/.ssh/authorized_keys
"#,
        username, pubkey
    );

    Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    info("Public key added");

    Ok(())
}

/// Flushes the verification database file by truncating all payload contents
async fn handle_key_clear(conn: &SshConnection, username: String) -> Result<()> {
    section("SSH Key");

    info("Action : Clear authorized keys");

    info(&format!("User   : {}", username));

    let script = format!(
        r#"
if [[ -f /home/{0}/.ssh/authorized_keys ]]; then
    sudo truncate -s 0 /home/{0}/.ssh/authorized_keys
fi
"#,
        username
    );

    Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    info("Authorized keys cleared");

    Ok(())
}

/// Generates an asymmetric keypair locally and mounts the public node remotely
async fn handle_key_gen(
    conn: &SshConnection,
    username: String,
    output_file: Option<PathBuf>,
) -> Result<()> {
    section("Generate SSH Key");

    let target_key_path = match output_file {
        Some(path) => path,

        None => {
            let local_home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or_else(|| Error::Operational("Cannot resolve HOME directory".into()))?;

            local_home.join(".ssh").join(format!("infra-{}", username))
        }
    };

    info(&format!("User   : {}", username));

    info(&format!("Output : {}", target_key_path.display()));

    if target_key_path.exists() {
        return Err(Error::Operational(format!(
            "Key already exists: {}",
            target_key_path.display()
        ))
        .into());
    }

    let tmp_dir = format!("/tmp/ssh-gen.{}", std::process::id());

    fs::create_dir_all(&tmp_dir)?;

    let key_path = Path::new(&tmp_dir).join("key");

    let status = Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-q", "-f"])
        .arg(&key_path)
        .status()
        .await?;

    if !status.success() || !key_path.exists() {
        let _ = fs::remove_dir_all(&tmp_dir);

        return Err(Error::Operational("Key generation failed".into()).into());
    }

    let pub_key = fs::read_to_string(Path::new(&tmp_dir).join("key.pub"))?
        .trim()
        .to_string();

    let priv_key = fs::read_to_string(&key_path)?;

    fs::remove_dir_all(&tmp_dir)?;

    info("Uploading public key");

    let script = format!(
        r#"
sudo mkdir -p /home/{0}/.ssh

echo '{1}' |
sudo tee -a /home/{0}/.ssh/authorized_keys >/dev/null

sudo chown -R {0}:{0} /home/{0}/.ssh
sudo chmod 700 /home/{0}/.ssh
sudo chmod 600 /home/{0}/.ssh/authorized_keys
"#,
        username, pub_key
    );

    let upload = Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    if upload.success() {
        fs::write(&target_key_path, priv_key)?;

        Command::new("chmod")
            .args(["600"])
            .arg(&target_key_path)
            .status()
            .await?;

        info("SSH key generated");

        Ok(())
    } else {
        Err(Error::Operational("Remote key installation failed".into()).into())
    }
}
