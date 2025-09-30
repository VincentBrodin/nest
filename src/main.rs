use std::{
    collections::HashMap,
    fs::{File, OpenOptions, create_dir_all},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::atomic::Ordering,
    time::Duration,
};

use hyprland::{
    dispatch::{
        Dispatch,
        DispatchType::{self},
        WindowIdentifier, WorkspaceIdentifierWithSpecial,
    },
    error::HyprError,
    event_listener::AsyncEventListener,
};
use thiserror::Error;
use tokio::time::sleep;

use crate::state::{Position, Program, State};
mod state;

#[derive(Error, Debug)]
enum Error {
    #[error("hyprland error")]
    HyprError(#[from] HyprError),
    #[error("io error")]
    IO(#[from] std::io::Error),
    #[error("parse error")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("missing config path")]
    MissingConfig,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let state_path = match get_state_file_path() {
        Some(val) => val,
        None => return Err(Error::MissingConfig),
    };
    match state_path.parent() {
        Some(val) => create_dir_all(val)?,
        None => (),
    };

    let mut state_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(state_path)?;

    let state = State::load(load_state(&mut state_file)?).await;

    let mut event_listener = AsyncEventListener::new();

    let add_state = state.clone();
    event_listener.add_window_opened_handler(move |event| {
        let state = add_state.clone();
        Box::pin(async move {
            state
                .add_program(event.window_class.clone(), event.window_address.clone())
                .await;
            let program = match state.get_program(event.window_class).await {
                Some(val) => val,
                None => return,
            };
            let mut freq_map: HashMap<i32, i32> = HashMap::new();

            for position in program.positions {
                match freq_map.get(&position.workspace_id) {
                    Some(val) => freq_map.insert(position.workspace_id, *val + 1),
                    None => freq_map.insert(position.workspace_id, 1),
                };
            }

            let (workspace_id, _) = match freq_map.iter().max_by_key(|&(_, count)| count) {
                Some(val) => val,
                None => return,
            };

            match Dispatch::call_async(DispatchType::MoveToWorkspace(
                WorkspaceIdentifierWithSpecial::Id(*workspace_id),
                Some(WindowIdentifier::Address(event.window_address.clone())),
            ))
            .await
            {
                Ok(()) => print!(
                    "Moved window {} to {}\n",
                    event.window_address, workspace_id
                ),
                Err(err) => print!("Failed to dispatch window move: {}\n", err),
            };
        })
    });

    let remove_state = state.clone();
    event_listener.add_window_closed_handler(move |address| {
        let state = remove_state.clone();
        Box::pin(async move { state.remove_window(address).await })
    });

    let move_state = state.clone();
    event_listener.add_window_moved_handler(move |event| {
        let state = move_state.clone();
        Box::pin(async move {
            state
                .window_moved(event.window_address, event.workspace_id)
                .await;
        })
    });

    let runtime_state = state.clone();
    tokio::spawn(async move {
        let state = runtime_state.clone();
        loop {
            if state.changed.load(Ordering::Relaxed) {
                let programs = state.get_programs().await;
                for program in programs.iter() {
                    print!("{}:{:?}\n", program.class, program.positions)
                }
                match write_state(&mut state_file, &programs) {
                    Ok(()) => state.changed.store(false, Ordering::Relaxed),
                    Err(err) => print!("Failed to write changes: {}\n", err),
                }
            } else {
                print!("No changes found in the state\n");
            }

            sleep(Duration::from_secs(5)).await;
        }
    });

    event_listener.start_listener_async().await?;
    Ok(())
}

fn load_state(file: &mut File) -> Result<Vec<Program>, Error> {
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    file.seek(std::io::SeekFrom::Start(0))?;

    let mut programs: Vec<Program> = Vec::new();

    let lines = buf.lines();
    for line in lines {
        let split: Vec<&str> = line.split(':').collect();
        let class = split[0];
        let mut program = Program {
            class: class.to_string(),
            positions: Vec::new(),
        };
        let list_values: Vec<&str> = split[1]
            .trim()
            .trim_matches(&['[', ']'])
            .split(',')
            .collect();
        for values in list_values {
            if values.len() == 0 {
                continue;
            }
            let split: Vec<&str> = values.split(';').collect();
            let workspace_id: i32 = split[0].parse()?;
            let time: i64 = split[1].parse()?;
            program.positions.push(Position {
                workspace_id: workspace_id,
                time: time,
            });
            // print!("{}:{:?}\n", class, program);
        }
        programs.push(program);
    }

    Ok(programs)
}

fn write_state(file: &mut File, programs: &Vec<Program>) -> Result<(), Error> {
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    let mut content = String::new();
    for program in programs {
        content.push_str(&format!("{}:[", program.class));
        for (i, position) in program.positions.iter().enumerate() {
            if i == program.positions.len() - 1 {
                content.push_str(&format!(
                    "{};{}",
                    position.workspace_id, position.time as i32
                ));
            } else {
                content.push_str(&format!(
                    "{};{},",
                    position.workspace_id, position.time as i32
                ));
            }
        }
        content.push_str("]\n");
    }
    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(())
}

fn get_state_file_path() -> Option<PathBuf> {
    let config_dir = dirs::config_dir()?;
    let app_dir = config_dir.join("nest");
    Some(app_dir.join("state.txt"))
}
