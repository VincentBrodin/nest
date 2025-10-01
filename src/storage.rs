use std::{
    fs::{File, OpenOptions, create_dir_all},
    io::{Read, Seek, SeekFrom, Write},
};

use thiserror::Error;

use crate::state::{Position, Program};

pub struct Storage {
    file: File,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("could not find config directory")]
    MissingConfig,
    #[error("io operation failed")]
    IO(#[from] std::io::Error),
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
            .open(storage_path)?;

        Ok(Self { file: file })
    }

    pub fn read(&mut self) -> Result<Vec<Program>, Error> {
        let mut buf = String::new();
        self.file.seek(SeekFrom::Start(0))?;
        self.file.read_to_string(&mut buf)?;

        let lines: Vec<&str> = buf.lines().collect();
        let mut programs: Vec<Program> = Vec::new();
        programs.reserve(lines.len());

        for line in lines {
            let split: Vec<&str> = line.split(':').collect();
            let class = match split.first() {
                Some(val) => *val,
                None => continue,
            };
            let objects: Vec<&str> = match split.last() {
                Some(val) => val.trim().trim_matches(&['[', ']']).split(',').collect(),
                None => continue,
            };

            let mut program = Program {
                class: class.to_string(),
                positions: Vec::new(),
                moved: false,
            };
            program.positions.reserve(objects.len());
            for object in objects {
                let position = match str_to_position(object) {
                    Some(val) => val,
                    None => continue,
                };
                program.positions.push(position);
            }
            programs.push(program);
        }
        Ok(programs)
    }

    pub fn write(&mut self, programs: &Vec<Program>) -> Result<(), Error> {
        self.file.set_len(0)?;
        self.file.seek(SeekFrom::Start(0))?;
        let mut content = String::new();
        for program in programs {
            content.push_str(&format!("{}:[", program.class));
            for (i, position) in program.positions.iter().enumerate() {
                if i == program.positions.len() - 1 {
                    content.push_str(&format!("{};{}", position.workspace_id, position.time));
                } else {
                    content.push_str(&format!("{};{},", position.workspace_id, position.time));
                }
            }
            content.push_str("]\n");
        }
        self.file.write_all(content.as_bytes())?;
        self.file.flush()?;
        Ok(())
    }
}

fn str_to_position(value: &str) -> Option<Position> {
    if value.is_empty() {
        return None;
    }
    let split: Vec<&str> = value.split(";").collect();
    let workspace_id: i32 = match split.first()?.parse() {
        Ok(val) => val,
        Err(_) => return None,
    };
    let time: i64 = match split.last()?.parse() {
        Ok(val) => val,
        Err(_) => return None,
    };
    Some(Position {
        workspace_id: workspace_id,
        time: time,
    })
}
