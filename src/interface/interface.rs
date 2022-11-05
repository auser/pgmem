use futures::future::abortable;
use neon::{prelude::*, types::Deferred};
use std::cell::RefCell;
use tokio::sync::mpsc;
use tracing::*;

use crate::system::System;

use super::{
    system_server::{SystemMessage, SystemServer},
    utils::{block_on, runtime},
};

pub fn hello(mut cx: FunctionContext) -> JsResult<JsString> {
    Ok(cx.string("hello node"))
}

impl SystemServer {
    fn new<'a, C>(cx: &mut C) -> anyhow::Result<Self>
    where
        C: Context<'a>,
    {
        // Get a channel to chat over callbacks
        // let func = Function {
        //     channel: cx.channel(),
        //     callback: Arc::new(cx.argument::<JsFunction>(0)?.root(&mut cx)),
        // };
        let (tx, mut rx) = mpsc::channel::<SystemMessage>(32);
        let channel = cx.channel();

        let mut system = block_on(async move { System::initialize().await.unwrap() });

        let quit = system.quit.clone();
        // let mut system = Arc::new(Mutex::new(system));

        let manager = async move {
            let mut on_quit = quit.subscribe();
            tokio::select! {
                _ = async {
                    loop {
                        while let Some(msg) = rx.recv().await {
                            match msg {
                                SystemMessage::Callback(deferred, f) => {
                                    f(&mut system, &channel, deferred);
                                }
                                SystemMessage::Close => {
                                    let _ = quit.send(());
                                }
                            }
                        }
                    }
                }=> {
                    println!("Something sent on the tx channel");
                }
                _ = on_quit.recv()  => {
                    // Break out
                    println!("Got a message in the loop");
                }
            };
            let _res = system.cleanup().await;
        };

        let (abortable, handle) = abortable(manager);
        let rt = match runtime(cx) {
            Ok(rt) => rt,
            Err(e) => {
                return Err(anyhow::anyhow!(format!(
                    "Error getting runtime: {:#?}",
                    e.to_string()
                )))
            }
        };

        let _handle = rt.spawn(abortable);
        Ok(Self {
            // name: "Bob".to_string(),
            handle: RefCell::new(Some(handle)),
            tx,
            // system: system.clone(),
        })
    }
    // Idiomatic rust would take an owned `self` to prevent use after close
    // However, it's not possible to prevent JavaScript from continuing to hold a closed database
    fn close(&self) -> Result<(), mpsc::error::SendError<SystemMessage>> {
        block_on(self.tx.send(SystemMessage::Close))
    }

    fn send(
        &self,
        deferred: Deferred,
        callback: impl FnOnce(&mut System, &Channel, Deferred) + Send + 'static,
    ) -> Result<(), mpsc::error::SendError<SystemMessage>> {
        block_on(
            self.tx
                .send(SystemMessage::Callback(deferred, Box::new(callback))),
        )
    }
}

impl SystemServer {
    pub fn js_start_db(mut cx: FunctionContext) -> JsResult<JsBox<SystemServer>> {
        let system_server =
            SystemServer::new(&mut cx).or_else(|err| cx.throw_error(err.to_string()))?;
        Ok(cx.boxed(system_server))
    }

    pub fn js_send(mut cx: FunctionContext) -> JsResult<JsString> {
        Ok(cx.string("Hello"))
    }

    pub fn js_stop_db(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let handle = match cx.this().downcast::<JsBox<SystemServer>, _>(&mut cx) {
            Err(_) => cx.argument::<JsBox<SystemServer>>(0)?,
            Ok(v) => v,
        };

        let _res = handle.close();

        Ok(cx.undefined())
    }

    pub fn js_new_database(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let handle = cx
            .this()
            .downcast_or_throw::<JsBox<SystemServer>, _>(&mut cx)?;

        let (deferred, promise) = cx.promise();
        info!("js_new_database");

        handle
            .send(deferred, move |sys, channel, deferred| {
                println!("msg: {:#?}", sys.msg);
                let res = sys.msg.send("newDatabase".to_string());
                info!("js_new_database: {:#?}", res);
                // TODO: wait for the return
                deferred.settle_with(channel, move |mut cx| -> JsResult<JsValue> {
                    info!("settle_with");
                    Ok(cx.string("WILL BE A URL").upcast())
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

impl SendResultExt for Result<(), mpsc::error::SendError<SystemMessage>> {
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
