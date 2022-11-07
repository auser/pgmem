use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::bail;
use futures::executor::ThreadPool;
use tokio::{runtime::Builder, sync::broadcast};
use tracing::*;

use super::{
    config::ConfigDatabase,
    db::{DBLock, DbPool, DB},
    logger,
    system_server::SystemMessage,
};

#[derive(Debug, Clone)]
pub enum SystemAction {
    Start,
    Stop,
}

pub struct SystemInner {
    pub db_lock: Arc<Mutex<DB>>,
    pub running: bool,
    pub msg: broadcast::Sender<SystemAction>,
}

impl SystemInner {
    pub async fn new() -> Self {
        let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let (msg, _recv_msg) = broadcast::channel(10);

        // Make configurable?
        let db_lock = Arc::new(Mutex::new(
            DB::new_embedded(ConfigDatabase::default()).await,
        ));
        let running = false;
        Self {
            db_lock,
            running,
            msg,
        }
    }

    async fn process(&mut self, msg: SystemAction) -> anyhow::Result<bool> {
        match msg {
            SystemAction::Start => self.start().await,
            SystemAction::Stop => self.stop().await,
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
        if !self.running {
            self.start().await?;
        }
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

pub struct System {
    inner: Arc<Mutex<SystemInner>>,
}

impl System {
    pub async fn initialize() -> anyhow::Result<System> {
        let _ = logger::init_logging(None);

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

    pub async fn run_loop(self) -> anyhow::Result<bool> {
        let mut inner = self.inner.lock().unwrap();
        let msg = inner.msg.clone();
        let mut on_msg = msg.subscribe();

        let runtime = Builder::new_multi_thread()
            .worker_threads(5)
            .thread_name("worker-thread")
            .thread_stack_size(3 * 1024 * 1024)
            .build()
            .unwrap();

        runtime.block_on(async move {
            while let msg = on_msg.recv().await {
                let _res = match msg {
                    Ok(msg) => match msg {
                        SystemAction::Stop => Ok(inner.stop().await),
                        SystemAction::Start => Ok(inner.start().await),
                    },
                    Err(e) => {
                        error!("Error in run_loop: {:?}", e.to_string());
                        Err(e)
                    }
                };
            }
        });
        Ok(true)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_system_can_be_created() {}
}
