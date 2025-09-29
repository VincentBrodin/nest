use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use chrono::Utc;
use hyprland::shared::Address;
use tokio::sync::Mutex;

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

#[derive(Clone)]
pub struct Program {
    pub class: String,
    pub positions: Vec<Position>,
}

#[derive(Clone, Debug)]
pub struct Position {
    pub workspace_id: i32,
    pub time: i64,
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

    pub async fn add_program(&self, class: String, address: Address) {
        {
            // Creates new program if none exists
            let mut programs = self.programs.0.lock().await;
            if !programs.contains_key(&class) {
                let _ = programs.insert(
                    class.clone(),
                    Program {
                        class: class.clone(),
                        positions: Vec::new(),
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

    pub async fn window_moved(&self, address: Address, workspace_id: i32) -> bool {
        let addresses = self.addresses.0.lock().await;
        let class = match addresses.get(&address) {
            Some(val) => val,
            None => return false,
        };

        let mut programs = self.programs.0.lock().await;
        let state = match programs.get_mut(class) {
            Some(val) => val,
            None => return false,
        };
        let position = Position {
            workspace_id: workspace_id,
            time: Utc::now().timestamp(),
        };
        state.positions.push(position);
        self.changed.store(true, Ordering::Relaxed);
        print!(
            "Program of type {} got moved to workspace {}\n",
            class, workspace_id
        );

        true
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
