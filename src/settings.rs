use crate::{RemoteHost, SyncConfig, prelude::*};

/// The settings instance
static SETTINGS: State<Config<Settings>> = State::default();

/// The remote hosts options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteOptions {
    pub default: String,
    pub hosts: HashMap<String, RemoteHost>,
}

impl ::std::default::Default for RemoteOptions {
    fn default() -> Self {
        Self {
            default: str!("main"),
            hosts: map! {
                str!("main") => RemoteHost::new("root", "127.0.0.1", None),
                str!("admin") => RemoteHost::new("admin", "127.0.0.1", Some(path!("~/.ssh/infra-admin")))
            },
        }
    }
}

/// The synchronization options
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncOptions {
    pub configs: HashMap<String, SyncConfig>,
}

impl ::std::default::Default for SyncOptions {
    fn default() -> Self {
        Self {
            configs: map! {
                str!("helix") => SyncConfig::new(vec![path!("$config/helix/config.toml")]),
                str!("nvim") => SyncConfig::new(vec![path!("$config/nvim/init.lua")]),
            },
        }
    }
}

/// The settings
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub remote: RemoteOptions,
    pub sync: SyncOptions,
}

impl Settings {
    /// Reads & initializes the settings
    pub async fn init<P>(file_path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let conf = Config::<Settings>::new(file_path.as_ref()).await?;
        SETTINGS.set(conf).await;
        Ok(())
    }

    /// Returns settings file path
    pub fn path() -> PathBuf {
        SETTINGS.dirty_get().path().clone()
    }

    /// Returns global settings instance
    pub fn get() -> Arc<Config<Settings>> {
        SETTINGS.dirty_get()
    }

    /// Returns settings state guard
    pub async fn lock() -> StateGuard<Config<Settings>> {
        SETTINGS.lock().await
    }

    /// Returns actual settings file data
    pub async fn read() -> Result<Config<Settings>> {
        let path = SETTINGS.dirty_get().path().clone();
        Config::<Settings>::read(path).await
    }

    /// Reads actual settings from file
    pub async fn update() -> Result<bool> {
        let mut cfg = SETTINGS.lock().await;

        if cfg.check(0).await? {
            cfg.update().await
        } else {
            Ok(false)
        }
    }
}
