use anyhow::bail;
use url::{Position, Url};

use pg_embed::{
    pg_enums::{Architecture, OperationSystem, PgAuthMethod},
    pg_fetch::{PgFetchSettings, PostgresVersion},
    postgres::{PgEmbed, PgSettings},
};
use portpicker::pick_unused_port;
use sqlx::{postgres::PgPoolOptions, Executor};

use std::{
    fmt::Debug,
    fs::{self},
    path::PathBuf,
    time::Duration,
};
use sysinfo::{ProcessExt, ProcessRefreshKind, RefreshKind, System as SysInfoSystem, SystemExt};
use tracing::*;

use super::config::ConfigDatabase;

#[derive(Debug)]
pub struct DB {
    connection: DBLock,
}

impl DB {
    #[allow(unused)]
    pub async fn new_external(root_path: String, uri: impl Into<String>) -> Self {
        let db_type = DBType::External(uri.into());
        let connection = db_type.init_conn_string().await.unwrap();
        let root_path = PathBuf::from(root_path);
        Self { connection }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn new_embedded(config: ConfigDatabase) -> Self {
        // let port = pick_unused_port().expect("Unable to pick a random port");
        let port = match config.port {
            None => pick_unused_port().expect("Unable to pick an unused port") as i16,
            Some(p) => p as i16,
        };
        let cfg_root_path = config.root_path.unwrap();
        let root_path = PathBuf::from(&cfg_root_path);
        // TODO: decide to put this back or not?
        let db_type = DBType::Embedded {
            root_path,
            port,
            username: config.username.unwrap(),
            password: config.password.unwrap(),
            persistent: config.persistent.unwrap(),
            timeout: config.timeout.unwrap(),
            host: config.host.unwrap(),
        };
        let connection = match db_type.init_conn_string().await {
            Err(e) => {
                error!("Error creating connection: {:?}", e.to_string());
                panic!("ERROR: {:?}", e.to_string())
            }
            Ok(r) => r,
        };
        Self { connection }
    }

    #[allow(unused)]
    pub fn as_uri(&self) -> &str {
        self.connection.as_uri()
    }

    #[allow(unused)]
    pub fn full_db_uri(&self, db_name: &str) -> String {
        self.connection.full_db_uri(db_name)
    }

    pub async fn start(&mut self) -> anyhow::Result<bool> {
        let res = self.connection.start().await;
        res
    }

    pub async fn create_new_db(&mut self, name: Option<String>) -> anyhow::Result<String> {
        log::info!("Creating new database");
        let (_db_name, conn_url) = self.connection.create_new_db(name).await?;
        // log::info!(
        //     "New database created. Now running sql migration: {:?}",
        //     db_name
        // );
        // let full_uri = self.connection.full_db_uri(&db_name);
        // let _ = self
        //     .connection
        //     .sql(full_uri, self.migration_sql.clone())
        //     .await;

        Ok(conn_url)
    }

    pub async fn drop_database(&mut self, uri: String, db_name: String) -> anyhow::Result<()> {
        log::info!("Dropping database {}", db_name);
        match self.connection.drop_database(uri, db_name).await {
            Err(e) => {
                error!("Error dropping database: {:?}", e.to_string());
                Err(anyhow::anyhow!(e.to_string()))
            }
            Ok(_) => Ok(()),
        }
    }

    pub async fn stop(&mut self) -> anyhow::Result<bool> {
        let res = self.connection.stop().await?;
        log::debug!("Stopped connection");
        Ok(res)
    }

    pub async fn execute_sql(&mut self, uri: String, sql: String) -> anyhow::Result<()> {
        let res = self.connection.sql(uri, sql).await?;
        log::debug!("Executed sql");
        Ok(res)
    }
}

#[derive()]
pub enum DBLock {
    External(String),
    Embedded(Box<PgEmbed>),
}

impl Debug for DBLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DBLock::External(s) => write!(f, "External: {:?}", s),
            DBLock::Embedded(pg) => write!(f, "Embedded: {:?}", pg.db_uri),
        }
    }
}

impl DBLock {
    pub fn as_uri(&self) -> &str {
        match self {
            DBLock::External(conn_string) => &conn_string,
            DBLock::Embedded(pg) => &pg.db_uri,
        }
    }

    pub fn full_db_uri(&self, db_name: &str) -> String {
        match self {
            DBLock::External(conn_string) => String::from(conn_string),
            DBLock::Embedded(pg) => (&pg).full_db_uri(db_name),
        }
    }

    async fn start(&mut self) -> anyhow::Result<bool> {
        match self {
            DBLock::External(_s) => Ok(true),
            DBLock::Embedded(pg) => {
                log::info!("Starting embedded postgresql database");
                // start postgresql database
                match pg.start_db().await {
                    Ok(e) => {
                        error!("Error when starting database: {:?}", e);
                        Ok(true)
                    }
                    Err(e) => {
                        error!("An error occurred starting database: {:?}", e);
                        return Err(anyhow::anyhow!(e.to_string()));
                    }
                }
            }
        }
    }

    async fn create_new_db(&mut self, name: Option<String>) -> anyhow::Result<(String, String)> {
        let name = name.unwrap_or_else(|| cuid::cuid().unwrap().to_string());
        match self {
            DBLock::External(_s) => {
                // TODO: Implement, maybe?
                Err(anyhow::anyhow!(
                    "Not implemented for external databases, yet"
                ))
            }
            DBLock::Embedded(pg) => {
                let res = pg.create_database(&name).await;
                log::info!("Create database result: {:?}", res);
                let conn_url = pg.full_db_uri(&name);
                Ok((name, conn_url))
            }
        }
    }

    async fn drop_database(&mut self, uri: String, db_name: String) -> anyhow::Result<()> {
        let uri2 = uri.clone();
        let parsed = Url::parse(uri2.as_str()).unwrap();
        let cleaned: &str = &parsed[..Position::AfterPort];
        let _res = self
            .sql(
                cleaned.to_owned(),
                String::from(format!(
                    r#"SELECT pg_terminate_backend(pg_stat_activity.pid)
        FROM pg_stat_activity
        WHERE pg_stat_activity.datname = '{}';
        "#,
                    db_name
                )),
            )
            .await;

        match self {
            DBLock::External(_s) => Ok(()),
            DBLock::Embedded(pg) => match pg.drop_database(db_name.as_str()).await {
                Err(e) => {
                    error!("Error occurred dropping database: {:?}", e.to_string());
                    return Err(anyhow::anyhow!(e.to_string()));
                }
                Ok(_) => {
                    log::info!("Dropped database in DBLock: {}", db_name);
                    Ok(())
                }
            },
        }
    }

    async fn stop(&mut self) -> anyhow::Result<bool> {
        log::trace!("Called stop in db");
        match self {
            DBLock::External(_s) => Ok(true),
            DBLock::Embedded(pg) => {
                log::info!("Stopping embedded postgresql database");
                // start postgresql database
                match pg.stop_db().await {
                    Ok(_e) => {
                        log::debug!("Database successfully stopped");
                        Ok(true)
                    }
                    Err(e) => {
                        log::error!("An error occurred stopping database: {:?}", e);
                        Err(anyhow::anyhow!(e.to_string()))
                    }
                }
            }
        }
    }

    async fn sql(&mut self, uri: String, sql: String) -> anyhow::Result<()> {
        let pool = PgPoolOptions::new()
            .max_connections(32)
            .connect(&uri)
            .await?;

        match self {
            DBLock::External(_s) => {
                // TODO: implement, maybe?
                Err(anyhow::anyhow!(
                    "Not implemented for external databases yet"
                ))
            }
            DBLock::Embedded(_pg) => match pool.execute(sql.as_str()).await {
                Ok(_r) => Ok(()),
                Err(e) => {
                    error!("Error occurrend when migrating: {:?}", e.to_string());
                    Err(anyhow::anyhow!(e.to_string()))
                }
            },
        }
    }
}

#[derive(Debug)]
pub enum DBType {
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

impl Default for DBType {
    fn default() -> Self {
        let cfg = ConfigDatabase::default();
        DBType::Embedded {
            root_path: PathBuf::from(cfg.root_path.unwrap()),
            port: portpicker::pick_unused_port().unwrap() as i16,
            username: cfg.username.unwrap(),
            password: cfg.password.unwrap(),
            persistent: cfg.persistent.unwrap(),
            timeout: cfg.timeout.unwrap(),
            host: cfg.host.unwrap(),
        }
    }
}

impl DBType {
    pub async fn init_conn_string(&self) -> anyhow::Result<DBLock> {
        match self {
            DBType::External(conn_string) => Ok(DBLock::External(conn_string.clone())),
            DBType::Embedded {
                root_path,
                port,
                username,
                password,
                persistent,
                timeout,
                host,
            } => {
                log::info!("initializing an embedded postgresql database");
                let database_dir = root_path.join("db").to_owned();

                self.clear_out_db_path(database_dir.clone())?;
                // self.kill_any_existing_postgres_process()?;

                let pg_settings = PgSettings {
                    database_dir: database_dir.canonicalize().unwrap().clone(),
                    // Why is port an `i16` instead of a `u16`?!?
                    port: *port,
                    user: username.to_owned(),
                    password: password.to_owned(),
                    persistent: *persistent,
                    timeout: Some(*timeout),
                    migration_dir: Some(root_path.into()),
                    auth_method: PgAuthMethod::Plain,
                };

                log::info!("Initializing embedded postgresql database");
                let mut pg = match PgEmbed::new(
                    pg_settings,
                    Self::get_fetch_settings(host.clone())?,
                )
                .await
                {
                    Ok(e) => e,
                    Err(e) => {
                        error!("An error occurred creating new PgEmbed: {:?}", e);
                        return Err(anyhow::anyhow!(e.to_string()));
                    }
                };

                log::info!("Setting up embedded postgresql database");
                match pg.setup().await {
                    Ok(_) => {}
                    Err(e) => {
                        error!("An error occurred setting up: {:?}", e);
                        return Err(anyhow::anyhow!(e.to_string()));
                    }
                };

                log::info!("Embedded postgresql database successfully started");
                log::info!("Database connection URI: {}", &pg.db_uri);
                Ok(DBLock::Embedded(Box::new(pg)))
            }
        }
    }

    fn clear_out_db_path(&self, database_dir: PathBuf) -> anyhow::Result<()> {
        log::info!("initializing an embedded postgresql database");
        if database_dir.exists() {
            let database_dir_str = database_dir.to_str().unwrap();
            if database_dir.is_dir() {
                if database_dir_str != "/" && database_dir_str != env!("CARGO_MANIFEST_DIR") {
                    fs::remove_dir_all(database_dir_str)?;
                }
            } else {
                fs::remove_file(database_dir_str)?;
            }
        }
        fs::create_dir_all(&database_dir)?;

        Ok(())
    }

    #[allow(unused)]
    fn kill_any_existing_postgres_process(&self) -> anyhow::Result<()> {
        debug!("Checking if postgres is already running");
        let s = SysInfoSystem::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
        );

        for process in s.processes_by_name("postgres") {
            debug!(
                "Found postgres process: {} {}",
                process.pid(),
                process.name()
            );
            process.kill();
        }
        Ok(())
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_clear_out_db_path() {}
}
