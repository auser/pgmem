use glob::glob;
use std::{
    convert::Infallible,
    fs::File,
    io::{BufRead, BufReader, Read},
};
use tokio::task::JoinHandle;
use tracing::*;
use warp::Filter;

use crate::system::system::{QuitOnError, System, SystemPlugin};

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct TestRuns {}

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct TestRun {
    enabled: bool,
    data_path: String,
}

impl TestRun {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            data_path: "test_run".to_owned(),
            // connections: Default::default(),
        }
    }

    #[allow(dead_code)]
    pub fn test_run_data(self, data_path: String) -> Self {
        Self { data_path, ..self }
    }
}

#[typetag::serde]
impl SystemPlugin for TestRun {
    fn spawn(&self, system: &System) -> Option<JoinHandle<anyhow::Result<()>>> {
        if !self.enabled {
            return None;
        }
        info!("SystemPlugin for TestRun");
        let db_pool = system.db_pool.clone();
        let do_quit = system.quit.clone();
        let mut on_quit = do_quit.subscribe();
        let do_msg = system.msg.clone();
        let mut on_msg = do_msg.subscribe();

        // Read all the sql files
        let mut sql = String::new();
        let pattern = String::from(system.root_path.join("**/*.sql").to_str().unwrap());
        info!("Looking in directory: {:?}", pattern);
        for entry in glob(pattern.as_str()).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    info!("Found path: {:#?}", path);
                    match File::open(path) {
                        Err(e) => {
                            error!("Unable to open {:?}", e);
                        }
                        Ok(mut file) => {
                            let mut contents = String::new();
                            file.read_to_string(&mut contents)
                                .expect("Unable to read file");
                            sql.push_str(contents.as_str());
                            sql.push_str(&format!("\n\n----\n\n"));
                        }
                    };
                }
                Err(e) => {
                    error!("Unable to read file: {:?}", e);
                }
            }
        }

        info!("SQL to run everytime: {:#?}", sql);

        // let mut on_quit = system.quit.subscribe();
        let handle = tokio::task::spawn(async move {
            info!("TestRun Handler task has launched");

            // let routes = warp::any().map(|| "Hello From Warp!");
            // let svc = warp::service(routes);
            // let make_svc =
            //     warp::hyper::service::make_service_fn(
            //         move |_| async move { Ok::<_, Infallible>(svc) },
            //     );

            tokio::select! {
                Ok(msg) = on_msg.recv() => {
                    info!("Received message: {:?}", msg);
                }
                else => {
                    debug!("Received something else in tokio::select!");
                }
            }

            // info!("Launching listener");
            // if let Err(error) = warp::hyper::Server::bind(&([127, 0, 0, 1], 3030).into())
            //     .serve(make_svc)
            //     .with_graceful_shutdown(async {
            //         let _ = on_quit.recv().await;
            //         info!("Quitting")
            //     })
            //     .await
            // {
            //     error!("Listener shutdown poorly: {:#?}", error);
            // } else {
            //     info!("Shut down correctly");
            // }
            let _ = do_quit.send(());

            Ok(())
        });
        Some(handle)
    }
}
