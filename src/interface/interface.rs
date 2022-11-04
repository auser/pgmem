use futures::{future::abortable, stream::AbortHandle};
use neon::{prelude::*, types::Deferred};
use once_cell::sync::OnceCell;
use std::{
    cell::RefCell,
    sync::{mpsc, Arc, Mutex},
};

use crate::system::System;

use super::{
    system_server::{Function, SystemServer},
    utils::{block_on, runtime},
};

pub fn hello(mut cx: FunctionContext) -> JsResult<JsString> {
    Ok(cx.string("hello node"))
}

type SystemCallback = Box<dyn FnOnce(&Channel, Deferred) + Send>;

#[allow(unused)]
#[derive()]
enum SystemMessage {
    Callback(Deferred, SystemCallback),
    Close,
}

pub struct JSSystem {
    tx: mpsc::Sender<SystemMessage>,
    // db_uri: String,
    // pub system: System,
}
impl Finalize for JSSystem {}

impl JSSystem {
    pub fn js_start_db(mut cx: FunctionContext) -> JsResult<JsBox<SystemServer>> {
        // Get a channel to chat over callbacks
        // let func = Function {
        //     channel: cx.channel(),
        //     callback: Arc::new(cx.argument::<JsFunction>(0)?.root(&mut cx)),
        // };
        let (tx, mut rx) = mpsc::channel::<SystemMessage>();
        let channel = cx.channel();

        // We have a few things going on here, we need to start up the database
        // Next we have to run the execution loop for any tasks in the queue
        // We need to listen on the communication channel to see if there's anything
        // we need to execute
        let manager = async move {
            println!("In manager");
            let system = System::initialize().await.unwrap();
        };

        let (abortable, handle) = abortable(manager);
        let rt = match runtime(&mut cx) {
            Ok(rt) => rt,
            Err(e) => {
                return cx.throw_error(format!("Error getting runtime: {:#?}", e.to_string()))
            }
        };

        let _handle = rt.spawn(abortable);
        let server = SystemServer {
            handle: RefCell::new(Some(handle)),
        };

        Ok(cx.boxed(server))
    }

    pub fn js_stop_db(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let server = cx.argument::<JsBox<SystemServer>>(0)?;

        *server.handle.borrow_mut() = None;
        Ok(cx.undefined())
    }
}

// JS interfaces
