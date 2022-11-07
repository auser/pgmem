use futures::Future;
use neon::prelude::*;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;

pub fn runtime<'a, C: Context<'a>>(cx: &mut C) -> NeonResult<&'static Runtime> {
    static RUNTIME: OnceCell<Runtime> = OnceCell::new();
    RUNTIME.get_or_try_init(|| Runtime::new().or_else(|err| cx.throw_error(err.to_string())))
}

pub fn block_on<F: Future>(future: F) -> F::Output {
    // let rt = tokio::runtime::Runtime::new().expect("could not start tokio rt");
    // let handle = Handle::current();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("could not create runtime");
    rt.block_on(future)
}
