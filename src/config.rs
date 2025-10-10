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
    #[error("io operation failed: {0}")]
    IO(#[from] std::io::Error),
    #[error("failed to write config: {0}")]
    TomlSer(#[from] toml::ser::Error),
    #[error("failed to read config: {0}")]
    TomlDe(#[from] toml::de::Error),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub workspace: WorkspaceConfig,
    pub floating: FloatingConfig,
    pub save_frequency: u64,
    pub log_level: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub filter: ProgramFilter,
    pub buffer: usize,
    pub tau: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FloatingConfig {
    pub filter: ProgramFilter,
    pub frequency: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProgramFilter {
    pub mode: FilterMode,
    pub programs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum FilterMode {
    Include,
    Exclude,
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
            workspace: WorkspaceConfig {
                filter: ProgramFilter {
                    mode: FilterMode::Exclude,
                    programs: Vec::new(),
                },
                buffer: 30,
                tau: 604800.0,
            },
            floating: FloatingConfig {
                filter: ProgramFilter {
                    mode: FilterMode::Include,
                    programs: Vec::new(),
                },
                frequency: 5,
            },
            save_frequency: 10,
            log_level: log::LevelFilter::Info.as_str().to_string(),
        }
    }
}
