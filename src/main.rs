use std::{cmp, collections::HashMap, f64, str::FromStr, sync::atomic, time::Duration};

use chrono::Utc;
use hyprland::{error::HyprError, event_listener::AsyncEventListener};
use log::{LevelFilter, debug, error, info};
use thiserror::Error;
use tokio::time::sleep;

use crate::{config::Config, logger::setup_logger, state::State, storage::Storage};
mod config;
mod logger;
mod state;
mod storage;

const APP_NAME: &str = "nest";
const STORAGE_FILE_NAME: &str = "storage.txt";
const CONFIG_FILE_NAME: &str = "config.toml";
const LOG_FILE_NAME: &str = "output.txt";

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
    #[error("config error")]
    Config(#[from] crate::config::Error),
    #[error("logger error")]
    Logger(#[from] crate::logger::Error),
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let config = Config::new(APP_NAME, CONFIG_FILE_NAME)?;

    let log_level = match LevelFilter::from_str(&config.log_level) {
        Ok(val) => val,
        Err(err) => {
            print!("Failed to set log level: {}\n", err,);
            LevelFilter::Error
        }
    };
    setup_logger(APP_NAME, LOG_FILE_NAME, log_level)?;

    let mut storage = Storage::new(APP_NAME, STORAGE_FILE_NAME)?;
    let state = State::load(storage.read()?, config.buffer, config.ignore.into()).await;

    let mut event_listener = AsyncEventListener::new();

    let workspace_state = state.clone();
    event_listener.add_workspace_changed_handler(move |event| {
        let state = workspace_state.clone();
        Box::pin(async move {
            state.workspace_changed(event.id);
        })
    });

    let add_state = state.clone();
    event_listener.add_window_opened_handler(move |event| {
        let state = add_state.clone();
        Box::pin(async move {
            if !state
                .add_window(event.window_class.clone(), event.window_address.clone())
                .await {
                return;
            }
            let program = match state.get_program(event.window_class).await {
                Some(val) => val,
                None => return,
            };
            let mut score_map: HashMap<i32, f64> = HashMap::new();
            let now = Utc::now().timestamp();
            for position in program.positions {
                // Aging function score = e^(-age / τ)
                let age = (now - position.timestamp) as f64;
                let score = f64::powf(f64::consts::E, -age / config.tau);
                debug!("Position got a score of {score}");
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
                Ok(moved) => {
                    if moved {
                        info!(
                            "Moved window {} to {} with score {}",
                            event.window_address, workspace_id, score
                        )
                    } else {
                        info!(
                            "Tried to move window {} to {} with score {} but a move could not be completed",
                            event.window_address, workspace_id, score
                        )
                    }
                }
                Err(err) => error!("Failed to dispatch window move: {err}"),
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
                Err(err) => print!("Failed react to window move: {}\n", err),
            }
        })
    });

    let runtime_state = state.clone();
    tokio::spawn(async move {
        let state = runtime_state.clone();
        loop {
            if state.changed.load(atomic::Ordering::Relaxed) {
                let programs = state.get_programs().await;
                match storage.write(&programs) {
                    Ok(()) => {
                        info!("State saved to storage");
                        state.changed.store(false, atomic::Ordering::Relaxed)
                    }
                    Err(err) => error!("Failed to write changes: {err}"),
                }
            } else {
                debug!("No changes found in the state");
            }

            sleep(Duration::from_secs(config.save_frequency)).await;
        }
    });

    event_listener.start_listener_async().await?;
    Ok(())
}
