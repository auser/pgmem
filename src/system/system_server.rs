use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use neon::event::Channel;
use neon::{prelude::*, types::Deferred};
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use tracing::*;

use super::system::System;
use super::utils::{block_on, runtime};

// pub type SystemCallback =
//     Box<dyn FnOnce(&mut broadcast::Sender<String>, &Channel, Deferred) + Send>;
pub type SystemCallback = Box<dyn FnOnce(&mut Arc<Mutex<System>>, &Channel, Deferred) + Send>;

#[allow(unused)]
#[derive()]
pub enum SystemMessage {
    Callback(Deferred, SystemCallback),
    Close,
}

impl Debug for SystemMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemMessage::Callback(_, _) => write!(f, "Callback"),
            SystemMessage::Close => write!(f, "Close"),
        }
    }
}

#[derive(Debug)]
pub struct SystemServer {
    handle: JoinHandle<()>,
    tx: tokio::sync::mpsc::Sender<SystemMessage>,
}

impl Finalize for SystemServer {}

impl SystemServer {
    fn new<'a, C>(cx: &mut C, root_dir: String) -> anyhow::Result<Self>
    where
        C: Context<'a>,
    {
        // First get the database -- make this configurable, maybe?
        // let db = DBType::default();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<SystemMessage>(32);
        let channel = cx.channel();

        let rt = runtime(cx).unwrap(); //.unwrap_or_else(|err| anyhow::anyhow!(err.to_string()));
        let system = rt.block_on(async move { System::initialize(root_dir).await.unwrap() });
        // We need a channel for communication back to JS
        let mut sys = Arc::new(Mutex::new(system));

        // system.run_loop().await;

        let handle = rt.spawn(async move {
            while let Some(msg) = rx.recv().await {
                match msg {
                    SystemMessage::Callback(deferred, f) => {
                        f(&mut sys, &channel, deferred);
                    }
                    SystemMessage::Close => break,
                }
            }
        });

        Ok(Self { tx, handle })
    }

    // Idiomatic rust would take an owned `self` to prevent use after close
    // However, it's not possible to prevent JavaScript from continuing to hold a closed database
    #[allow(unused)]
    fn close(&self) -> Result<(), tokio::sync::mpsc::error::SendError<SystemMessage>> {
        block_on(self.tx.send(SystemMessage::Close))
    }

    fn send(
        &self,
        deferred: Deferred,
        callback: impl FnOnce(&mut Arc<Mutex<System>>, &Channel, Deferred) + Send + 'static,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<SystemMessage>> {
        block_on(
            self.tx
                .send(SystemMessage::Callback(deferred, Box::new(callback))),
        )
    }
}

impl Drop for SystemServer {
    fn drop(&mut self) {
        let _ = self.close();
        self.handle.abort();
    }
}

impl SystemServer {
    pub fn js_new(mut cx: FunctionContext) -> JsResult<JsBox<SystemServer>> {
        let root_dir = cx.argument::<JsString>(0)?.value(&mut cx);
        let system_server =
            SystemServer::new(&mut cx, root_dir).or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.boxed(system_server))
    }

    pub fn js_start(mut cx: FunctionContext) -> JsResult<JsPromise> {
        // let callback = cx.argument::<JsFunction>(1)?.root(&mut cx);

        let (deferred, promise) = cx.promise();
        // let system_server = cx.argument::<JsBox<SystemServer>>(0)?;
        // Get the `this` value as a `JsBox<Database>`
        let system_server = cx
            .this()
            .downcast_or_throw::<JsBox<SystemServer>, _>(&mut cx)?;

        system_server
            .send(deferred, move |sys, channel, deferred| {
                // Local this
                let mut sys = sys.lock().unwrap();
                let handle = Handle::current();
                let _ = handle.enter();
                let res = futures::executor::block_on(sys.start());

                info!("In start: {:?}", res);
                deferred.settle_with(channel, move |mut cx| -> JsResult<JsBoolean> {
                    Ok(cx.boolean(true))
                });
            })
            .into_rejection(&mut cx)?;

        Ok(promise)
    }

    pub fn js_stop(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let (deferred, promise) = cx.promise();
        let system_server = cx
            .this()
            .downcast_or_throw::<JsBox<SystemServer>, _>(&mut cx)?;

        system_server
            .send(deferred, move |sys, channel, deferred| {
                // Local this
                let mut sys = sys.lock().unwrap();
                let handle = Handle::current();
                let _ = handle.enter();
                let res = futures::executor::block_on(sys.stop());

                info!("In start: {:?}", res);
                deferred.settle_with(channel, move |mut cx| -> JsResult<JsBoolean> {
                    Ok(cx.boolean(true))
                });
            })
            .into_rejection(&mut cx)?;

        Ok(promise)
    }

    pub fn js_create_new_db(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let (deferred, promise) = cx.promise();
        let system_server = cx
            .this()
            .downcast_or_throw::<JsBox<SystemServer>, _>(&mut cx)?;

        system_server
            .send(deferred, move |sys, channel, deferred| {
                let mut sys = sys.lock().unwrap();
                let handle = Handle::current();
                let _ = handle.enter();
                let res = futures::executor::block_on(sys.create_new_db(None));

                deferred.settle_with(channel, move |mut cx| -> JsResult<JsString> {
                    match res {
                        Err(e) => Ok(cx.string(e.to_string())),
                        Ok(conn_url) => Ok(cx.string(conn_url)),
                    }
                });
            })
            .into_rejection(&mut cx)?;

        Ok(promise)
    }

    pub fn js_drop_database(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let (deferred, promise) = cx.promise();
        let system_server = cx
            .this()
            .downcast_or_throw::<JsBox<SystemServer>, _>(&mut cx)?;

        let db_name = cx.argument::<JsString>(0)?.value(&mut cx);

        system_server
            .send(deferred, move |sys, channel, deferred| {
                let mut sys = sys.lock().unwrap();
                let handle = Handle::current();
                let _ = handle.enter();
                println!("Dropping database");
                let res = futures::executor::block_on(sys.drop_database(db_name));
                println!("Dropped database");

                deferred.settle_with(channel, move |mut cx| -> JsResult<JsBoolean> {
                    match res {
                        Err(_e) => Ok(cx.boolean(false)),
                        Ok(_) => Ok(cx.boolean(true)),
                    }
                });
            })
            .into_rejection(&mut cx)?;

        Ok(promise)
    }
}

trait SendResultExt {
    // Sending a query closure to execute may fail if the channel has been closed.
    // This method converts the failure into a promise rejection.
    fn into_rejection<'a, C: Context<'a>>(self, cx: &mut C) -> NeonResult<()>;
}

impl SendResultExt for Result<(), tokio::sync::mpsc::error::SendError<SystemMessage>> {
    fn into_rejection<'a, C: Context<'a>>(self, cx: &mut C) -> NeonResult<()> {
        self.or_else(|err| {
            let msg = err.to_string();

            match err.0 {
                SystemMessage::Callback(deferred, _) => {
                    let err = cx.error(msg)?;
                    deferred.reject(cx, err);
                    Ok(())
                }
                SystemMessage::Close => cx.throw_error("Expected SystemMessage::Callback"),
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_system_can_be_created() {}
}
