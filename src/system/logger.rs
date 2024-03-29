use std::path::Path;

use flexi_logger::{
    AdaptiveFormat, Age, Cleanup, Criterion, Duplicate, FileSpec, Logger, LoggerHandle, Naming,
    WriteMode,
};

pub fn init_logging(_config_dir: Option<&Path>) -> anyhow::Result<LoggerHandle> {
    let logger = Logger::try_with_env_or_str("warn")?
        .log_to_file(FileSpec::default().directory("logs").basename("pmem"))
        .rotate(
            Criterion::Age(Age::Hour),
            Naming::Numbers,
            Cleanup::KeepLogFiles(7),
        )
        .print_message()
        .format_for_writer(flexi_logger::colored_default_format)
        .adaptive_format_for_stderr(AdaptiveFormat::Default)
        .adaptive_format_for_stdout(AdaptiveFormat::Detailed)
        .duplicate_to_stderr(Duplicate::Info) // print warnings and errors also to the console
        .write_mode(WriteMode::BufferAndFlush)
        .start()?;

    Ok(logger)
}
