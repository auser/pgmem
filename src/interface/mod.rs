mod interface;
mod system_server;
mod utils;

pub(crate) use interface::*;
use neon::prelude::*;

pub fn neon_main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("hello", hello)?;
    cx.export_function("start_db", JSSystem::js_start_db)?;
    cx.export_function("stop_db", JSSystem::js_stop_db)?;
    Ok(())
}
