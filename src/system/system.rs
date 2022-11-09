use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::bail;
use tempdir::TempDir;
use tracing::*;

use super::{config::ConfigDatabase, db::DB, logger};

#[derive(Debug)]
pub struct SystemInner {
    pub db_lock: Arc<Mutex<DB>>,
    pub running: bool,
    pub root_path: String,
}

impl SystemInner {
    pub async fn new(root_dir: String) -> Self {
        // Make configurable?
        let mut config = ConfigDatabase::default();
        let database_dir = match TempDir::new("db") {
            Ok(v) => String::from(v.path().to_str().unwrap()),
            Err(_) => root_dir,
        };
        let root_path = Some(database_dir.clone());
        config.root_path = root_path;

        let db_lock = Arc::new(Mutex::new(DB::new_embedded(config).await));
        let running = false;
        Self {
            db_lock,
            running,
            root_path: database_dir.clone(),
        }
    }

    pub async fn start(&mut self) -> anyhow::Result<bool> {
        // Incase we're already running, don't start
        if !self.running {
            let mut db_lock = self.db_lock.lock().unwrap();
            match db_lock.start().await {
                Err(e) => {
                    error!("Unable to start database: {:?}", e.to_string());
                    bail!(e.to_string())
                }
                Ok(res) => {
                    self.running = true;
                    Ok(res)
                }
            }
        } else {
            Ok(true)
        }
    }

    pub async fn create_new_db(&mut self, name: Option<String>) -> anyhow::Result<String> {
        println!("create_new_db called in system");
        if self.running {
            info!("Creating new database");
            match self.db_lock.lock().unwrap().create_new_db(name).await {
                // match self.db_lock.create_new_db(name).await {
                Err(e) => {
                    error!("Unable to create a new database: {:?}", e.to_string());
                    bail!(e.to_string())
                }
                Ok(res) => {
                    info!("Created new database: {:?}", res);
                    Ok(res)
                }
            }
        } else {
            error!("Not running. Call start first");
            Ok("error".to_string())
        }
    }

    pub async fn drop_database(&mut self, uri: String, name: String) -> anyhow::Result<()> {
        if self.running {
            info!("Dropping database: {}", name);
            let mut lock = self.db_lock.lock().unwrap();
            match lock.drop_database(uri, name).await {
                Err(e) => {
                    error!("Unable to drop database: {:?}", e.to_string());
                    bail!(e.to_string())
                }
                Ok(_) => Ok(()),
            }
        } else {
            Ok(())
        }
    }

    pub async fn stop(&mut self) -> anyhow::Result<bool> {
        // Incase we're not running, don't stop
        if self.running {
            let mut db_lock = self.db_lock.lock().unwrap();
            match db_lock.stop().await {
                Err(e) => {
                    error!("Unable to stop database: {:?}", e.to_string());
                    bail!(e.to_string())
                }
                Ok(res) => {
                    self.running = false;
                    info!("DBLock stopped");
                    Ok(res)
                }
            }
        } else {
            Ok(true)
        }
    }
}

#[derive(Debug)]
pub struct System {
    inner: Arc<Mutex<SystemInner>>,
}

impl System {
    pub async fn initialize(root_dir: String) -> anyhow::Result<System> {
        let _logger = logger::init_logging(None);

        let inner = SystemInner::new(root_dir).await;
        let inner = Arc::new(Mutex::new(inner));

        Ok(Self { inner })
    }

    pub async fn start(&mut self) -> anyhow::Result<bool> {
        let mut inner = self.inner.lock().unwrap();
        Ok(inner.start().await?)
    }

    pub async fn stop(&mut self) -> anyhow::Result<bool> {
        let mut inner = self.inner.lock().unwrap();
        Ok(inner.stop().await?)
    }

    pub async fn create_new_db(&mut self, name: Option<String>) -> anyhow::Result<String> {
        let mut inner = self.inner.lock().unwrap();
        Ok(inner.create_new_db(name).await?)
    }

    pub async fn drop_database(&mut self, uri: String, name: String) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        Ok(inner.drop_database(uri, name).await?)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_system_can_be_created() {}
}
