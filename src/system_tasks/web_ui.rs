use crate::database::Migrations;
use crate::system::{QuitOnError, System, SystemPlugin};
use std::convert::Infallible;
use tokio::task::JoinHandle;
use tracing::*;

#[allow(clippy::upper_case_acronyms)]
#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct WebUiRocket {
	enabled: bool,
	url_root: String,
}

impl WebUiRocket {
	pub fn new(enabled: bool) -> Self {
		Self {
			enabled,
			url_root: "/".to_owned(),
		}
	}

	pub fn url_root(self, url_root: impl Into<String>) -> Self {
		Self {
			url_root: url_root.into(),
			..self
		}
	}
}

#[typetag::serde]
impl SystemPlugin for WebUiRocket {
	fn spawn(&self, system: &System) -> Option<JoinHandle<anyhow::Result<()>>> {
		if !self.enabled {
			return None;
		}
		let url_root = self.url_root.clone();
		let registered_data = system.registered_data.clone();
		let db_pool = system.db_pool.clone();
		let do_quit = system.quit.clone();
		let mut on_quit = system.quit.subscribe();
		let handle = tokio::spawn(async move {
			info!("Migrating web UI, waiting up to 60 seconds for the database pool to initialize");
			MIGRATIONS.migrate_up(&db_pool).await?;
			info!("Web UI now building routes");
			let routes = warp::any().map(|| "Hello From Warp!");
			// Convert warp into a Tower service
			let svc = warp::service(routes);
			// Use hyper to create the service on demand
			let make_svc =
				warp::hyper::service::make_service_fn(
					move |_| async move { Ok::<_, Infallible>(svc) },
				);
			info!("Web UI now launching the web service");
			// And start the hyper Tower service root
			if let Err(error) = warp::hyper::Server::bind(&([127, 0, 0, 1], 3030).into())
				.serve(make_svc)
				.with_graceful_shutdown(async {
					let _ = on_quit.recv().await;
					info!("Quit requested, safely shutting down the Web UI");
				})
				.await
			{
				error!("Web UI shut down improperly: {:#?}", error);
			} else {
				info!("Web UI shut down successfully");
			}
			let _ = do_quit.send(());
			Ok(())
		});
		Some(handle)
	}
}

const MIGRATIONS: Migrations = Migrations::new("WebUI", &[]);
