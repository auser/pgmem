extern crate tempdir;
use neon::prelude::*;

use crate::system::neon_main;
pub(crate) mod system;

#[neon::main]
pub fn main(cx: ModuleContext) -> NeonResult<()> {
    neon_main(cx)
}
