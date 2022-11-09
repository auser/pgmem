use std::{fs::File, io::Read, path::PathBuf};

use futures::Future;
use glob::glob;
use neon::prelude::*;
use once_cell::sync::OnceCell;
use tokio::runtime::Runtime;
use tracing::*;

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

#[allow(unused)]
pub fn read_all_migrations(root_path: PathBuf) -> String {
    debug!("read_all_migrations in : {:?}", root_path);
    let mut sql = String::new();
    let pattern = String::from(root_path.join("**/*.sql").to_str().unwrap());
    info!("Looking in directory: {:?}", pattern);
    for entry in glob(pattern.as_str()).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                info!("Found path: {:#?}", path);
                match File::open(path) {
                    Err(e) => {
                        error!("Unable to open {:?}", e);
                    }
                    Ok(mut file) => {
                        let mut contents = String::new();
                        file.read_to_string(&mut contents)
                            .expect("Unable to read file");
                        sql.push_str(contents.as_str());
                        sql.push_str(&format!("\n\n----\n\n"));
                    }
                };
            }
            Err(e) => {
                error!("Unable to read file: {:?}", e);
            }
        }
    }

    info!("SQL to run everytime: {:#?}", sql);
    sql
}
