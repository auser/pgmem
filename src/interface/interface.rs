use futures::{future::abortable, stream::AbortHandle};
use neon::{prelude::*, types::Deferred};
use once_cell::sync::OnceCell;
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc;

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
        // let channel = cx.channel();

        // We have a few things going on here, we need to start up the database
        // Next we have to run the execution loop for any tasks in the queue
        // We need to listen on the communication channel to see if there's anything
        // we need to execute
        let system = block_on(async move { System::initialize().await.unwrap() });

        let quit = system.quit.clone();
        let manager_system = Arc::new(Mutex::new(system));

        let manager = async move {
            let mut on_quit = quit.subscribe();
            println!("In manager");
            tokio::select! {
                _ = async {
                    loop {
                        let msg = on_quit.recv().await;
                        println!("msg: {:?}", msg);
                        // let _ = manager_system.lock().unwrap().run_loop().await;
                    }
                } => {
                    // Break out
                }
                msg = rx.recv() => {
                    println!("Something sent on the tx channel: {:#?}", msg);
                    match msg {
                        Some(SystemMessage::Close) => {
                            quit.send(());
                        },
                        None => {},
                        _a => {
                            unimplemented!();
                        }
                    }
                }
            };
            // let res = system.cleanup().await;
            // println!("res: {:#?}", res);
            // drop(system);
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
            handle: RefCell::new(Some(handle)),
            tx,
            system: manager_system.clone(),
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
        callback: impl FnOnce(&Channel, Deferred) + Send + 'static,
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
    pub fn js_stop_db(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let server = cx.argument::<JsBox<SystemServer>>(0)?;

        println!("js_stop_db called");
        let _ = block_on(server.tx.send(SystemMessage::Close));
        println!("js_stop_db called");

        Ok(cx.undefined())
    }
}
