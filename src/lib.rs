use neon::prelude::*;
pub(crate) mod interface;
pub(crate) mod system;
pub(crate) mod system_tasks;
pub(crate) mod web;

pub use interface::JSSystem;

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("new_system", JSSystem::js_new)?;
    cx.export_function("reset", JSSystem::js_reset)?;
    cx.export_function("js_execute", JSSystem::js_handle_request)?;
    cx.export_function("full_db_uri", JSSystem::js_db_uri)?;
    Ok(())
}
