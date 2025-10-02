use std::{cmp, collections::HashMap, f64, sync::atomic, time::Duration};

use chrono::Utc;
use hyprland::{error::HyprError, event_listener::AsyncEventListener};
use thiserror::Error;
use tokio::time::sleep;

use crate::{state::State, storage::Storage};
mod state;
mod storage;

const APP_NAME: &str = "nest";
const STORAGE_FILE_NAME: &str = "storage.txt";
const TAU: f64 = 3600.0;
const SAVE_FREQUENCY: u64 = 10;

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
            let mut score_map: HashMap<i32, f64> = HashMap::new();
            let now = Utc::now().timestamp();

            for position in program.positions {
                // Aging function score = e^(-age / τ)
                let age = (now - position.timestamp) as f64;
                let score = f64::powf(f64::consts::E, -age / TAU);
                print!("Got score of {}\n", score);
                match score_map.get(&position.workspace_id) {
                    Some(val) => score_map.insert(position.workspace_id, *val + score),
                    None => score_map.insert(position.workspace_id, score),
                };
            }

            let (workspace_id, score) = match score_map.iter().max_by(|a, b| {
                if a.1 > b.1 {
                    cmp::Ordering::Greater
                } else if a.1 < b.1 {
                    cmp::Ordering::Less
                } else {
                    cmp::Ordering::Equal
                }
            }) {
                Some(val) => val,
                None => return,
            };

            match state
                .move_window(&event.window_address, *workspace_id)
                .await
            {
                Ok(()) => print!(
                    "Moved window {} to {} with score {}\n",
                    event.window_address, workspace_id, score
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
            if state.changed.load(atomic::Ordering::Relaxed) {
                let programs = state.get_programs().await;
                for program in programs.iter() {
                    print!("{}:{:?}\n", program.class, program.positions)
                }
                match storage.write(&programs) {
                    Ok(()) => state.changed.store(false, atomic::Ordering::Relaxed),
                    Err(err) => print!("Failed to write changes: {}\n", err),
                }
            } else {
                print!("No changes found in the state\n");
            }

            sleep(Duration::from_secs(SAVE_FREQUENCY)).await;
        }
    });

    event_listener.start_listener_async().await?;
    Ok(())
}
