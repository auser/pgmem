pub mod interface;
pub(crate) mod system;
pub(crate) mod system_tasks;

use interface::neon_main;
use neon::prelude::*;

#[neon::main]
pub fn main(cx: ModuleContext) -> NeonResult<()> {
    neon_main(cx)
}
