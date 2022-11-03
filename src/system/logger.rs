use std::path::{Path, PathBuf};

use flexi_logger::{Age, Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming, WriteMode};

pub fn init_logging(config_dir: Option<&Path>) -> anyhow::Result<()> {
    let _logger = Logger::try_with_str("info")?
        .log_to_file(FileSpec::default().directory("logs").basename("pgqlmem"))
        .rotate(
            Criterion::Age(Age::Hour),
            Naming::Numbers,
            Cleanup::KeepLogFiles(7),
        )
        // .print_message()
        .duplicate_to_stderr(Duplicate::Info) // print warnings and errors also to the console
        .write_mode(WriteMode::BufferAndFlush)
        .start()?;
    Ok(())
}
