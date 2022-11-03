use std::thread;

use crate::system::system::System;
use neon::{prelude::*, types::Deferred};
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;

type DbCallback = Box<dyn FnOnce(&mut System, &Channel, Deferred) + Send>;

// Return a global tokio runtime or create one if it doesn't exist.
// Throws a JavaScript exception if the `Runtime` fails to create.
fn runtime<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    static RUNTIME: OnceCell<Runtime> = OnceCell::new();

    RUNTIME.get_or_try_init(|| Runtime::new().or_else(|err| cx.throw_error(err.to_string())))
}

pub struct JSSystem {
    tx: tokio::sync::mpsc::Sender<SystemMessage>,
}

enum SystemMessage {
    Callback(Deferred, DbCallback),
    Close,
}

// Clean-up when Database is garbage collected, could go here
// but, it's not needed,
impl Finalize for JSSystem {}

impl JSSystem {
    pub fn new<'a, C>(cx: &mut C) -> anyhow::Result<Self>
    where
        C: Context<'a>,
    {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<SystemMessage>(128);

        // let rt = runtime(&mut cx);

        let channel = cx.channel();
        let (deferred, promise) = cx.promise();

        // TODO: read all files *.sql
        // let sql = fs::read_to_string(&sys_path).expect("Cannot read system path");

        tokio::spawn(async move {
            let mut system = System::initialize()
                .await
                .expect("Unable to initialize system");

            // rt.spawn(async move { system.run_loop().await });
            tokio::select! {
                    _res = async {
                        let _res = system.run_loop().await;
                        Ok::<_, anyhow::Error>(())
                     } => {}
                Some(message) = rx.recv() => {
                    let _ = match message {
                        SystemMessage::Callback(deferred, f) => {
                            f(&mut system, &channel, deferred);
                            Ok::<_, anyhow::Error>(())
                        },
                        SystemMessage::Close => {
                            // system.cleanup().await;
                            Ok::<_, anyhow::Error>(())
                        },
                    };
                }
                else => {}
            }
        });

        let res = Self { tx };

        Ok(res)
    }

    // Idiomatic rust would take an owned `self` to prevent use after close
    // However, it's not possible to prevent JavaScript from continuing to hold a closed database
    async fn async_close(&self) -> Result<(), tokio::sync::mpsc::error::SendError<SystemMessage>> {
        self.tx.send(SystemMessage::Close).await
    }

    pub fn close(cx: FunctionContext) -> JsResult<JsPromise> {
        // ) -> Result<(), tokio::sync::mpsc::error::SendError<SystemMessage>> {
        let (deferred, promise) = cx.promise();
        let channel = cx.channel();
        let sys = cx.this().downcast_or_throw::<JsBox<JSSystem>, _>(&mut cx)?;

        sys.async_close();

        // tokio::spawn(async move {
        //     let closed_result = sys.async_close().await;

        //     deferred.settle_with(&channel, move |mut cx| Ok(cx.undefined()))
        // });
        Ok(promise)
    }

    async fn async_send(
        &self,
        deferred: Deferred,
        callback: impl FnOnce(&mut System, &Channel, Deferred) + Send + 'static,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<SystemMessage>> {
        self.tx
            .send(SystemMessage::Callback(deferred, Box::new(callback)))
            .await
    }

    pub fn send(cx: FunctionContext) -> JsResult<JsPromise> {
        // ) -> Result<(), tokio::sync::mpsc::error::SendError<SystemMessage>> {
        let (deferred, promise) = cx.promise();
        let channel = cx.channel();
        let sys = cx.this().downcast_or_throw::<JsBox<JSSystem>, _>(&mut cx)?;

        tokio::spawn(async move {
            sys.async_send(deferred, move |sys, channel, deferred| {
                deferred.settle_with(channel, move |mut cx| Ok(cx.boolean(true)));
            })
            .await
            // .into_rejection(&mut cx);
        });

        // tokio::spawn(async move {
        //     let closed_result = sys.async_close().await;

        //     deferred.settle_with(&channel, move |mut cx| Ok(cx.undefined()))
        // });
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

impl JSSystem {
    pub fn new_system<'a>(mut cx: FunctionContext) -> JsResult<JsBox<JSSystem>> {
        let system = JSSystem::new(&mut cx).or_else(|err| cx.throw_error(err.to_string()))?;
        //.or_else(|err| cx.throw_error(err.to_string()))?;
        Ok(cx.boxed(system))
    }

    pub fn close_system<'a>(mut cx: FunctionContext<'a>) -> JsResult<JsUndefined> {
        let sys = cx.this().downcast_or_throw::<JsBox<JSSystem>, _>(&mut cx)?;
        sys.close(cx)?
    }
}
