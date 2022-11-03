use neon::{prelude::*, types::Deferred};
use once_cell::sync::OnceCell;
use std::future::Future;
use tokio::runtime::{Handle, Runtime};
use tokio::signal;
use tokio::sync::mpsc::channel;
use tracing::*;

use crate::system::system::System;

type SystemCallback = Box<dyn FnOnce(&Channel, Deferred) + Send>;

static RUNTIME: OnceCell<Runtime> = OnceCell::new();
// Return a global tokio runtime or create one if it doesn't exist.
// Throws a JavaScript exception if the `Runtime` fails to create.
fn runtime<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    // Runtime::new()
    RUNTIME.get_or_try_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .or_else(|err| cx.throw_error(err.to_string()))
    })
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    // let rt = tokio::runtime::Runtime::new().expect("could not start tokio rt");
    // let handle = Handle::current();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("could not create runtime");
    rt.block_on(future)
}

pub struct JSSystem {
    tx: tokio::sync::mpsc::Sender<SystemMessage>,
    db_uri: String,
    // pub system: System,
}
impl Finalize for JSSystem {}

#[allow(unused)]
#[derive()]
enum SystemMessage {
    Callback(Deferred, SystemCallback),
    Close,
}

impl JSSystem {
    pub fn new<'a, C>(cx: &mut C) -> Self
    where
        C: Context<'a>,
    {
        let (tx, mut rx) = channel::<SystemMessage>(1);
        // let mut system = block_on(async {
        //     // let channel = cx.channel();
        //     // // let mut do_msg
        //     // tokio::select! {
        //     // _ = do_quit.recv() => {
        //     //     info!("Received a kill from somewhere in the app");
        //     // }
        //     //     _ = async {
        //     //         let message = rx.recv().await.unwrap();
        //     //         let _ = match message {
        //     //             SystemMessage::Callback(deferred, f) => {
        //     //                 f(&channel, deferred);
        //     //                 Ok::<_, anyhow::Error>(())
        //     //             },
        //     //             SystemMessage::Close => {
        //     //                 // system.cleanup().await;
        //     //                 Ok::<_, anyhow::Error>(())
        //     //             },
        //     //         };
        //     //     }=> {}
        //     //     else => {
        //     //         println!("Got something else?");
        //     //     }
        //     // }
        //     // let _res = sys.cleanup().await;
        //     sys
        // });

        // let handle = Handle::current();
        let mut system = block_on(async {
            let mut sys = System::initialize().await.unwrap();
            sys
        });
        let mut do_quit = system.quit.subscribe();
        let mut do_msg = system.msg.subscribe();
        let db_uri = String::from(system.as_uri().unwrap());
        let handle = match tokio::runtime::Handle::try_current() {
            Err(_) => runtime(cx).unwrap().handle(),
            Ok(handle) => &handle,
        };
        handle.spawn(async move {
            let _ = system.run_loop().await;
            info!("run loop is running");
        });
        handle.block_on(async move {
            tokio::select! {
                _ = async {
                    loop {
                        if let Ok(msg) = do_msg.recv().await {
                            info!("Received a message: {:?}", msg);
                        }
                    }
                } => {}
                _ = do_quit.recv() => {}
            }
        });
        // rt.spawn(async move {
        //     let _ = system.run_loop().await;
        // });
        // let handle = rt.spawn(async move {
        //     let _ = system.run_loop();
        // });
        // let _ = rt.block_on(handle);

        // rt.spawn(async move {
        //     let _ = system.run_loop().await;
        // });

        Self { db_uri, tx }
    }

    pub fn db_uri(&self) -> &String {
        &self.db_uri
    }

    // Idiomatic rust would take an owned `self` to prevent use after close
    // However, it's not possible to prevent JavaScript from continuing to hold a closed database
    pub fn close(&self) -> anyhow::Result<()> {
        let _ = &self.tx.send(SystemMessage::Close);
        Ok(())
    }

    pub fn send(
        &self,
        deferred: Deferred,
        callback: impl FnOnce(&Channel, Deferred) + Send + 'static,
    ) -> anyhow::Result<()> {
        let _ = self
            .tx
            .send(SystemMessage::Callback(deferred, Box::new(callback)));
        Ok(())
    }
}

impl JSSystem {
    pub fn js_new(mut cx: FunctionContext) -> JsResult<JsBox<JSSystem>> {
        let sys = JSSystem::new(&mut cx);
        Ok(cx.boxed(sys))
    }

    pub fn js_db_uri(mut cx: FunctionContext) -> JsResult<JsString> {
        let sys = cx.this().downcast_or_throw::<JsBox<JSSystem>, _>(&mut cx)?;
        let uri = sys.db_uri();
        Ok(cx.string(uri))
    }

    pub fn js_handle_request(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let req = cx.argument::<JsString>(0)?.value(&mut cx);
        // let value: serde_json::Value = serde_json::from_str(&req).unwrap();
        // let request = serde_json::from_value::<SystemMessage>(value).unwrap();
        // let sys = cx.this().downcast_or_throw::<JsBox<JSSystem>, _>(&mut cx)?;

        // Boilerplate for Neon/Promises/Tokio
        let rt = runtime(&mut cx)?;
        let channel = cx.channel();
        let (deferred, promise) = cx.promise();
        // hack: don't even use a promise straight up block
        let a = block_on(async move {
            println!("Got a thing: {:?}", req);
            // let result = sys.execute(request).await;
            ()
        });
        rt.spawn(async move {
            // Perform the actual work in an sync block
            // let result = instance.api.execute(request).await;
            deferred.settle_with(&channel, move |mut cx| {
                // Ok(cx.string("1234"))
                Ok(cx.string(serde_json::to_string(&a).unwrap()))
            });
        });
        // Return our promise initially
        Ok(promise)
    }
}
