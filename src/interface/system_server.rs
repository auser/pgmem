use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

use futures::stream::AbortHandle;
use neon::{prelude::*, types::Deferred};

use crate::system::System;

pub type SystemCallback = Box<dyn FnOnce(&Channel, Deferred) + Send>;

#[derive(Debug)]
pub struct SystemServer {
    pub handle: RefCell<Option<AbortHandle>>,
    pub tx: tokio::sync::mpsc::Sender<SystemMessage>,
    pub system: Arc<Mutex<System>>,
}

impl Finalize for SystemServer {}

pub struct Function {
    pub channel: Channel,
    pub callback: Arc<Root<JsFunction>>,
}

#[allow(unused)]
#[derive()]
pub enum SystemMessage {
    Callback(Deferred, SystemCallback),
    Close,
}
