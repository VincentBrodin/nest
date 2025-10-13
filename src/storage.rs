use std::{
    fs::{File, OpenOptions, create_dir_all},
    io::{Read, Seek, SeekFrom, Write},
    str::FromStr,
};

use log::error;
use thiserror::Error;

use crate::state::{ParseError, Program};

pub struct Storage {
    file: File,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("could not find config directory")]
    MissingConfig,
    #[error("io operation failed: {0}")]
    IO(#[from] std::io::Error),
    #[error("parsing error: {0}")]
    ParseError(#[from] ParseError),
}

impl Storage {
    pub fn new(app_name: &str, file_name: &str) -> Result<Self, Error> {
        let config_dir = match dirs::config_dir() {
            Some(val) => val,
            None => return Err(Error::MissingConfig),
        };
        let app_dir = config_dir.join(app_name);
        create_dir_all(&app_dir)?;
        let storage_path = app_dir.join(file_name);

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(storage_path)?;

        Ok(Self { file })
    }

    pub fn read(&mut self) -> Result<Vec<Program>, Error> {
        let mut buf = String::new();
        self.file.seek(SeekFrom::Start(0))?;
        self.file.read_to_string(&mut buf)?;

        let lines: Vec<&str> = buf.lines().collect();
        let mut programs: Vec<Program> = Vec::with_capacity(lines.len());

        for line in lines {
            let program = match Program::from_str(line) {
                Ok(val) => val,
                Err(err) => {
                    // error!("A program failed to parse: {err}");
                    return Err(Error::ParseError(err));
                }
            };
            programs.push(program);
        }
        Ok(programs)
    }

    pub fn write(&mut self, programs: &Vec<Program>) -> Result<(), Error> {
        self.file.set_len(0)?;
        self.file.seek(SeekFrom::Start(0))?;
        let mut content = String::new();
        for program in programs {
            content.push_str(&program.to_string());
            content.push('\n');
        }
        self.file.write_all(content.as_bytes())?;
        self.file.flush()?;
        Ok(())
    }
}
