use anyhow::bail;
use pg_embed::{
    pg_enums::{Architecture, OperationSystem, PgAuthMethod},
    pg_fetch::{PgFetchSettings, PostgresVersion},
    postgres::{PgEmbed, PgSettings},
};
use portpicker::pick_unused_port;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{
    fmt::Debug,
    fs::{self},
    path::PathBuf,
    time::Duration,
};
use sysinfo::{ProcessExt, ProcessRefreshKind, RefreshKind, System as SysInfoSystem, SystemExt};
use tokio::runtime::Handle;
use tracing::*;

use super::{config::ConfigDatabase, utils::read_all_migrations};
pub type DbPool = PgPool;
pub type DbLock = DBLock;

#[derive(Debug)]
pub struct DB {
    connection: DBLock,
    // root_path: PathBuf,
    // migration_sql: String,
    pub db_pool: Option<DbPool>,
}

impl DB {
    #[allow(unused)]
    pub async fn new_external(root_path: String, uri: impl Into<String>) -> Self {
        let db_type = DBType::External(uri.into());
        let connection = db_type.init_conn_string().await.unwrap();
        let root_path = PathBuf::from(root_path);
        let migration_sql = read_all_migrations(root_path.clone());
        // let (connection, db_pool): (DbLock, DbPool) = db_type.create_database_pool().await.unwrap();
        Self {
            connection,
            // root_path: root_path.clone(),
            db_pool: None,
            // migration_sql,
        }
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
        let _migration_sql = read_all_migrations(root_path.clone());
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
        // let (connection, db_pool): (DbLock, DbPool) = db_type.create_database_pool().await.unwrap();
        Self {
            connection,
            // root_path: PathBuf::from(cfg_root_path),
            db_pool: None,
            // migration_sql,
        }
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
        let (_connection, db_pool): (&DbLock, DbPool) = self.create_database_pool().await.unwrap();
        self.db_pool = Some(db_pool);
        res
    }

    pub async fn create_database_pool(&self) -> anyhow::Result<(&DbLock, DbPool)> {
        info!("Initializing postgresql database connection");
        let connection = &self.connection;

        let pool = PgPoolOptions::new()
            .max_connections(32)
            .connect(connection.as_uri())
            .await?;

        // let pool: DbPool = Arc::new(pool);
        // let connection = Arc::new(connection);
        info!("Successfully initialized the database connection pool");
        Ok((connection, pool))
    }

    pub async fn create_new_db(&mut self, name: Option<String>) -> anyhow::Result<String> {
        info!("Creating new database");
        println!("Calling create_new_db on the connection");
        let (db_name, conn_url) = self.connection.create_new_db(name).await?;
        info!(
            "New database created. Now running sql migration: {:?}",
            db_name
        );
        self.connection.migrate(db_name).await?;
        // // sqlx::migrate!(self.root_path.as_str());
        info!("Migration ran return");
        Ok(conn_url)
    }

    pub async fn stop(&mut self) -> anyhow::Result<bool> {
        self.connection.stop().await
    }
}

impl<'a> Drop for DB {
    fn drop(&mut self) {
        info!("Called drop on DB");
        drop(self.db_pool.as_ref());
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

    pub async fn cleanup(&mut self) -> anyhow::Result<bool> {
        self.stop().await
    }

    async fn start(&mut self) -> anyhow::Result<bool> {
        match self {
            DBLock::External(_s) => Ok(true),
            DBLock::Embedded(pg) => {
                info!("Starting embedded postgresql database");
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
        let name = name.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        match self {
            DBLock::External(_s) => {
                // TODO: Implement, maybe?
                Err(anyhow::anyhow!(
                    "Not implemented for external databases, yet"
                ))
            }
            DBLock::Embedded(pg) => {
                info!("Creating new database");
                let res = pg.create_database(&name).await;
                println!("Result => {:?}", res);
                info!("Create database result: {:?}", res);
                let conn_url = pg.full_db_uri(&name);
                println!("Name and conn: {:?}", (name.clone(), conn_url.clone()));
                Ok((name, conn_url))
            }
        }
    }

    async fn migrate(&mut self, name: String) -> anyhow::Result<()> {
        // let m = Migrator::new(Path::new(self.root_path.as_path())).await?;

        // let db_pool = &self.db_pool.clone().unwrap();
        // let _res = m.run(db_pool).await?;
        // let sql = self.migration_sql.clone();

        // let _res = sqlx::query(&sql).execute(db_pool).await;
        match self {
            DBLock::External(_s) => {
                // TODO: implement, maybe?
                Err(anyhow::anyhow!(
                    "Not implemented for external databases yet"
                ))
            }
            DBLock::Embedded(pg) => {
                info!("Migrating {} in embedded db", &name);
                match pg.migrate(&name).await {
                    Ok(r) => Ok(r),
                    Err(e) => {
                        error!("Error occurrend when migrating: {:?}", e.to_string());
                        Err(anyhow::anyhow!(e.to_string()))
                    }
                }
            }
        }
    }

    async fn stop(&mut self) -> anyhow::Result<bool> {
        match self {
            DBLock::External(_s) => Ok(true),
            DBLock::Embedded(pg) => {
                info!("Starting embedded postgresql database");
                // start postgresql database
                match pg.stop_db().await {
                    Ok(_e) => Ok(true),
                    Err(e) => {
                        error!("An error occurred starting database: {:?}", e);
                        return Err(anyhow::anyhow!(e.to_string()));
                    }
                }
            }
        }
    }
}

impl<'a> Drop for DBLock {
    fn drop(&mut self) {
        info!("Called drop on ConnectionLock");
        let handle = Handle::current();
        let _ = handle.enter();
        let _ = futures::executor::block_on(self.cleanup());
        match self {
            DBLock::External(_s) => {}
            DBLock::Embedded(pg) => {
                drop(pg);
            }
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
                info!("initializing an embedded postgresql database");
                let database_dir = root_path.join("db").to_owned();

                self.clear_out_db_path(database_dir.clone())?;
                self.kill_any_existing_postgres_process()?;

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

                info!("Initializing embedded postgresql database");
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

                info!("Setting up embedded postgresql database");
                match pg.setup().await {
                    Ok(_) => {}
                    Err(e) => {
                        println!("ERROR: {:?}", e);
                        error!("An error occurred setting up: {:?}", e);
                        return Err(anyhow::anyhow!(e.to_string()));
                    }
                };

                info!("Embedded postgresql database successfully started");
                info!("Database connection URI: {}", &pg.db_uri);
                Ok(DBLock::Embedded(Box::new(pg)))
            }
        }
    }

    fn clear_out_db_path(&self, database_dir: PathBuf) -> anyhow::Result<()> {
        info!("initializing an embedded postgresql database");
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
