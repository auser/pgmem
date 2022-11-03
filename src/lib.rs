use neon::prelude::*;
mod interface;
mod system;
mod system_tasks;
mod web;

pub use interface::JSSystem;

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("new_system", JSSystem::new_system)?;
    cx.export_function("close_system", JSSystem::close_system)?;
    Ok(())
}
