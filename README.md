# Infra: Remote Infrastructure Orchestrator 

Infra is a lightweight, compiled CLI tool written in Rust for remote server administration, operational monitoring, and baseline security auditing.
It provides a structured, predictable alternative to complex shell scripts by handling low-level process spawning and connection state validation natively.

> ⚠️ **Important Notice:** This tool is strictly optimized and oriented toward managing **Debian** and **Ubuntu** server distributions
(it relies on core system packages such as `apt`, `ufw`, and `fail2ban`).

## Key Features

* **Concurrent Process Execution:** Executes remote commands, network audits, and file transfers concurrently without blocking the local user interface.
* **Deterministic Configuration Syncing:** Automatically strips local `$HOME` path prefixes, ensures idempotent remote directory structure generation (`mkdir -p`) via SSH,
  and streams updates using `rsync` over SSH.
* **Out-of-the-Box Security:** Instant network socket inventory, active firewall inspection, brute-force detection,
  and automated baseline hardening (SSH & Fail2Ban configuration).
* **Stateful SSH Tunneling:** Manages persistent SOCKS5 proxy daemons inside isolated process groups (`setsid`).
  Includes runtime checks (`fuser` and process mapping) to prevent zombie processes, PID collisions, and port binding failures.
* **Automated User & Key Lifecycle:** Standardizes remote unprivileged account provisioning, cryptographic `ed25519` keypair generation using temporary memory-mapped paths (`/tmp`),
  remote authorization injection, and precise permission enforcement (`chmod 700/600`).
* **Passive Host Inspections:** Aggregates remote system statistics by parsing low-overhead runtime counters, `ss` network socket tables, active firewall states,
  and active interactive terminal sessions.

## Command Architecture & Usage

The global flag `-t` | `--target` is used to specify the target host defined in your configuration file.

```bash
infra [OPTIONS] <COMMAND>
```

### 1. SSH Module (`cmds::ssh`)
Handles interactive login sessions and background network proxy streams.

* `infra list` — Lists all infrastructure hosts configured in `settings.toml`.
* `infra [-t TARGET] connect` — Spawns a native, interactive SSH terminal session to the target host.
* `infra [-t TARGET] tunnel [-p PORT] <ACTION>` — Manages a persistent background SOCKS5 SSH tunnel bound to local port (`1080` by default).
  * `start` — Launches the background monitoring watchdog within an isolated process group (`setsid`).
  * `stop` — Safely tears down the entire process group session and flushes the PID lockfile.
  * `restart` — Cycles the active network proxy daemon offline and online.
  * `status` — Inspects the process tree for running `bash`/`ssh` tunnel instances.

 > The flag `-g` | `--gateway` allows you to start a tunnel for all devices on the local network.

### 2. NETWORK Module (`cmds::net`)
Performs rapid network health checks and path discovery to the remote machine.

* `infra [-t TARGET] ping` — Triggers a rapid ICMP ping check to the remote server.
* `infra [-t TARGET] trace` — Traces the network packet route to the target (`traceroute`).
* `infra [-t TARGET] route` — Runs a continuous, real-time route quality audit using `mtr`.

### 3. SYSTEM & SECURITY Modules (`cmds::system`, `cmds::secure`)
Automates host configurations and evaluates threat posture (Debian/Ubuntu specific).

* `infra [-t TARGET] setup` — Performs initial, secure server hardening: triggers `apt` updates, installs and configures `fail2ban`,
  and disables remote password-based SSH access.
* `infra [-t TARGET] usage` — Streams real-time host hardware utilization metrics (LoadAvg, CPU, RAM, Disk, Swap).
* `infra [-t TARGET] secure` — Aggregates a security audit: polls UFW rules, Fail2Ban agent states, jail map metrics,
  open socket tables (`ss`), and current active user login records.

### 4. USERS Module (`cmds::user`)
Provides an end-to-end suite for remote user account lifecycle mutations.

```bash
infra [-t TARGET] user <-u USER_NAME> <ACTION>
```

User Actions: 
* `new` — Provisions a new unprivileged system user with a default `/bin/bash` shell environment and strictly isolated permissions.
* `grant-sudo` — Appends the targeted account to the secondary administrative `sudo` group.
* `status` — Audits structural configuration matrices and maps interactive login history for a specific user.
* `remove` — Destroys a user identity along with all attached files and home folders.
* `key <KEY_OP>` — Routes specific cryptographic key mutations:
  * `user gen [--output-file <PATH>]` — Allocates an isolated staging directory in local RAM,
    triggers an `ed25519` generation sequence, mounts the public node remotely, and secures the private key locally.
  * `user add --pubkey "<KEY>"` — Appends a raw structured public verification vector directly to the user's `authorized_keys`.
  * `user clear` — Flushes the verification database by truncating all authorized key signatures for the user.

### 5. FILE MANAGEMENT & SYNC Module (`cmds::sync`)
Handles high-performance file transfers and declarative environment synchronization using `rsync` over SSH.

* `infra [-t TARGET] send <LOCAL_PATH> <REMOTE_PATH>` — Transports a specific local file or directory to the remote host.
  * Automatically injects custom SSH arguments (ports, identity keys) from your host configuration.
  * Uses archive mode (`-azh`) with real-time compression and non-interactive stream progress tracking.
* `infra [-t TARGET] sync <CONFIG_NAME>` — Synchronizes local configuration profiles (dotfiles) defined in `settings.toml` to the remote host.
  * **Smart Path Resolution:** If `<CONFIG_NAME>` is specified as `@`, the utility aggregates and pushes **all** defined configuration profiles sequentially.
  * **Cross-User Home Normalization:** Automatically strips the local `$HOME` prefix from paths.
    If you are syncing a file from `/home/local_user/.config/helix` to a server where you log in as `ubuntu`,
    the file will be correctly mirrored into `/home/ubuntu/.config/helix`.
  * **Idempotent Directory Provisioning:** Prior to file transfer, executes an isolated, aggregated `mkdir -p` call over SSH to pre-generate deep remote directory structures,
    avoiding `rsync` target omission errors.

#### Sync Example
To synchronize only your editor configuration or backup everything:
```bash
# Sync only Helix editor configs
infra -t admin sync helix

# Sync absolutely all configuration blocks defined in settings.toml
infra -t admin sync @
```

## Infra Configuration

On initialization, the utility looks for its configuration blueprint at the following path:
`~/.config/infra/settings.toml`

Configuration profile by default (change to your own):

```toml
[remote]
default = "root"

[remote.hosts.root]
user_name = "root"
ip_addr = "127.0.0.1"

[remote.hosts.admin]
user_name = "admin"
ip_addr = "127.0.0.1"
ssh_file = "/home/<user-name>/.ssh/infra-admin"

[sync.configs.helix]
files = ["/home/<user-name>/.config/helix/config.toml"]

[sync.configs.nvim]
files = ["/home/<user-name>/.config/nvim/init.lua"]
```

## Installation Guide

To build the project executable from source, ensure you have the standard Rust toolchain (`cargo`) installed.

1. Clone the repository from GitHub
```bash
git clone https://github.com/fuderis/infra.git
cd infra
```

2. Run installation script
```bash
bash build.sh
```

3. Now you can delete the source code
```bash
rm -rf infra
```
 
## License & Feedback

> This software distributed under the [GPL 3.0](https://github.com/fuderis/infra/blob/main/LICENSE.md) license.

You can contact me via [GitHub](https://github.com/fuderis) or send a message to my [E-Mail](mailto:synapdrake@ya.ru).
This library is actively evolving, and your suggestions and feedback are always welcome!
