use std::time::Duration;
use tempdir::TempDir;

use serde::{Deserialize, Serialize};

use super::db::DBType;

#[derive(Debug, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ConfigDatabase {
    pub db_type: String,
    pub uri: String,
    pub root_path: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub persistent: Option<bool>,
    pub port: Option<i16>,
    pub timeout: Option<Duration>,
    pub host: Option<String>,
}

impl Into<DBType> for ConfigDatabase {
    fn into(self) -> DBType {
        match self.db_type.as_str() {
            "External" => DBType::External(self.uri.to_string()),
            _ => DBType::Embedded {
                root_path: self.root_path.unwrap().into(),
                port: self.port.unwrap() as i16,
                username: self.username.unwrap(),
                password: self.password.unwrap(),
                persistent: self.persistent.unwrap(),
                timeout: self.timeout.unwrap(),
                host: self.host.unwrap(),
            },
        }
    }
}

impl Default for ConfigDatabase {
    fn default() -> Self {
        let root_path = match TempDir::new("db") {
            Ok(v) => String::from(v.path().to_str().unwrap()),
            Err(_) => ".".to_string(),
        };
        Self {
            db_type: "Embedded".to_string(),
            uri: "127.0.0.1".to_string(),
            timeout: Some(Duration::from_secs(5)),
            root_path: Some(root_path.to_string()),
            username: Some("postgres".to_string()),
            password: Some("postgres".to_string()),
            persistent: Some(false),
            port: None, //Some(5433),
            // max_connections: 5,
            host: Some("https://repo1.maven.org".to_string()),
        }
    }
}
