use crate::prelude::*;
use tokio::process::Command;

/// Inspects security state, firewall rules, and authentication logs on the remote server.
pub async fn handle_secure(target: &Option<String>) -> Result<()> {
    // resolve ssh connection details for the target host
    let conn = super::get_ssh_conn(target)?;

    // inline bash script to collect security metrics and log data
    let script = r#"
echo "==================== FIREWALL RULES (UFW) ====================" && sudo ufw status verbose 2>/dev/null || echo "UFW engine disabled/not-found"
echo
echo "==================== SOCKET INVENTORY (ss) ====================" && ss -tunap | head -50
echo
echo "==================== FAIL2BAN AGENT STATE ====================" && systemctl status fail2ban --no-pager | head -10 || true
echo
echo "==================== BAN LIST PER JAILS ====================" && sudo fail2ban-client status sshd 2>/dev/null || true
echo
echo "==================== RECORDED AUTHENTICATION FAILURES ====================" && sudo grep "Failed password" /var/log/auth.log 2>/dev/null | tail -15 || true
echo
echo "==================== MALICIOUS NON-EXISTENT USERS ====================" && sudo grep "invalid user" /var/log/auth.log 2>/dev/null | tail -15 || true
echo
echo "==================== ESTABLISHED OPERATIONAL SESSIONS ====================" && last -a | head -15
"#;

    // execute the security audit script via ssh
    Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, script])
        .status()
        .await?;

    Ok(())
}
