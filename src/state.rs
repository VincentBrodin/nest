use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use chrono::Utc;
use hyprland::{
    dispatch::{Dispatch, DispatchType, WindowIdentifier, WorkspaceIdentifierWithSpecial},
    error::HyprError,
    shared::Address,
};
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
    pub changed: Arc<AtomicBool>,
}

impl State {
    pub fn new() -> Self {
        Self {
            addresses: SafeMap::new(),
            programs: SafeMap::new(),
            changed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn load(programs: Vec<Program>) -> Self {
        let state = Self::new();
        let mut programs_map = state.programs.0.lock().await;
        for program in programs {
            programs_map.insert(program.class.clone(), program);
        }
        state.clone()
    }

    pub async fn add_window(&self, class: String, address: Address) {
        {
            // Creates new program if none exists
            let mut programs = self.programs.0.lock().await;
            if !programs.contains_key(&class) {
                let _ = programs.insert(
                    class.clone(),
                    Program {
                        class: class.clone(),
                        positions: Vec::new(),
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
        print!("Program of type {} added\n", class)
    }

    // Removes mapping between window and program, it will never remove a programs state
    pub async fn remove_window(&self, address: Address) {
        let mut addresses = self.addresses.0.lock().await;
        if let Some(class) = addresses.remove(&address) {
            print!("Program of type {} removed\n", class)
        }
    }

    pub async fn window_moved(&self, address: Address, workspace_id: i32) -> Result<(), Error> {
        let addresses = self.addresses.0.lock().await;
        let class = match addresses.get(&address) {
            Some(val) => val,
            None => return Err(Error::BlankAddress),
        };

        let mut programs = self.programs.0.lock().await;
        let program = match programs.get_mut(class) {
            Some(val) => val,
            None => return Err(Error::BlankClass),
        };

        // This is true if the program moved a window
        if program.moved {
            println!("Internal move found, ignoring results");
            program.moved = false;
            return Ok(());
        }

        let position = Position {
            workspace_id: workspace_id,
            timestamp: Utc::now().timestamp(),
        };
        program.positions.push(position);

        while program.positions.len() > 30 {
            program.positions.remove(0);
        }

        self.changed.store(true, Ordering::Relaxed);
        print!(
            "Program of type {} got moved to workspace {}\n",
            class, workspace_id
        );

        Ok(())
    }

    pub async fn move_window(&self, address: &Address, workspace_id: i32) -> Result<(), Error> {
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

        Dispatch::call_async(DispatchType::MoveToWorkspace(
            WorkspaceIdentifierWithSpecial::Id(workspace_id),
            Some(WindowIdentifier::Address(address.clone())),
        ))
        .await?;

        Ok(())
    }

    pub async fn get_program(&self, class: String) -> Option<Program> {
        let programs = self.programs.0.lock().await;
        programs.get(&class).cloned()
    }

    pub async fn get_programs(&self) -> Vec<Program> {
        let programs = self.programs.0.lock().await;
        let v: Vec<Program> = programs
            .clone()
            .into_iter()
            .map(|val| val.1.clone())
            .collect();
        v
    }
}

impl Clone for State {
    fn clone(&self) -> Self {
        Self {
            addresses: self.addresses.clone(),
            programs: self.programs.clone(),
            changed: self.changed.clone(),
        }
    }
}
