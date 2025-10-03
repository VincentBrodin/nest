use std::fs::create_dir_all;

use log::LevelFilter;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("could not find config directory")]
    MissingConfig,
    #[error("fern failed to init")]
    Fern,
    #[error("io operation failed")]
    IO(#[from] std::io::Error),
}

pub fn setup_logger(app_name: &str, file_name: &str, log_level: LevelFilter) -> Result<(), Error> {
    let config_dir = match dirs::config_dir() {
        Some(val) => val,
        None => return Err(Error::MissingConfig),
    };
    let app_dir = config_dir.join(app_name);
    create_dir_all(&app_dir)?;
    let output_path = app_dir.join(file_name);

    match fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                chrono::Local::now().to_string(),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(log_level)
        .chain(std::io::stdout())
        .chain(fern::log_file(output_path)?)
        .apply()
    {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::Fern),
    }
}
