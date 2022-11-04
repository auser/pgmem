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
    system_server::{Function, SystemMessage, SystemServer},
    utils::{block_on, runtime},
};

pub fn hello(mut cx: FunctionContext) -> JsResult<JsString> {
    Ok(cx.string("hello node"))
}

#[derive()]
pub struct JSSystem {
    tx: mpsc::Sender<SystemMessage>,
    // db_uri: String,
    pub handle: RefCell<Option<AbortHandle>>,
    // pub system: Arc<Mutex<System>>,
}
impl Finalize for JSSystem {}

impl JSSystem {
    // pub fn js_start_db(mut cx: FunctionContext) -> JsResult<JsBox<SystemServer>> {
    fn start_db<'a, C>(cx: &mut C) -> anyhow::Result<Self>
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
        let mut on_quit = system.quit.subscribe();

        let manager = async move {
            tokio::select! {
                _ = async {
                    loop {
                        let msg = on_quit.recv().await;
                        println!("MSG: {:#?}", msg);
                        break
                        // let sys = &system.lock().unwrap();
                        // sys.run_loop().await;
                    }
                } => {
                    // Break out
                    println!("Message was received");
                }
                _ = rx.recv() => {
                    println!("Something sent on the tx channel")
                }
            }
            // let res = system.lock().unwrap().cleanup().await;
            // println!("res: {:#?}", res);
            // drop(system);
        };

        let (abortable, handle) = abortable(manager);
        let rt = match runtime(cx) {
            Ok(rt) => rt,
            Err(e) => return Err(anyhow::anyhow!(e.to_string())),
        };

        let _handle = rt.spawn(abortable);
        let server = Self {
            handle: RefCell::new(Some(handle)),
            tx,
            // system: Arc::new(Mutex::new(system)),
        };
        // let server = SystemServer {
        //     tx,
        //     system,
        // };

        Ok(server)
    }
}

impl JSSystem {
    pub fn js_start_db(mut cx: FunctionContext) -> JsResult<JsBox<JSSystem>> {
        let jssystem =
            JSSystem::start_db(&mut cx).or_else(|err| cx.throw_error(err.to_string()))?;
        println!("SYSTEM SERVER: {:#?}", jssystem.handle);
        Ok(cx.boxed(jssystem))
    }

    pub fn js_stop_db(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        // let server = cx.argument::<JsBox<SystemServer>>(0)?;
        let server = cx.this().downcast_or_throw::<JsBox<JSSystem>, _>(&mut cx)?;

        println!("js_stop_db called");
        let tx = &server.tx;
        let rt = runtime(&mut cx).unwrap(); // TODO: test for errors
        let _ = rt.block_on(async move { tx.send(SystemMessage::Close) });
        println!("js_stop_db called");

        Ok(cx.undefined())
    }
}

// JS interfaces
