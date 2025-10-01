use std::{collections::HashMap, sync::atomic::Ordering, time::Duration};

use hyprland::{error::HyprError, event_listener::AsyncEventListener};
use thiserror::Error;
use tokio::time::sleep;

use crate::{state::State, storage::Storage};
mod state;
mod storage;

const APP_NAME: &str = "nest";
const STORAGE_FILE_NAME: &str = "storage.txt";

#[derive(Error, Debug)]
enum Error {
    #[error("hyprland error")]
    HyprError(#[from] HyprError),
    #[error("io error")]
    IO(#[from] std::io::Error),
    #[error("parse error")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("storage error")]
    Storage(#[from] crate::storage::Error),
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let mut storage = Storage::new(APP_NAME, STORAGE_FILE_NAME)?;
    let state = State::load(storage.read()?).await;

    let mut event_listener = AsyncEventListener::new();

    let add_state = state.clone();
    event_listener.add_window_opened_handler(move |event| {
        let state = add_state.clone();
        Box::pin(async move {
            state
                .add_window(event.window_class.clone(), event.window_address.clone())
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

            match state
                .move_window(&event.window_address, *workspace_id)
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
            match state
                .window_moved(event.window_address, event.workspace_id)
                .await
            {
                Ok(_) => (),
                Err(err) => print!("Failed to move window: {}\n", err),
            }
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
                match storage.write(&programs) {
                    Ok(()) => state.changed.store(false, Ordering::Relaxed),
                    Err(err) => print!("Failed to write changes: {}\n", err),
                }
            } else {
                print!("No changes found in the state\n");
            }

            sleep(Duration::from_secs(10)).await;
        }
    });

    event_listener.start_listener_async().await?;
    Ok(())
}
