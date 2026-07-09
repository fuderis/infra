pub mod error;
pub mod prelude;
pub mod settings;

pub mod share;
pub use share::{RemoteHost, SyncConfig};

pub mod cmds;

use clap::{Parser, Subcommand};
use prelude::*;

#[derive(Parser, Debug)]
#[command(
    name = "infra",
    version = "0.1.3",
    about = "Remote Infrastructure Orchestrator"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Target remote host name
    #[arg(short, long, global = true)]
    pub target: Option<String>,
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

        /// Allow remote hosts to connect to local forwarded ports
        #[arg(short, long, global = true)]
        gateway: bool,
    },

    //         NETWORK
    /// Quick ICMP ping check to remote server
    Ping,
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
        #[command(subcommand)]
        action: UserAction,

        /// Target username (required for 'user' command)
        #[arg(short, long, global = true)]
        username: String,
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
    Start,
    Stop,
    Restart,
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
    Settings::init(path!("~/.config/infra/settings.toml")).await?;

    let cli = Cli::parse();

    if let Err(e) = match cli.command {
        Commands::List => cmds::ssh::handle_list().await,
        Commands::Connect => cmds::ssh::handle_connect(&cli.target).await,
        Commands::Tunnel {
            action,
            port,
            gateway,
        } => cmds::ssh::handle_tunnel(&cli.target, action, gateway, port).await,

        Commands::Setup => cmds::system::handle_setup(&cli.target).await,
        Commands::Usage => cmds::system::handle_usage(&cli.target).await,

        Commands::Ping => cmds::net::handle_ping(&cli.target).await,
        Commands::Trace => cmds::net::handle_trace(&cli.target).await,
        Commands::Route => cmds::net::handle_route(&cli.target).await,

        Commands::Secure => cmds::secure::handle_secure(&cli.target).await,

        Commands::User { action, username } => {
            cmds::user::handle_user(&cli.target, username, action).await
        }

        Commands::Upload {
            local_path,
            remote_path,
        } => cmds::sync::handle_upload(&cli.target, &local_path, &remote_path).await,
        Commands::Download {
            remote_path,
            local_path,
        } => cmds::sync::handle_download(&cli.target, &remote_path, &local_path).await,
        Commands::Sync { sync_config } => cmds::sync::handle_sync(&cli.target, &sync_config).await,
    } {
        println!("{} {e}", cmds::err());
    }

    Ok(())
}
