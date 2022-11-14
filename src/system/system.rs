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
    pub async fn new(mut config: ConfigDatabase) -> Self {
        // Make configurable?
        // let mut config = ConfigDatabase::default();
        let database_dir = match config.root_path {
            Some(v) => v,
            None => match TempDir::new("db") {
                Ok(v) => String::from(v.path().to_str().unwrap()),
                Err(_e) => String::from(PathBuf::from(".").join("db").as_path().to_str().unwrap()),
            },
        };
        let root_path = Some(database_dir.clone());
        config.root_path = root_path;

        let db_lock = match config.db_type.as_str() {
            "External" => Arc::new(Mutex::new(
                DB::new_external(database_dir.clone(), config.uri).await,
            )),
            _ => Arc::new(Mutex::new(DB::new_embedded(config).await)),
        };

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
        if self.running {
            log::info!("Creating new database");
            match self.db_lock.lock().unwrap().create_new_db(name).await {
                // match self.db_lock.create_new_db(name).await {
                Err(e) => {
                    error!("Unable to create a new database: {:?}", e.to_string());
                    bail!(e.to_string())
                }
                Ok(res) => {
                    log::info!("Created new database: {:?}", res);
                    Ok(res)
                }
            }
        } else {
            error!("Not running. Call start first");
            Ok("error".to_string())
        }
    }

    pub async fn drop_database(&mut self, name: String) -> anyhow::Result<()> {
        if self.running {
            log::trace!("Dropping database: {}", name);
            let mut lock = self.db_lock.lock().unwrap();
            match lock.drop_database(name).await {
                Err(e) => {
                    log::error!("Unable to drop database: {:?}", e.to_string());
                    Err(anyhow::anyhow!(e.to_string()))
                }
                Ok(_) => Ok(()),
            }
        } else {
            Ok(())
        }
    }

    pub async fn migration(
        &mut self,
        db_name: String,
        migrations_path_str: String,
    ) -> anyhow::Result<()> {
        if self.running {
            let mut db_lock = self.db_lock.lock().unwrap();
            log::info!("Calling migration on db_lock");
            let _ = db_lock.migration(db_name, &migrations_path_str);
            Ok(())
        } else {
            Ok(())
        }
    }

    pub async fn execute_sql(&mut self, sql: String) -> anyhow::Result<()> {
        if self.running {
            let mut db_lock = self.db_lock.lock().unwrap();
            log::debug!("System called stop on the db_lock");
            let _ = db_lock.execute_sql(sql, None).await?;
            Ok(())
        } else {
            Ok(())
        }
    }

    pub async fn stop(&mut self) -> anyhow::Result<bool> {
        // Incase we're not running, don't stop
        if self.running {
            let mut db_lock = self.db_lock.lock().unwrap();
            log::debug!("System called stop on the db_lock");
            match db_lock.stop().await {
                Err(e) => {
                    log::error!("Unable to stop database: {:?}", e.to_string());
                    Ok(false)
                    // bail!(e.to_string())
                }
                Ok(res) => {
                    self.running = false;
                    log::info!("DBLock stopped");
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
    pub async fn initialize(mut config: ConfigDatabase) -> anyhow::Result<System> {
        let _logger = logger::init_logging(None);

        let root_path = match config.root_path {
            None => match TempDir::new("db") {
                Ok(v) => String::from(v.path().to_str().unwrap()),
                Err(_) => ".".to_string(),
            },
            Some(v) => v,
        };
        config.root_path = Some(root_path);

        let inner = SystemInner::new(config).await;
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

    pub async fn execute_sql(&mut self, sql: String) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        Ok(inner.execute_sql(sql).await?)
    }

    pub async fn migration(
        &mut self,
        db_name: String,
        migrations_dir: String,
    ) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        Ok(inner.migration(db_name, migrations_dir).await?)
    }

    pub async fn create_new_db(&mut self, name: Option<String>) -> anyhow::Result<String> {
        let mut inner = self.inner.lock().unwrap();
        Ok(inner.create_new_db(name).await?)
    }

    pub async fn drop_database(&mut self, name: String) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        Ok(inner.drop_database(name).await?)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[tokio::test]
    async fn test_system_can_be_created() {
        let cd = ConfigDatabase::default();
        let system = System::initialize(cd).await;
        assert!(system.is_ok());
        let res = system.unwrap().start().await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_system_can_be_created_with_an_external_database() {
        let cd = ConfigDatabase {
            db_type: "External".to_string(),
            uri: "postgres://postgres:postgres@localhost:5432".to_string(),
            root_path: None,
            username: Some("postgres".to_string()),
            password: Some("postgres".to_string()),
            persistent: None,
            port: None,
            timeout: None,
            host: None,
        };
        // let id = start_docker_container().await.unwrap();
        let system = System::initialize(cd).await;
        assert!(system.is_ok());
        let mut system = Box::new(system.unwrap());
        let res = system.start().await;
        assert!(res.is_ok());
        let new_db = system.create_new_db(None).await;
        assert!(new_db.is_ok());
        println!("new_db: {:?}", new_db.unwrap());
        // let _ = stop_docker_container(id).await;
    }

    // const IMAGE: &str = "postgres";
    // async fn start_docker_container() -> anyhow::Result<String> {
    //     let docker = get_docker().unwrap();
    //     docker
    //         .create_image(
    //             Some(CreateImageOptions {
    //                 from_image: IMAGE,
    //                 ..Default::default()
    //             }),
    //             None,
    //             None,
    //         )
    //         .try_collect::<Vec<_>>()
    //         .await?;

    //     let alpine_config = bollard::container::Config {
    //         image: Some(IMAGE),
    //         tty: Some(false),
    //         ..Default::default()
    //     };

    //     let id: String = docker
    //         .create_container::<&str, &str>(None, alpine_config)
    //         .await?
    //         .id;
    //     docker.start_container::<String>(&id, None).await?;
    //     Ok(id)
    // }

    // async fn stop_docker_container(id: String) -> anyhow::Result<()> {
    //     let docker = get_docker().unwrap();

    //     docker
    //         .remove_container(
    //             &id,
    //             Some(RemoveContainerOptions {
    //                 force: true,
    //                 ..Default::default()
    //             }),
    //         )
    //         .await?;

    //     Ok(())
    // }

    // #[cfg(target_os = "linux")]
    // fn get_docker() -> Result<Docker, bollard::errors::Error> {
    //     Docker::connect_with_socket_defaults()
    // }
    // #[cfg(not(target_arch = "linux"))]
    // fn get_docker() -> Result<Docker, bollard::errors::Error> {
    //     use bollard::API_DEFAULT_VERSION;

    //     Docker::connect_with_local("/var/run/docker.sock", 120, API_DEFAULT_VERSION)
    // }
}
