use neon::prelude::*;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::future::Future;
use tokio::runtime::Runtime;
use tokio::signal;
use tokio::sync::mpsc::channel;

use crate::system::system::System;

#[derive(Debug, Serialize, Deserialize)]
pub struct JSSystemRequest {}

static RUNTIME: OnceCell<Runtime> = OnceCell::new();
// Return a global tokio runtime or create one if it doesn't exist.
// Throws a JavaScript exception if the `Runtime` fails to create.
fn runtime<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    RUNTIME.get_or_try_init(|| Runtime::new().or_else(|err| cx.throw_error(err.to_string())))
}

fn reset<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    RUNTIME.set(
        Runtime::new()
            .or_else(|err| cx.throw_error(err.to_string()))
            .unwrap(),
    );
    runtime(cx)
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    let rt = tokio::runtime::Runtime::new().expect("could not start tokio rt");
    rt.block_on(future)
}

pub struct JSSystem {
    // tx: tokio::sync::mpsc::Sender<SystemMessage>,
    pub system: System,
}
impl Finalize for JSSystem {}

#[allow(unused)]
#[derive(Debug)]
enum SystemMessage {
    Close,
}

impl JSSystem {
    pub fn new() -> Self {
        let (send, mut recv) = channel::<SystemMessage>(1);
        let system = block_on(async move {
            let mut sys = System::initialize().await.unwrap();
            let mut do_quit = sys.quit.subscribe();

            tokio::select! {
                _ = signal::ctrl_c() => {}
                _ = do_quit.recv() => {
                }
                Some(message) = recv.recv() => {
                    println!("Received a message: {:?}", message);
                                        // let _ = match message {
                                        //     SystemMessage::Callback(deferred, f) => {
                                        //         f(&mut system, &channel, deferred);
                                        //         Ok::<_, anyhow::Error>(())
                                        //     },
                                        //     SystemMessage::Close => {
                                        //         // system.cleanup().await;
                                        //         Ok::<_, anyhow::Error>(())
                                        //     },
                                        // };
                                    }
                else => {
                    println!("Got something else?");
                }
            }
            let _res = sys.cleanup().await;
            drop(send);
            sys
        });
        Self { system }
    }

    pub fn as_uri(&self) -> anyhow::Result<String> {
        self.system.as_uri()
    }
}

impl JSSystem {
    pub fn js_new(mut cx: FunctionContext) -> JsResult<JsBox<JSSystem>> {
        let sys = JSSystem::new();
        Ok(cx.boxed(sys))
    }

    pub fn js_reset(mut cx: FunctionContext) -> JsResult<JsBoolean> {
        reset(&mut cx);
        Ok(cx.boolean(true))
    }

    pub fn js_db_uri(mut cx: FunctionContext) -> JsResult<JsString> {
        let sys = cx.this().downcast_or_throw::<JsBox<JSSystem>, _>(&mut cx)?;
        let uri = sys.as_uri().unwrap(); // TODO check this?
        Ok(cx.string(uri))
    }

    pub fn js_handle_request(mut cx: FunctionContext) -> JsResult<JsPromise> {
        let req = cx.argument::<JsString>(0)?.value(&mut cx);
        let value: serde_json::Value = serde_json::from_str(&req).unwrap();
        let request = serde_json::from_value::<JSSystemRequest>(value).unwrap();
        // let sys = cx.this().downcast_or_throw::<JsBox<JSSystem>, _>(&mut cx)?;

        // Boilerplate for Neon/Promises/Tokio
        let rt = runtime(&mut cx)?;
        let channel = cx.channel();
        let (deferred, promise) = cx.promise();
        // hack: don't even use a promise straight up block
        let a = block_on(async move {
            println!("Got a thing: {:?}", request);
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
