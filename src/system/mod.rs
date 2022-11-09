mod config;
mod db;
mod logger;
mod system;
mod system_server;
mod utils;

use neon::prelude::*;

use self::system_server::SystemServer;

fn hello(mut cx: FunctionContext) -> JsResult<JsString> {
    Ok(cx.string("hello node"))
}

pub fn neon_main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("hello", hello)?;
    cx.export_function("init_db", SystemServer::js_new)?;
    cx.export_function("start_db", SystemServer::js_start)?;
    cx.export_function("stop_db", SystemServer::js_stop)?;
    cx.export_function("new_db", SystemServer::js_create_new_db)?;
    cx.export_function("drop_db", SystemServer::js_drop_database)?;
    Ok(())
}
