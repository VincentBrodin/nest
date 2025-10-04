use std::{
    fs::{File, create_dir_all},
    io::{Read, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("could not find config directory")]
    MissingConfig,
    #[error("io operation failed")]
    IO(#[from] std::io::Error),
    #[error("toml serialize operation failed")]
    TomlSer(#[from] toml::ser::Error),
    #[error("toml deserialize operation failed")]
    TomlDe(#[from] toml::de::Error),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub tau: f64,
    pub buffer: usize,
    pub save_frequency: u64,
    pub log_level: String,
    pub ignore: Vec<String>,
}

impl Config {
    pub fn new(app_name: &str, file_name: &str) -> Result<Self, Error> {
        let config_dir = match dirs::config_dir() {
            Some(val) => val,
            None => return Err(Error::MissingConfig),
        };
        let app_dir = config_dir.join(app_name);
        create_dir_all(&app_dir)?;
        let config_path = app_dir.join(file_name);

        if !Path::exists(&config_path) {
            let mut file = File::create(&config_path)?;
            let config = Config::default();
            let toml = toml::to_string(&config)?;
            file.write_all(toml.as_bytes())?;
            Ok(config)
        } else {
            let mut buf = String::new();
            let mut file = File::open(&config_path)?;
            file.read_to_string(&mut buf)?;
            let config = toml::from_str(&buf)?;
            Ok(config)
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tau: 3600.0,
            buffer: 30,
            save_frequency: 10,
            ignore: Vec::new(),
            log_level: log::LevelFilter::Info.as_str().to_string(),
        }
    }
}
