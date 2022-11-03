use anyhow::bail;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool, Transaction};
use std::convert::TryInto;
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
        // let port = pick_unused_port().expect("Unable to pick a random port");
        let connection: ConnectionType = config_database.into();
        Self {
            connection,
            max_connections,
        }
    }

    #[allow(unused)]
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
        info!("Successfully initialized the database connection pool");
        Ok((connection, pool))
    }
}

pub type DbPool = Arc<PgPool>;
