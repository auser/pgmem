use std::{cell::RefCell, sync::Arc};

use futures::stream::AbortHandle;
use neon::prelude::*;

#[derive(Debug)]
pub struct SystemServer {
    pub handle: RefCell<Option<AbortHandle>>,
}

impl Finalize for SystemServer {}

pub struct Function {
    pub channel: Channel,
    pub callback: Arc<Root<JsFunction>>,
}
