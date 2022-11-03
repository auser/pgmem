use anyhow::bail;
use portpicker::pick_unused_port;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool, Transaction};
use std::convert::TryInto;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::*;

use super::connection::{ConnectionLock, ConnectionType};

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConfigDatabase {
    pub root_path: String,
    pub username: String,
    pub password: String,
    pub persistent: bool,
    pub port: i16,
    pub timeout: Duration,
    pub db_type: String,
    pub max_connections: u8,
    pub uri: Option<String>,
    pub host: Option<String>,
}

impl Default for ConfigDatabase {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            db_type: "Embedded".to_string(),
            root_path: ".".to_string(),
            username: "postgres".to_string(),
            password: "postgres".to_string(),
            persistent: false,
            port: 5432,
            max_connections: 5,
            uri: None,
            host: Some("https://repo1.maven.org".to_owned()),
        }
    }
}

impl Into<ConnectionType> for ConfigDatabase {
    fn into(self) -> ConnectionType {
        ConnectionType::Embedded {
            root_path: self.root_path.into(),
            port: self.port as i16,
            username: self.username.into(),
            password: self.password.into(),
            persistent: self.persistent,
            timeout: self.timeout,
            host: self.host.unwrap(),
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DatabaseConfig {
    pub connection: ConnectionType,
    pub max_connections: u8,
}

impl DatabaseConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new_embedded(max_connections: u8, config_database: ConfigDatabase) -> Self {
        let port = pick_unused_port().expect("Unable to pick a random port");
        let connection: ConnectionType = config_database.into();
        Self {
            connection,
            max_connections,
        }
    }

    pub fn new_external(max_connections: u8, uri: impl Into<String>) -> Self {
        Self {
            connection: ConnectionType::External(uri.into()),
            max_connections,
        }
    }

    pub async fn create_database_pool(&self) -> anyhow::Result<(ConnectionLock, DbPool)> {
        info!("Initializing postgresql database connection");
        let connection = self.connection.init_conn_string().await?;

        let pool = PgPoolOptions::new()
            .max_connections(self.max_connections as u32)
            .connect(connection.as_uri())
            .await?;

        let pool: DbPool = Arc::new(pool);
        migrate_migration_table(&pool)
            .await
            .expect("failed migrating the migration table");

        info!("Successfully initialized the database connection pool");
        Ok((connection, pool))
    }
}

pub type DbPool = Arc<PgPool>;
pub type DbTransaction<'a> = Transaction<'a, sqlx::Postgres>;

async fn migrate_migration_table(pool: &PgPool) -> anyhow::Result<()> {
    pool.execute(
        r#"
		CREATE TABLE IF NOT EXISTS _migrations (
			module text NOT NULL,
			version bigint NOT NULL,
			checksum bytea NOT NULL,
			description text NOT NULL,
			inserted_at timestamp without time zone NOT NULL DEFAULT now(),
			CONSTRAINT _migrations_pkey PRIMARY KEY (module, version)
		) WITH (
			OIDS=FALSE
		);
	"#,
    )
    .await?;
    info!("Migration Table loaded");
    Ok(())
}

#[derive(Clone)]
pub struct Migration<'d, 'su, 'sd> {
    pub description: &'d str,
    pub sql_up: &'su str,
    pub sql_down: &'sd str,
}

pub struct Migrations<'n, 'm, 'd, 'su, 'sd> {
    pub module: &'n str,
    pub migrations: &'m [Migration<'d, 'su, 'sd>],
}

impl<'d, 'su, 'sd> Migration<'d, 'su, 'sd> {
    pub const fn new(description: &'d str) -> Self {
        Self {
            description,
            sql_up: "",
            sql_down: "",
        }
    }

    pub const fn up(self, sql_up: &'su str) -> Self {
        // const-time panics are not yet in Rust:
        // assert!(self.sql_up.is_empty(), "cannot replace existing up sql");
        Self { sql_up, ..self }
    }

    pub const fn down(self, sql_down: &'sd str) -> Self {
        // const-time panics are not yet in Rust:
        // assert!(self.sql_down.is_empty(), "cannot replace existing down sql");
        Self { sql_down, ..self }
    }

    pub const fn sql(self, sql_up: &'su str, sql_down: &'sd str) -> Self {
        // const-time panics are not yet in Rust:
        // assert!(self.sql_up.is_empty(), "cannot replace existing up sql");
        // assert!(self.sql_down.is_empty(), "cannot replace existing down sql");
        Self {
            sql_up,
            sql_down,
            ..self
        }
    }

    pub fn checksum(&self) -> [u8; 64] {
        use sha2::digest::{Digest, Update};
        sha2::Sha512::default()
            .chain(self.sql_up.as_bytes())
            .chain(self.sql_down.as_bytes())
            .finalize()
            .as_slice()
            .try_into()
            .expect("somehow SHA512 is suddenly no longer returning 512 bits?!?")
    }

    async fn migrate_up(
        &self,
        module: &str,
        conn: &mut Transaction<'_, sqlx::Postgres>,
    ) -> anyhow::Result<()> {
        info!("Migrate up {}", module);
        conn.execute(self.sql_up.as_ref()).await?;
        Ok(())
    }
}

impl<'n, 'm, 'd, 'su, 'sd> Migrations<'n, 'm, 'd, 'su, 'sd> {
    pub const fn new(module: &'n str, migrations: &'m [Migration<'d, 'su, 'sd>]) -> Self {
        Self { module, migrations }
    }

    pub async fn migrate_up(&self, pool: &PgPool) -> anyhow::Result<()> {
        if !self.migrations.is_empty() {
            info!("Migrating all up on {}", &self.module);
            // Why is the `conn.transaction` call wrapper boxing a future?!?  Wasteful...
            let mut conn = pool.begin().await?;
            // Why doesn't sqlx support decoding to unsigned integers?!
            // Why doesn't sqlx support decoding to a constant length array or reference thereof?!
            let mut current = sqlx::query_as::<_, (i64, Vec<u8>)>(
                "SELECT version, checksum FROM _migrations WHERE module = $1 ORDER BY version DESC",
            )
            .bind(self.module)
            .fetch_all(&mut conn)
            .await?;
            for (mig_version, mig) in self.migrations.iter().enumerate() {
                let mig_version = mig_version as i64;
                if let Some((version, checksum)) = current.pop() {
                    if checksum.len() != 64 {
                        bail!("Migration database checksum length is invalid for module {} with version {}", &self.module, version);
                    } else if version != mig_version {
                        bail!(
                            "Version mismatch in {}: {} -> {}",
                            &self.module,
                            version,
                            mig_version
                        );
                    } else if checksum != mig.checksum() {
                        bail!(
                            "Checksum mismatch in {} for version {}: {:?} -> {:?}",
                            &self.module,
                            version,
                            &checksum,
                            mig.checksum()
                        );
                    }
                } else {
                    mig.migrate_up(&self.module, &mut conn).await?;
                    sqlx::query("INSERT INTO _migrations(module, version, checksum, description) VALUES ($1, $2, $3, $4)")
						.bind(self.module)
						.bind(mig_version)
						.bind(mig.checksum().as_ref()) // Why can't sqlx accept an array directly to bind?
						.bind(mig.description)
						.execute(&mut conn)
						.await?;
                }
            }
            conn.commit().await?;
        }
        Ok(())
    }
}
