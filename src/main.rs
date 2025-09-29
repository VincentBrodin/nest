use std::{
    fs::{OpenOptions, create_dir_all},
    io::Write,
    ops::DerefMut,
    path::PathBuf,
    sync::atomic::Ordering,
    time::Duration,
};

use hyprland::{error::HyprError, event_listener::AsyncEventListener};
use thiserror::Error;
use tokio::time::sleep;

use crate::state::State;
mod state;

#[derive(Error, Debug)]
enum Error {
    #[error("hyprland error")]
    HyprError(#[from] HyprError),
    #[error("io error")]
    IO(#[from] std::io::Error),
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

    let state = State::new();

    let mut event_listener = AsyncEventListener::new();

    let add_state = state.clone();
    event_listener.add_window_opened_handler(move |event| {
        let state = add_state.clone();
        Box::pin(async move {
            state
                .add_program(event.window_class, event.window_address)
                .await
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
                for program in programs {
                    let mut row = String::new();
                    row.push_str(&format!("{}:[", program.class));
                    for (i, position) in program.positions.iter().enumerate() {
                        if i == program.positions.len() - 1 {
                            row.push_str(&format!(
                                "{};{}",
                                position.workspace_id, position.time as i32
                            ));
                        } else {
                            row.push_str(&format!(
                                "{};{},",
                                position.workspace_id, position.time as i32
                            ));
                        }
                    }
                    row.push_str("]\n");
                    let _ = state_file.write(row.as_bytes());
                }
                let _ = state_file.flush();
                state.changed.store(false, Ordering::Relaxed);
            } else {
                print!("No changes found in the state\n");
            }

            sleep(Duration::from_secs(5)).await;
        }
    });

    event_listener.start_listener_async().await?;
    Ok(())
}

fn get_state_file_path() -> Option<PathBuf> {
    let config_dir = dirs::config_dir()?;
    let app_dir = config_dir.join("nest");
    Some(app_dir.join("state.txt"))
}
