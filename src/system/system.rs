use std::sync::{Arc, Mutex};

use anyhow::bail;
use tracing::*;

use super::{config::ConfigDatabase, db::DB, logger};

pub struct SystemInner {
    pub db_lock: Arc<Mutex<DB>>,
    pub running: bool,
}

impl SystemInner {
    pub async fn new() -> Self {
        // let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        // Make configurable?
        let db_lock = Arc::new(Mutex::new(
            DB::new_embedded(ConfigDatabase::default()).await,
        ));
        let running = false;
        Self { db_lock, running }
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
        if !self.running {
            self.start().await?;
        }
        info!("Creating new database");
        println!("create_new_db asdjfkasjdkfjaskdfajsdf");
        match self.db_lock.lock().unwrap().create_new_db(name).await {
            // match self.db_lock.create_new_db(name).await {
            Err(e) => {
                error!("Unable to create a new database: {:?}", e.to_string());
                bail!(e.to_string())
            }
            Ok(res) => {
                println!("create_new_db asdjfkasjdkfjaskdfajsdf: {:#?}", res);
                info!("Created new database: {:?}", res);
                Ok(res)
            }
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
                    Ok(res)
                }
            }
        } else {
            Ok(true)
        }
    }
}

impl Drop for SystemInner {
    fn drop(&mut self) {
        drop(&self.db_lock);
    }
}

pub struct System {
    inner: Arc<Mutex<SystemInner>>,
}

impl System {
    pub async fn initialize() -> anyhow::Result<System> {
        let _logger = logger::init_logging(None);

        let inner = SystemInner::new().await;
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_system_can_be_created() {}
}
