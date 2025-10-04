use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI32, Ordering},
    },
};

use chrono::Utc;
use hyprland::{
    dispatch::{Dispatch, DispatchType, WindowIdentifier, WorkspaceIdentifierWithSpecial},
    error::HyprError,
    shared::Address,
};
use log::{debug, info};
use thiserror::Error;
use tokio::sync::Mutex;

#[derive(Error, Debug)]
pub enum Error {
    #[error("hyprland error")]
    HyprError(#[from] HyprError),
    #[error("address not mapped to a class")]
    BlankAddress,
    #[error("class not mapped to a program")]
    BlankClass,
}

pub struct SafeMap<T, U>(Arc<Mutex<HashMap<T, U>>>);

impl<T, U> SafeMap<T, U> {
    fn new() -> Self {
        SafeMap(Arc::new(Mutex::new(HashMap::new())))
    }
}

impl<T, U> Clone for SafeMap<T, U> {
    fn clone(&self) -> Self {
        SafeMap(self.0.clone())
    }
}

#[derive(Clone, Debug)]
pub struct Program {
    pub class: String,
    pub positions: Vec<Position>,
    pub moved: bool,
}

#[derive(Clone, Debug)]
pub struct Position {
    pub workspace_id: i32,
    pub timestamp: i64,
}

pub struct State {
    addresses: SafeMap<Address, String>,
    programs: SafeMap<String, Program>,
    buffer: usize,
    workspace: Arc<AtomicI32>,
    ignore: Arc<[String]>,
    pub changed: Arc<AtomicBool>,
}

impl State {
    pub fn new(buffer: usize, ignore: Arc<[String]>) -> Self {
        Self {
            addresses: SafeMap::new(),
            programs: SafeMap::new(),
            buffer: buffer,
            ignore: ignore,
            workspace: Arc::new(AtomicI32::new(1)),
            changed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn load(programs: Vec<Program>, buffer: usize, ignore: Arc<[String]>) -> Self {
        let state = Self::new(buffer, ignore);
        let mut programs_map = state.programs.0.lock().await;
        for program in programs {
            programs_map.insert(program.class.clone(), program);
        }
        state.clone()
    }

    pub async fn add_window(&self, class: String, address: Address) -> bool {
        {
            // Creates new program if none exists
            if self.ignore.contains(&class) {
                debug!("{class} is in the ignore list");
                return false;
            }
            let mut programs = self.programs.0.lock().await;
            if !programs.contains_key(&class) {
                let mut positions: Vec<Position> = Vec::new();
                // If this is the first time we are opening a window we should store that
                positions.push(Position {
                    workspace_id: self.workspace.load(Ordering::Relaxed),
                    timestamp: Utc::now().timestamp(),
                });
                let _ = programs.insert(
                    class.clone(),
                    Program {
                        class: class.clone(),
                        positions: positions,
                        moved: false,
                    },
                );
            }
        }
        {
            // Maps the address to the program
            let mut addresses = self.addresses.0.lock().await;
            addresses.insert(address, class.clone());
        }
        debug!("Program of type {class} added");

        true
    }

    // Removes mapping between window and program, it will never remove a programs state
    pub async fn remove_window(&self, address: Address) {
        let mut addresses = self.addresses.0.lock().await;
        if let Some(class) = addresses.remove(&address) {
            debug!("Program of type {class} removed")
        }
    }

    pub async fn window_moved(&self, address: Address, workspace_id: i32) -> Result<(), Error> {
        let addresses = self.addresses.0.lock().await;
        let class = match addresses.get(&address) {
            Some(val) => val,
            None => {
                return Err(Error::BlankAddress);
            }
        };

        let mut programs = self.programs.0.lock().await;
        let program = match programs.get_mut(class) {
            Some(val) => val,
            None => {
                return Err(Error::BlankClass);
            }
        };

        // This is true if the program moved a window
        if program.moved {
            debug!("Internal move, ignoring results");
            program.moved = false;
            return Ok(());
        }

        let position = Position {
            workspace_id: workspace_id,
            timestamp: Utc::now().timestamp(),
        };
        program.positions.push(position);

        while program.positions.len() > self.buffer {
            program.positions.remove(0);
        }

        self.changed.store(true, Ordering::Relaxed);
        info!("Program of type {class} got moved to workspace {workspace_id}");

        Ok(())
    }

    pub async fn move_window(&self, address: &Address, workspace_id: i32) -> Result<bool, Error> {
        let addresses = self.addresses.0.lock().await;
        let mut programs = self.programs.0.lock().await;

        let class = match addresses.get(address) {
            Some(val) => val,
            None => return Err(Error::BlankAddress),
        };

        let program = match programs.get_mut(class) {
            Some(val) => val,
            None => return Err(Error::BlankClass),
        };

        program.moved = true;

        match Dispatch::call_async(DispatchType::MoveToWorkspace(
            WorkspaceIdentifierWithSpecial::Id(workspace_id),
            Some(WindowIdentifier::Address(address.clone())),
        ))
        .await
        {
            Ok(_) => Ok(true),
            Err(_) => {
                // We failed to move the window (this does not mean an error the window could be in the right position already)
                program.moved = false;
                Ok(false)
            }
        }
    }

    pub async fn get_program(&self, class: String) -> Option<Program> {
        let programs = self.programs.0.lock().await;
        programs.get(&class).cloned()
    }

    pub async fn get_programs(&self) -> Vec<Program> {
        let programs = self.programs.0.lock().await;
        let val: Vec<Program> = programs
            .clone()
            .into_iter()
            .map(|val| val.1.clone())
            .collect();
        val
    }

    pub fn workspace_changed(&self, id: i32) {
        self.workspace.store(id, Ordering::Relaxed);
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        Self {
            addresses: self.addresses.clone(),
            programs: self.programs.clone(),
            changed: self.changed.clone(),
            ignore: self.ignore.clone(),
            workspace: self.workspace.clone(),
            buffer: self.buffer,
        }
    }
}
