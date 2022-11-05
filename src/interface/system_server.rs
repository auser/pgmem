use std::{
    cell::RefCell,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use futures::stream::AbortHandle;
use neon::{prelude::*, types::Deferred};

use crate::system::System;

pub type SystemCallback = Box<dyn FnOnce(&mut System, &Channel, Deferred) + Send>;

#[derive(Debug)]
pub struct SystemServer {
    pub handle: RefCell<Option<AbortHandle>>,
    pub tx: tokio::sync::mpsc::Sender<SystemMessage>,
    // pub system: Arc<Mutex<System>>,
}

impl Finalize for SystemServer {}

#[allow(unused)]
#[derive()]
pub enum SystemMessage {
    Callback(Deferred, SystemCallback),
    Close,
}

impl Debug for SystemMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemMessage::Callback(_, _) => write!(f, "Callback"),
            SystemMessage::Close => write!(f, "Close"),
        }
    }
}
