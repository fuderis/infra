use super::SshConnection;
use crate::{UserAction, UserKeyOp, prelude::*};
use std::fs;
use tokio::process::Command;

/// Dispatches account management routines based on the specified user action
pub async fn handle_user(
    target: &Option<String>,
    username: String,
    action: UserAction,
) -> Result<()> {
    let conn = super::get_ssh_conn(&target)?;

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
    println!(":: Provisioning isolated user workspace: {}...", username);

    // inline bash script to register user and secure home permissions
    let script = format!(
        r#"
if sudo id {0} >/dev/null 2>&1; then echo 'User space already allocated'; exit 0; fi
sudo useradd -m -s /bin/bash {0}
sudo mkdir -p /home/{0}/.ssh
sudo chown -R {0}:{0} /home/{0}/.ssh
sudo chmod 700 /home/{0}/.ssh
"#,
        username
    );

    // run user provisioning script on the remote host
    Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    println!("{} User account injected successfully.", super::ok());
    Ok(())
}

/// Appends the targeted user account to the secondary administrative sudo group
async fn handle_grant_sudo(conn: &SshConnection, username: String) -> Result<()> {
    println!(
        ":: Granting administrative privileges to user: {}...",
        username
    );

    // build target validation and group modification query
    let script = format!(
        r#"
if ! sudo id {0} >/dev/null 2>&1; then echo 'Error: Target account does not exist'; exit 1; fi
sudo usermod -aG sudo {0}
"#,
        username
    );

    // execute group modification command over ssh
    let status = Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    // verify transaction state
    if status.success() {
        println!(
            "{} User {} successfully added to the sudo group.",
            super::ok(),
            username
        );
        Ok(())
    } else {
        Err(Error::Operational("Failed to append user to sudo group on remote host".into()).into())
    }
}

/// Removes the targeted user account from the secondary administrative sudo group
async fn handle_revoke_sudo(conn: &SshConnection, username: String) -> Result<()> {
    println!(
        ":: Revoking administrative privileges from user: {}...",
        username
    );

    // build target validation and group removal query
    let script = format!(
        r#"
if ! sudo id {0} >/dev/null 2>&1; then echo 'Error: Target account does not exist'; exit 1; fi
sudo deluser {0} sudo
"#,
        username
    );

    // execute group modification command over ssh
    let status = Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    // verify transaction state
    if status.success() {
        println!(
            "{} Privileges revoked. User {} removed from the sudo group.",
            super::ok(),
            username
        );
        Ok(())
    } else {
        Err(
            Error::Operational("Failed to remove user from sudo group on remote host".into())
                .into(),
        )
    }
}

/// Audits detailed structural and session metrics for a single host account
async fn handle_status(conn: &SshConnection, username: String) -> Result<()> {
    // gather id, path, terminal, and operational log data
    let script = format!(
        r#"
if ! id {0} >/dev/null 2>&1; then echo 'Error: Target account does not exist'; exit 1; fi
echo '=== RUNTIME IDENTIFIERS ===' && id {0}
echo -e '\n=== HOME TARGET PATH ===' && eval echo ~{0}
echo -e '\n=== SHELL ENVIRONMENT ===' && getent passwd {0} | cut -d: -f7
echo -e '\n=== PERMISSIONS & PRIVILEGES ===' && groups {0}
echo -e '\n=== ACTIVE INTERACTIVE TERMINALS ==='
if who | grep -q "^{0}"; then echo 'ONLINE STATE'; who | grep "^{0}"; else echo 'OFFLINE STATE'; fi
echo -e '\n=== INTERACTIVE SESSION RECORDS ===' && (last -n 5 {0} || true)
"#,
        username
    );

    // execute status lookup script on the remote environment
    Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    Ok(())
}

/// Destroys a user identity along with all attached files and home folders
async fn handle_remove(conn: &SshConnection, username: String) -> Result<()> {
    println!(
        ":: Wiping target environment structures for: {}...",
        username
    );

    // wipe profile configurations completely
    let script = format!("sudo deluser --remove-home {}", username);

    // execute destructive clean routine over ssh
    Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    println!("{} User context destroyed safely.", super::ok());
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
    println!(
        ":: Exporting static verification vector to user {}...",
        username
    );

    // map incoming public string matrix onto local storage files safely
    let script = format!(
        r#"
sudo mkdir -p /home/{0}/.ssh
echo '{1}' | sudo tee -a /home/{0}/.ssh/authorized_keys >/dev/null
sudo chown -R {0}:{0} /home/{0}/.ssh
sudo chmod 700 /home/{0}/.ssh
sudo chmod 600 /home/{0}/.ssh/authorized_keys
"#,
        username, pubkey
    );

    // trigger keys injection sequence over ssh
    Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    println!("{} Verification vector appended.", super::ok());
    Ok(())
}

/// Flushes the verification database file by truncating all payload contents
async fn handle_key_clear(conn: &SshConnection, username: String) -> Result<()> {
    println!(":: Wiping authorized crypt-keys for user {}...", username);

    // safely clear file data boundaries if present without breaking references
    let script = format!(
        "if [[ -f /home/{0}/.ssh/authorized_keys ]]; then sudo truncate -s 0 /home/{0}/.ssh/authorized_keys; fi",
        username
    );

    // fire cleanup commands onto remote endpoint
    Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    println!("{} Access matrices purged.", super::ok());
    Ok(())
}

/// Generates an asymmetric keypair locally and mounts the public node remotely
async fn handle_key_gen(
    conn: &SshConnection,
    username: String,
    output_file: Option<PathBuf>,
) -> Result<()> {
    // defining the final path for the private key
    let target_key_path = match output_file {
        Some(path) => path,
        None => {
            let local_home = std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
                Error::Operational("Could not resolve local $HOME directory".into())
            })?;
            local_home.join(".ssh").join(format!("infra-{}", username))
        }
    };

    // check if such a key already exists, so as not to erase it
    if target_key_path.exists() {
        return Err(Error::Operational(format!(
            "Identity file already exists locally: {}",
            target_key_path.display()
        ))
        .into());
    }

    println!(":: Allocation of isolated staging directory in RAM...");
    let tmp_dir = format!("/tmp/ssh-gen.{}", std::process::id());
    fs::create_dir_all(&tmp_dir)?;

    // trigger system ed25519 payload assembler engine
    let key_path = Path::new(&tmp_dir).join("key");
    let status = Command::new("ssh-keygen")
        .args(["-t", "ed25519", "-N", "", "-q", "-f"])
        .arg(&key_path)
        .status()
        .await?;

    // abort sequence if underlying execution drops out or data vanishes
    if !status.success() || !key_path.exists() {
        let _ = fs::remove_dir_all(&tmp_dir);
        return Err(
            Error::Operational("Local crypto-engine failure during keygen process".into()).into(),
        );
    }

    // load text buffers for transaction mirroring routines
    let pub_path = Path::new(&tmp_dir).join("key.pub");
    let pub_key = fs::read_to_string(&pub_path)?.trim().to_string();
    let priv_key = fs::read_to_string(&key_path)?;

    // tear down local filesystem traces safely
    fs::remove_dir_all(&tmp_dir)?;

    println!(":: Mirroring key verification vector onto target environment...");
    let script = format!(
        r#"
sudo mkdir -p /home/{0}/.ssh
echo '{1}' | sudo tee -a /home/{0}/.ssh/authorized_keys >/dev/null
sudo chown -R {0}:{0} /home/{0}/.ssh
sudo chmod 700 /home/{0}/.ssh
sudo chmod 600 /home/{0}/.ssh/authorized_keys
"#,
        username, pub_key
    );

    // dispatch mirror payloads via ssh stream channel
    let upload_status = Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, &script])
        .status()
        .await?;

    // finalize file state locally on successful upload transactions
    if upload_status.success() {
        fs::write(&target_key_path, priv_key)?;
        Command::new("chmod")
            .args(["600"])
            .arg(&target_key_path)
            .status()
            .await?;
        println!(
            "{} Secret key token securely generated at: {}",
            super::ok(),
            target_key_path.display()
        );
        Ok(())
    } else {
        Err(Error::Operational(
            "Transaction aborted: remote target rejected public mapping.".into(),
        )
        .into())
    }
}
