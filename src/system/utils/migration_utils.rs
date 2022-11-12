use std::{fs::File, io::Read, path::PathBuf};

use glob::glob;

use tracing::*;

#[allow(unused)]
pub fn read_all_migrations(root_path: PathBuf) -> String {
    debug!("read_all_migrations in : {:?}", root_path);
    let mut sql = String::new();
    let pattern = String::from(root_path.join("**/*.sql").to_str().unwrap());
    log::info!("Looking in directory: {:?}", pattern);
    for entry in glob(pattern.as_str()).expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                log::info!("Found path: {:#?}", path);
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

    log::info!("SQL to run everytime: {:#?}", sql);
    sql
}
