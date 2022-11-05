use std::{fs, path::PathBuf, time::Duration};

use anyhow::bail;
use pg_embed::{
    pg_enums::{Architecture, OperationSystem, PgAuthMethod},
    pg_fetch::{PgFetchSettings, PostgresVersion},
    postgres::{PgEmbed, PgSettings},
};
use tokio::runtime::Handle;
use tracing::*;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum ConnectionType {
    External(String),
    Embedded {
        root_path: PathBuf,
        port: i16,
        username: String,
        password: String,
        persistent: bool,
        timeout: Duration,
        host: String,
    },
}

pub enum ConnectionLock {
    External(String),
    Embedded(Box<PgEmbed>),
}

impl ConnectionLock {
    pub fn as_uri(&self) -> &str {
        match self {
            ConnectionLock::External(conn_string) => &conn_string,
            ConnectionLock::Embedded(pg) => &pg.db_uri,
        }
    }

    pub fn full_db_uri(&self, db_name: &str) -> String {
        match self {
            ConnectionLock::External(conn_string) => String::from(conn_string),
            ConnectionLock::Embedded(pg) => (&pg).full_db_uri(db_name),
        }
    }

    pub async fn cleanup(&mut self) {
        match self {
            ConnectionLock::External(_) => (),
            ConnectionLock::Embedded(pg) => {
                info!("Shutting down embedded database...");
                if let Err(e) = pg.stop_db().await {
                    error!("Error occurred: {:#?}", e);
                } else {
                    info!("Embedded database is shut down");
                }
            }
        }
    }
}

impl Drop for ConnectionLock {
    fn drop(&mut self) {
        info!("Called drop on ConnectionLock");
        // let ct = tokio::runtime::Builder::new_current_thread()
        //     .enable_all()
        //     .build()
        //     .unwrap();
        let handle = Handle::current();
        let _ = handle.enter();
        futures::executor::block_on(self.cleanup());
        // let rt = tokio::runtime::Runtime::new().expect("could not start tokio rt");
        // let _ = ct.spawn_blocking(async move || self.cleanup().await);
    }
}

fn get_fetch_settings(host: String) -> anyhow::Result<PgFetchSettings> {
    let operating_system = if cfg!(target_os = "linux") {
        OperationSystem::Linux
    } else if cfg!(target_os = "macos") {
        OperationSystem::Darwin
    } else if cfg!(target_os = "windows") {
        OperationSystem::Windows
    } else {
        bail!("unsupported `target_os`");
    };

    let architecture = if cfg!(target_arch = "x86_64") {
        Architecture::Amd64
    } else if cfg!(target_arch = "aarch64") {
        Architecture::Arm64v8
    } else {
        bail!("unsupported `target_arch`");
    };

    Ok(PgFetchSettings {
        host: host.clone(),
        operating_system,
        architecture,
        // Yay hardcoding because can't select a custom version that PgEmbed doesn't have hardcoded in for some reason, 13 is old but may as well as its the latest for pg_embed...
        version: PostgresVersion("14.3.0"),
    })
}

impl ConnectionType {
    pub async fn init_conn_string(&self) -> anyhow::Result<ConnectionLock> {
        match self {
            ConnectionType::External(conn_string) => {
                Ok(ConnectionLock::External(conn_string.clone()))
            }
            ConnectionType::Embedded {
                root_path,
                port,
                username,
                password,
                persistent,
                timeout,
                host,
            } => {
                info!("initializing an embedded postgresql database");
                let database_dir = root_path.join("db").to_owned();

                if database_dir.exists() {
                    let database_dir_str = database_dir.to_str().unwrap();
                    if database_dir.is_dir() {
                        if database_dir_str != "/" && database_dir_str != env!("CARGO_MANIFEST_DIR")
                        {
                            fs::remove_dir_all(database_dir_str)?;
                        }
                    } else {
                        fs::remove_file(database_dir_str)?;
                    }
                }
                fs::create_dir_all(&database_dir)?;

                let pg_settings = PgSettings {
                    database_dir: database_dir.canonicalize().unwrap().clone(),
                    // Why is port an `i16` instead of a `u16`?!?
                    port: *port,
                    user: username.to_owned(),
                    password: password.to_owned(),
                    persistent: *persistent,
                    timeout: Some(*timeout),
                    migration_dir: None,
                    auth_method: PgAuthMethod::Plain,
                };

                info!("Initializing embedded postgresql database");
                let mut pg =
                    match PgEmbed::new(pg_settings, get_fetch_settings(host.clone())?).await {
                        Ok(e) => e,
                        Err(e) => {
                            error!("An error occurred creating new PgEmbed: {:?}", e);
                            return Err(anyhow::anyhow!(e.to_string()));
                        }
                    };

                info!("Setting up embedded postgresql database");
                match pg.setup().await {
                    Ok(_) => {}
                    Err(e) => {
                        println!("ERROR: {:?}", e);
                        error!("An error occurred setting up: {:?}", e);
                        return Err(anyhow::anyhow!(e.to_string()));
                    }
                };

                info!("Starting embedded postgresql database");
                // start postgresql database
                match pg.start_db().await {
                    Ok(e) => e,
                    Err(e) => {
                        error!("An error occurred starting database: {:?}", e);
                        return Err(anyhow::anyhow!(e.to_string()));
                    }
                }

                info!("Embedded postgresql database successfully started");
                info!("Database connection URI: {}", &pg.db_uri);
                Ok(ConnectionLock::Embedded(Box::new(pg)))
            }
        }
    }
}
