pub mod error;
pub mod prelude;
pub mod settings;

pub mod share;
pub use share::{RemoteHost, SyncConfig};

pub mod commands;

use clap::{Parser, Subcommand};
use prelude::*;

pub const APP_NAME: &str = "infra";
pub const APP_VERSION: &str = "0.3.0";

#[derive(Parser, Debug)]
#[command(
    name = APP_NAME,
    version = APP_VERSION,
    about = "Remote Infrastructure Orchestrator"
)]
pub struct Cli {
    /// Target remote host name
    #[arg(short, long, global = true)]
    pub target: Option<String>,

    /// Explicitly specify the target IP address (bypasses config check)
    #[arg(long, global = true)]
    pub ip: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    //         SSH
    /// Listing all available hosts
    List,
    /// SSH connection to the host
    Connect,
    /// SOCKS5 SSH Tunnel Management
    Tunnel {
        #[command(subcommand)]
        action: TunnelAction,

        /// Specifying the port for the SSH tunnel (1080 by default)
        #[arg(short, long, global = false)]
        port: Option<u16>,
    },

    //         NETWORK
    /// Quick ICMP ping check to remote server
    Ping {
        /// Number of packets to send
        #[arg(short, long, default_value = "10")]
        count: usize,
    },
    /// Route tracing (traceroute)
    Trace,
    /// Continuous Route Quality Audit (mtr)
    Route,

    //         SYSTEM
    /// Initial secure server setup (apt, ssh hardening, fail2ban)
    Setup,
    /// Monitoring of system resources (LoadAvg, CPU, RAM, Disk, Swap)
    Usage,

    //         SECURITY
    /// Security check (firewall, fail2ban, active sockets, sessions)
    Secure,

    //         USERS
    /// User management on a remote server
    User {
        /// Target username (required for 'user' command)
        #[arg(short, long, global = true)]
        username: Option<String>,

        #[command(subcommand)]
        action: UserAction,
    },

    //         FILE MANAGEMENT
    /// Uploads a local file or directory to the remote host (uses rsync)
    Upload {
        /// Local path to file or directory
        local_path: PathBuf,
        /// Remote destination path
        remote_path: String,
    },
    /// Downloads a file or directory from the remote host to the local machine (uses rsync)
    Download {
        /// Remote source path
        remote_path: String,
        /// Local destination path
        local_path: PathBuf,
    },
    /// Synchronize local configurations and dotfiles to the remote host
    Sync { sync_config: String },
}

#[derive(Subcommand, Debug, Clone, Copy)]
pub enum TunnelAction {
    Start {
        /// Allow remote hosts to connect to local forwarded ports
        #[arg(short, long)]
        gateway: bool,
    },
    Stop,
    Restart {
        /// Allow remote hosts to connect to local forwarded ports
        #[arg(short, long)]
        gateway: bool,
    },
    Status,
}

#[derive(Subcommand, Debug)]
pub enum UserAction {
    /// Create a new user
    New,
    /// Adds a user to the sudo group
    GrantSudo,
    /// Removes a user from the sudo group
    RevokeSudo,
    /// Operations with specific user's keys
    Key {
        #[command(subcommand)]
        op: UserKeyOp,
    },
    /// Show user status and login history
    Status,
    /// Delete a user and their home directory
    Remove,
}

#[derive(Subcommand, Debug)]
pub enum UserKeyOp {
    /// Generate and add a key to the user
    Gen { output_file: Option<PathBuf> },
    /// Add an existing public key to the user
    Add { pubkey: String },
    /// Clear the user's authorized_keys
    Clear,
}

#[tokio::main]
async fn main() -> Result<()> {
    use commands as cmds;

    Settings::init(path!("$config$/settings.toml")).await?;

    let cli = Cli::parse();

    let target = &cli.target;
    let ip = &cli.ip;

    if let Err(e) = match cli.command {
        Commands::List => cmds::ssh::handle_list().await,
        Commands::Connect => cmds::ssh::handle_connect(&target, &ip).await,
        Commands::Tunnel { action, port } => {
            cmds::ssh::handle_tunnel(&target, &ip, action, port).await
        }

        Commands::Setup => cmds::system::handle_setup(&target, &ip).await,
        Commands::Usage => cmds::system::handle_usage(&target, &ip).await,

        Commands::Ping { count } => cmds::net::handle_ping(&target, &ip, count).await,
        Commands::Trace => cmds::net::handle_trace(&target, &ip).await,
        Commands::Route => cmds::net::handle_route(&target, &ip).await,

        Commands::Secure => cmds::secure::handle_secure(&target, &ip).await,

        Commands::User { action, username } => {
            if let Some(username) = username {
                cmds::user::handle_user(&target, &ip, username, action).await
            } else {
                Err(Error::Operational(str!("Expected --username argument")).into())
            }
        }

        Commands::Upload {
            local_path,
            remote_path,
        } => cmds::sync::handle_upload(&target, &ip, &local_path, &remote_path).await,
        Commands::Download {
            remote_path,
            local_path,
        } => cmds::sync::handle_download(&target, &ip, &remote_path, &local_path).await,
        Commands::Sync { sync_config } => cmds::sync::handle_sync(&target, &ip, &sync_config).await,
    } {
        println!("{} {e}", "Error:".red());
    }

    Ok(())
}
