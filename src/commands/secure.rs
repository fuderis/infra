use crate::prelude::*;

use tokio::process::Command;

/// Inspects security state, firewall rules and SSH activity on the remote server.
pub async fn handle_secure(target: &Option<String>, ip: &Option<String>) -> Result<()> {
    let conn = super::get_ssh_conn(target, ip)?;

    let script = r#"
section() {
    echo
    printf "\033[1;36m▶ %s\033[0m\n\n" "$1"
}

###############################################################################
section "Firewall"

sudo ufw status numbered 2>/dev/null || echo "UFW is disabled"

###############################################################################
section "Listening Ports"

(
    echo "PORT|PROCESS"
    echo "------|---------------------------"

    ss -ltnpH 2>/dev/null | awk '
    {
        split($4,a,":")
        port=a[length(a)]

        proc="-"
        if(match($NF,/"[^"]+"/))
            proc=substr($NF,RSTART+1,RLENGTH-2)

        print port "|" proc
    }' | sort -n -u
) | column -t -s '|'

###############################################################################
section "Fail2Ban"

if systemctl is-active --quiet fail2ban; then
    echo "Status : active"

    fail2ban-client status sshd 2>/dev/null | awk -F: '
        /Currently banned/ {gsub(/^[ \t]+/,"",$2); print "Currently banned : " $2}
        /Total banned/     {gsub(/^[ \t]+/,"",$2); print "Total banned     : " $2}
    '

    echo
    echo "Recent bans"
    echo

    (
        echo "TIME|JAIL|IP"
        echo "-----------------------|------|----------------"

        grep " Ban " /var/log/fail2ban.log 2>/dev/null \
            | tail -20 \
            | awk '{
                gsub(/[][]/,"",$6)
                print $1" "$2 "|" $6 "|" $NF
            }'
    ) | column -t -s '|'

else
    echo "Fail2Ban is not running"
fi

###############################################################################
section "Failed SSH Authentication"

LOGCMD="journalctl -u ssh -u sshd --since \"24 hours ago\" --no-pager -o cat"

(
    echo "USER|IP|TYPE"
    echo "------------|---------------|--------------"

    eval "$LOGCMD" 2>/dev/null | awk '
    /Failed password/ {

        user="-"
        ip="-"

        for(i=1;i<=NF;i++) {
            if($i=="for") {
                if($(i+1)=="invalid")
                    user=$(i+3)
                else
                    user=$(i+2)
            }

            if($i=="from")
                ip=$(i+1)
        }

        print user "|" ip "|password"
    }

    /Invalid user/ {

        user="-"
        ip="-"

        for(i=1;i<=NF;i++) {

            if($i=="user")
                user=$(i+1)

            if($i=="from")
                ip=$(i+1)
        }

        print user "|" ip "|invalid-user"
    }
    ' | tail -20

) | column -t -s '|'

###############################################################################
section "Recent SSH Logins"

(
    echo "USER|FROM|LOGIN"
    echo "----------|----------------|----------------"

    last -w -i 2>/dev/null \
        | grep -v "^reboot" \
        | grep -v "^wtmp" \
        | head -10 \
        | awk '{
            printf "%s|%s|%s %s %s\n",$1,$3,$4,$5,$6
        }'

) | column -t -s '|'
"#;

    Command::new("ssh")
        .args(&conn.args)
        .args([&conn.target, script])
        .status()
        .await?;

    Ok(())
}
