use std::{
    borrow::Cow,
    fmt::Debug,
    io::Write,
    path::{Path, PathBuf},
    pin::Pin,
    str::FromStr,
    sync::{Arc, Mutex},
    task::Context,
    time::Instant,
};

use config::{Config, FileFormat};
use futures::task::ArcWake;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use tokio::{sync::broadcast, task::JoinHandle};

use tracing::*;

use crate::system::{db::DatabaseConfig, logger::init_logging};

use super::{
    connection::{ConnectionLock, ConnectionType},
    db::{ConfigDatabase, DbPool},
};

pub trait QuitOnError {
    fn quit_on_err(self, quit: &broadcast::Sender<()>) -> Self;
}

impl<S, E> QuitOnError for Result<S, E> {
    fn quit_on_err(self, quit: &broadcast::Sender<()>) -> Self {
        if self.is_err() {
            let _ = quit.send(());
        }
        self
    }
}

#[typetag::serde()]
pub trait SystemPlugin {
    fn name(&self) -> Cow<str> {
        Cow::Borrowed(std::any::type_name::<Self>())
    }

    fn spawn(&self, system: &System) -> Option<JoinHandle<anyhow::Result<()>>>;
}

#[derive(Clone, Debug, StructOpt)]
#[structopt()]
pub struct SystemArgs {
    #[structopt(long, short = "m")]
    /// Override the run mode from the configuration file
    run_mode: Option<RunMode>,

    #[structopt(long, short, default_value = ".")]
    /// Path to the configuration files and every related external file
    root_dir: PathBuf,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum RunMode {
    Foreground,
    Daemon,
}

impl FromStr for RunMode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "foreground" => Ok(RunMode::Foreground),
            "daemon" => Ok(RunMode::Daemon),
            _ => Err("unsupported run-mode, valid values:  Foreground, Daemon"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SettingsConfig {
    pub run_mode: RunMode,
    pub database: ConfigDatabase,
}

impl Default for SettingsConfig {
    fn default() -> Self {
        Self {
            run_mode: RunMode::Foreground,
            database: ConfigDatabase::default(),
        }
    }
}

impl Into<SystemConfig> for SettingsConfig {
    fn into(self) -> SystemConfig {
        let db = self.database;
        let max_connections = db.max_connections;
        let db_type = db.db_type;
        let port = portpicker::pick_unused_port().unwrap_or(db.port as u16) as i16;
        let connection = match db_type.as_str() {
            "External" => ConnectionType::External(db.uri.unwrap()),
            _ => ConnectionType::Embedded {
                root_path: PathBuf::from(db.root_path),
                port,
                username: db.username,
                password: db.password,
                persistent: db.persistent,
                timeout: db.timeout,
                host: db
                    .host
                    .map(Into::into)
                    .unwrap_or_else(|| "https://repo1.maven.org".to_owned()),
            },
        };
        SystemConfig {
            run_mode: self.run_mode,
            database: DatabaseConfig {
                connection,
                max_connections,
            },
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SystemConfig {
    run_mode: RunMode,
    database: DatabaseConfig,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            run_mode: RunMode::Foreground,
            database: DatabaseConfig::new_embedded(5, ConfigDatabase::default()),
            // ],
        }
    }
}

impl SystemConfig {
    fn get_or_create(path: &Path) -> anyhow::Result<Option<Self>> {
        if path.is_file() {
            let config = Config::builder()
                // Add in `./Settings.toml`
                .add_source(config::File::new("settings", FileFormat::Toml))
                // Add in settings from the environment (with a prefix of APP)
                // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
                .add_source(config::Environment::with_prefix("APP"))
                .build()
                .unwrap();
            let s: SettingsConfig = config.try_deserialize().unwrap();
            let system_config = s.into();
            Ok(Some(system_config))
        } else {
            // let mut file = std::fs::File::create(path)?;
            // file.write_all(DEFAULT_CONFIG.as_bytes())?;
            // file.write_all("\n".as_bytes())?;
            // file.flush()?;
            // drop(file);
            Ok(Some(SystemConfig::default()))
        }
    }
}

pub struct System {
    config: SystemConfig,
    pub root_path: PathBuf,
    db_lock: ConnectionLock,
    pub db_pool: DbPool,
    pub running: bool,
    /// These tasks are ones that keep the system running, useful for daemon's, TUI's, network, etc.
    /// These tasks should *ALWAYS* quit when `quit` is broadcast on or the system may not ever die.
    pub system_tasks: Arc<crossbeam::queue::SegQueue<JoinHandle<anyhow::Result<()>>>>,
    pub quit: broadcast::Sender<()>,
    pub msg: broadcast::Sender<String>,
}

impl Debug for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"SYSTEM SERVER
        running: {}
        task-count: {}
        "#,
            self.running,
            self.system_tasks.len()
        )
    }
}

impl System {
    // pub async fn run() -> anyhow::Result<Self> {
    //     Self::run_with_args(SystemArgs::from_args()).await
    // }

    // pub async fn run_with_args(args: SystemArgs) -> anyhow::Result<Self> {
    //     let config_path = args.root_dir.join("settings.toml");
    //     if let Some(mut config) = SystemConfig::get_or_create(&config_path)? {
    //         if let Some(run_mode) = args.run_mode {
    //             config.run_mode = run_mode
    //         }

    //         Self::run_with_config(args.root_dir.clone(), config).await
    //     } else {
    //         println!(
    //   "No configuration found, wrote out new configuration file at: {:?}, please make edits as necessary and launch again",
    //   config_path
    // );
    //         Err(anyhow::anyhow!("Retry with config"))
    //     }
    // }

    pub async fn initialize() -> anyhow::Result<System> {
        // Self::initialize_with_args(SystemArgs::from_args()).await

        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let system =
            Self::initialize_with_config(root_dir.clone(), SystemConfig::default()).await?;
        Ok(system)
    }

    // pub async fn initialize_with_args(args: SystemArgs) -> anyhow::Result<System> {
    //     println!("initialize_with_args => {:#?}", args);
    //     let config_path = args.root_dir.join("settings.toml");
    //     if let Some(mut config) = SystemConfig::get_or_create(&config_path)? {
    //         if let Some(run_mode) = args.run_mode {
    //             config.run_mode = run_mode
    //         }

    //         let system = Self::initialize_with_config(args.root_dir.clone(), config).await?;
    //         Ok(system)
    //     } else {
    //         println!(
    //   "No configuration found, wrote out new configuration file at: {:?}, please make edits as necessary and launch again",
    //   config_path
    // );
    //         Err(anyhow::anyhow!("No configuration file"))
    //     }
    // }

    pub async fn initialize_with_config(
        root_path: PathBuf,
        config: SystemConfig,
    ) -> anyhow::Result<System> {
        info!("Initialized logging system");
        match init_logging(Some(&root_path)) {
            _ => {}
        };
        let (quit, _recv_quit) = broadcast::channel(1);
        let (msg, _recv_msg) = broadcast::channel(10);
        let (db_lock, db_pool) = config.database.create_database_pool().await?;
        let mut system = System {
            root_path,
            config,
            db_lock,
            db_pool,
            system_tasks: Default::default(),
            quit,
            msg,
            running: false,
        };
        system.startup_systems().await?;
        Ok(system)
    }

    pub async fn run_with_config(root_path: PathBuf, config: SystemConfig) -> anyhow::Result<Self> {
        info!("Initialized logging system");
        let mut system = System::initialize_with_config(root_path, config).await?;
        info!(
            "Running system, {} system tasks upon startup",
            system.system_tasks.len()
        );
        system.run_loop().await?;
        info!("System running completed, no system tasks remaining, shutting down database");
        system.cleanup().await?;
        Ok(system)
    }

    pub async fn cleanup(&mut self) -> anyhow::Result<()> {
        self.db_pool.close().await;
        let _ = &self.db_lock.cleanup().await;
        drop(&self.db_pool);
        drop(&self.db_lock);
        info!("Database shut down, exiting");
        Ok(())
    }

    pub async fn startup_systems(&mut self) -> anyhow::Result<()> {
        anyhow::ensure!(self.system_tasks.is_empty(), "systems already exist");
        match self.config.run_mode {
            RunMode::Foreground => {
                if let Some(handle) = crate::system_tasks::daemon::Daemon::new(false).spawn(self) {
                    self.system_tasks.push(handle);
                }
            }
            RunMode::Daemon => {
                if let Some(handle) = crate::system_tasks::daemon::Daemon::new(true).spawn(self) {
                    self.system_tasks.push(handle);
                }
            }
        }
        for plugin in &[&crate::system_tasks::test_run::TestRun::new(true)] {
            if let Some(handle) = plugin.spawn(self) {
                self.system_tasks.push(handle);
            }
        }
        info!("System startup complete");
        Ok(())
    }

    // #[tracing::instrument(name = "System RunLoop", skip(self))]
    // pub async fn run_loop(&mut self) -> anyhow::Result<()> {
    //     info!("Entering the run_loop");
    //     while let Some(task) = self.system_tasks.pop() {
    //         match task.await {
    //             Ok(Ok(())) => (),
    //             Ok(Err(e)) => {
    //                 error!("System Task returned an error result: {}", e);
    //             }
    //             Err(e) => {
    //                 error!("System Task Join Error: {}", e);
    //             }
    //         }
    //     }
    //     Ok(())
    // }

    #[tracing::instrument(name = "System RunLoop", skip(self))]
    pub async fn run_loop(&mut self) -> anyhow::Result<()> {
        info!("Entering the run_loop");
        while let Some(task) = self.system_tasks.pop() {
            tokio::spawn(async move {
                match task.await {
                    Ok(Ok(())) => (),
                    Ok(Err(e)) => {
                        error!("System Task returned an error result: {}", e);
                    }
                    Err(e) => {
                        error!("System Task Join Error: {}", e);
                    }
                }
            });
        }
        Ok(())
    }

    pub fn as_uri(&self, dbname: String) -> anyhow::Result<String> {
        Ok(self.db_lock.full_db_uri(dbname.as_str()))
    }
}
