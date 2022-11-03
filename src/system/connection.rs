use std::{error::Error, path::PathBuf, time::Duration};

use anyhow::bail;
use pg_embed::{
    pg_enums::{Architecture, OperationSystem, PgAuthMethod},
    pg_fetch::{PgFetchSettings, PostgresVersion, PG_V13},
    postgres::{PgEmbed, PgSettings},
};
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
}

impl Drop for ConnectionLock {
    fn drop(&mut self) {
        match self {
            ConnectionLock::External(_) => (),
            ConnectionLock::Embedded(pg) => {
                info!("Shutting down embedded database...");
                // let rt = tokio::runtime::Builder::new_current_thread()
                //     .enable_time()
                //     .build()
                //     .unwrap();

                // rt.block_on(async move {
                let _dropped_ = pg.stop_db();
                // });
                // info!("Shutting down embedded database...");
                // if let Err(e) = pg.stop_db() {
                //     error!("error upon shutting down embedded postgres: {:?}", e);
                // } else {
                //     info!("Embedded database is shut down");
                // }
            }
        }
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
        version: PostgresVersion("14.5.0"),
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

                let pg_settings = PgSettings {
                    // Why are these utf-8 strings instead of `Path`/`PathBuf`'s?!?
                    database_dir: PathBuf::from(database_dir.clone()),
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
                // Download, unpack, create password file and database cluster
                // This is the next 3 lines: pg.setup().await?;
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
