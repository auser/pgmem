use tokio::task::JoinHandle;
use tracing::*;

use crate::system::system::{System, SystemPlugin};

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct Daemon {
    headless: bool,
}

impl Daemon {
    pub fn new(headless: bool) -> Self {
        Self { headless }
    }

    pub fn setup_handler(&self, system: &System) -> JoinHandle<anyhow::Result<()>> {
        let do_quit = system.quit.clone();
        let mut on_quit = system.quit.subscribe();
        tokio::task::spawn(async move {
            info!("Daemon task has launched");
            loop {
                #[cfg(target_os = "linux")]
                let mut hangup =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())
                        .context("failed registering hangup signal stream")?;
                #[cfg(target_os = "linux")]
                let mut interrupt =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
                        .context("failed registering interrupt signal stream")?;
                #[cfg(target_os = "linux")]
                let mut quit = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::quit())
                    .context("failed registering quit signal stream")?;
                #[cfg(target_os = "linux")]
                let mut terminate =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                        .context("failed registering terminate signal stream")?;
                #[cfg(target_os = "linux")]
                let do_break = tokio::select! {
                    _ = hangup.recv() => {
                        if headless {
                            info!("Hangup requested, daemon mode ignores it");
                            false
                        } else {
                            info!("Hangup requested, cleanly exiting");
                            let _ = do_quit.send(());
                            true
                        }
                    }
                    _ = interrupt.recv() => {
                        info!("Interrupt signal received, cleanly exiting...");
                        let _ = do_quit.send(());
                        true
                    }
                    _ = quit.recv() => {
                        info!("Quit signal received, cleanly exiting...");
                        let _ = do_quit.send(());
                        true
                    }
                    _ = terminate.recv() => {
                        info!("Terminate signal received, cleanly exiting...");
                        let _ = do_quit.send(());
                        true
                    }
                    _ = on_quit.recv() => {
                        info!("Signal handler has received a quit request, exiting...");
                        true
                    }
                };

                #[cfg(not(target_os = "linux"))]
                let do_break = tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        info!("Ctrl+C signal received, cleanly exiting...");
                        let _ = do_quit.send(());
                        true
                    }
                    _ = on_quit.recv() => {
                        info!("Signal handler has received a quit request, cleanly exiting...");
                        true
                    }
                };

                if do_break {
                    break;
                }
            }

            Ok(())
        })
    }
}

#[typetag::serde]
impl SystemPlugin for Daemon {
    fn spawn(&self, system: &System) -> Option<JoinHandle<anyhow::Result<()>>> {
        let _headless = self.headless;
        let do_quit = system.quit.clone();
        let mut on_quit = system.quit.subscribe();

        let handle = self.setup_handler(system);
        Some(handle)
    }
}
