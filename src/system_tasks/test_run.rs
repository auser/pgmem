use std::borrow::BorrowMut;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::task::JoinHandle;
use tracing::*;
use warp::Filter;

use crate::system::{
    db::{Migration, Migrations},
    system::{QuitOnError, System, SystemPlugin},
};

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
        let db_pool = system.db_pool.clone();
        let do_quit = system.quit.clone();
        let mut on_quit = do_quit.subscribe();
        // let mut on_quit = system.quit.subscribe();
        let handle = tokio::task::spawn(async move {
            info!("TestRun Handler task has launched");
            MIGRATIONS
                .migrate_up(&db_pool)
                .await
                .quit_on_err(&do_quit)?;

            let routes = warp::any().map(|| "Hello From Warp!");
            let svc = warp::service(routes);
            let make_svc =
                warp::hyper::service::make_service_fn(
                    move |_| async move { Ok::<_, Infallible>(svc) },
                );

            info!("Launching listener");
            if let Err(error) = warp::hyper::Server::bind(&([127, 0, 0, 1], 3030).into())
                .serve(make_svc)
                .with_graceful_shutdown(async {
                    let _ = on_quit.recv().await;
                    info!("Quitting")
                })
                .await
            {
                error!("Listener shutdown poorly: {:#?}", error);
            } else {
                info!("Shut down correctly");
            }
            let _ = do_quit.send(());

            // let pg_pool = MIGRATIONS
            // 	.migrate_up_and_get_pool(&registered_data, Duration::from_secs(60))
            // 	.await
            // 	.quit_on_err(&do_quit)?;
            // tokio::select! {
            //   msg = recv_msg.recv() => {
            //     println!("msg: {:?}", msg);
            //   }
            //   _ = recv_quit.recv() => {
            //     println!("Received quit");
            //   }
            // }
            Ok(())
        });
        Some(handle)
    }
}

const MIGRATIONS: Migrations = Migrations::new("TestRun", &[]);
