use std::{
    collections::HashMap,
    num::ParseIntError,
    str::{FromStr, ParseBoolError},
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

use crate::config::{Config, FilterMode};

#[derive(Error, Debug)]
pub enum Error {
    #[error("hyprland error")]
    HyprError(#[from] HyprError),
    #[error("address not mapped to a class")]
    BlankAddress,
    #[error("class not mapped to a program")]
    BlankClass,
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("invalid format found")]
    InvalidFormat,
    #[error("could parse int: {0}")]
    InvalidInt(#[from] ParseIntError),
    #[error("could parse bool: {0}")]
    InvalidBool(#[from] ParseBoolError),
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
    pub workspaces: Vec<Workspace>,
    pub floating_window: Option<Window>,
    pub moved: bool,
    pub float_moved: bool,
}

impl ToString for Program {
    fn to_string(&self) -> String {
        let mut buf = String::new();
        buf.push_str(&format!("{}:[", self.class));
        for (i, workspace) in self.workspaces.iter().enumerate() {
            buf.push_str(&workspace.to_string());
            if i != self.workspaces.len() - 1 {
                buf.push(',');
            }
        }
        match &self.floating_window {
            Some(window) => buf.push_str(&format!("]&[{}]", window.to_string())),
            None => buf.push_str("]&[]"),
        }
        buf
    }
}

impl FromStr for Program {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split: Vec<&str> = s.split(':').collect();
        let class = split.first().unwrap_or(&"0");
        let data: Vec<&str> = split.last().unwrap_or(&"0").split('&').collect();

        let workspaces_str: Vec<&str> = data
            .first()
            .unwrap_or(&"0")
            .trim()
            .trim_matches(&['[', ']'])
            .split(',')
            .collect();

        let mut workspaces: Vec<Workspace> = Vec::new();
        workspaces.reserve(workspaces_str.len());
        for workspace_str in workspaces_str.iter() {
            let workspace = match Workspace::from_str(&workspace_str) {
                Ok(val) => val,
                Err(err) => return Err(err),
            };
            workspaces.push(workspace);
        }

        let window_str = data.last().unwrap_or(&"0").trim().trim_matches(&['[', ']']);

        let window = match Window::from_str(window_str) {
            Ok(window) => Some(window),
            Err(_) => None,
        };

        Ok(Program {
            class: class.to_string(),
            workspaces: workspaces,
            floating_window: window,
            moved: false,
            float_moved: false,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Workspace {
    pub workspace_id: i32,
    pub timestamp: i64,
}

impl ToString for Workspace {
    fn to_string(&self) -> String {
        format!("{};{}", self.workspace_id, self.timestamp)
    }
}

impl FromStr for Workspace {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseError::InvalidFormat);
        }
        let parts: Vec<&str> = s.split(";").collect();
        if parts.len() != 2 {
            return Err(ParseError::InvalidFormat);
        }

        let workspace_id: i32 = parts[0].parse()?;
        let timestamp: i64 = parts[1].parse()?;

        Ok(Workspace {
            workspace_id: workspace_id,
            timestamp: timestamp,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Window {
    pub at: (i16, i16),
    pub size: (i16, i16),
}

impl ToString for Window {
    fn to_string(&self) -> String {
        format!(
            "{};{};{};{}",
            self.at.0, self.at.1, self.size.0, self.size.1
        )
    }
}

impl FromStr for Window {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseError::InvalidFormat);
        }
        let parts: Vec<&str> = s.split(";").collect();
        if parts.len() != 4 {
            // Return a ParseIntError by attempting a dummy parse
            // because ParseIntError has no public constructor.
            return Err(ParseError::InvalidFormat);
        }

        let at_x: i16 = parts[0].parse()?;
        let at_y: i16 = parts[1].parse()?;
        let size_x: i16 = parts[2].parse()?;
        let size_y: i16 = parts[3].parse()?;

        Ok(Window {
            at: (at_x, at_y),
            size: (size_x, size_y),
        })
    }
}

#[derive(Clone)]
pub struct State {
    addresses: SafeMap<Address, String>,
    programs: SafeMap<String, Program>,
    current_workspace: Arc<AtomicI32>,
    workspace_list: Arc<[String]>,
    workspace_mode: FilterMode,
    workspace_buffer: usize,
    floating_list: Arc<[String]>,
    floating_mode: FilterMode,
    pub changed: Arc<AtomicBool>,
}

impl State {
    pub fn new(
        workspace_buffer: usize,
        workspace_list: Arc<[String]>,
        workspace_mode: FilterMode,
        floating_list: Arc<[String]>,
        floating_mode: FilterMode,
    ) -> Self {
        Self {
            addresses: SafeMap::new(),
            programs: SafeMap::new(),
            workspace_list: workspace_list,
            workspace_mode: workspace_mode,
            workspace_buffer: workspace_buffer,
            floating_list: floating_list,
            floating_mode: floating_mode,
            current_workspace: Arc::new(AtomicI32::new(1)),
            changed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn load(programs: Vec<Program>, config: Config) -> Self {
        let state = Self::new(
            config.workspace.buffer,
            config.workspace.filter.programs.into(),
            config.workspace.filter.mode,
            config.floating.filter.programs.into(),
            config.floating.filter.mode,
        );
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
                let mut positions: Vec<Workspace> = Vec::new();
                // If this is the first time we are opening a window we should store that
                positions.push(Workspace {
                    workspace_id: self.current_workspace.load(Ordering::Relaxed),
                    timestamp: Utc::now().timestamp(),
                });
                let _ = programs.insert(
                    class.clone(),
                    Program {
                        class: class.clone(),
                        workspaces: positions,
                        floating_window: None,
                        moved: false,
                        float_moved: false,
                    },
                );
            }
        }
        {
            // Maps the address to the program
            let mut addresses = self.addresses.0.lock().await;
            addresses.insert(address.clone(), class.clone());
        }
        debug!("Window {address} of type {class} added");
    }

    // Removes mapping between window and program, it will never remove a programs state
    pub async fn remove_window(&self, address: Address) {
        let mut addresses = self.addresses.0.lock().await;
        if let Some(class) = addresses.remove(&address) {
            debug!("Window {address} of type {class} removed")
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

        let position = Workspace {
            workspace_id: workspace_id,
            timestamp: Utc::now().timestamp(),
        };
        program.workspaces.push(position);

        while program.workspaces.len() > self.workspace_buffer {
            program.workspaces.remove(0);
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

        let is_in_list = self.workspace_list.contains(class);
        if (!is_in_list && self.workspace_mode == FilterMode::Include)
            || (is_in_list && self.workspace_mode == FilterMode::Exclude)
        {
            return Ok(false);
        }

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

    pub async fn add_floating_window(&self, class: &str, window: Window) -> Result<(), Error> {
        let mut programs = self.programs.0.lock().await;

        let program = match programs.get_mut(class) {
            Some(val) => val,
            None => return Err(Error::BlankClass),
        };

        let change = match &program.floating_window {
            Some(last) => last.at != window.at || last.size != window.size,
            None => true,
        };

        if change {
            program.floating_window = Some(window);
            self.changed.store(true, Ordering::Relaxed);
        }

        Ok(())
    }

    pub async fn remove_floating_window(&self, class: &str) -> Result<(), Error> {
        let mut programs = self.programs.0.lock().await;

        let program = match programs.get_mut(class) {
            Some(val) => val,
            None => return Err(Error::BlankClass),
        };

        program.floating_window = None;
        self.changed.store(true, Ordering::Relaxed);
        Ok(())
    }

    pub async fn move_float_window(
        &self,
        address: &Address,
        at: (i16, i16),
        size: (i16, i16),
    ) -> Result<bool, Error> {
        let addresses = self.addresses.0.lock().await;
        let mut programs = self.programs.0.lock().await;

        let class = match addresses.get(address) {
            Some(val) => val,
            None => return Err(Error::BlankAddress),
        };

        let is_in_list = self.floating_list.contains(class);
        if (!is_in_list && self.floating_mode == FilterMode::Include)
            || (is_in_list && self.floating_mode == FilterMode::Exclude)
        {
            return Ok(false);
        }

        let program = match programs.get_mut(class) {
            Some(val) => val,
            None => return Err(Error::BlankClass),
        };

        program.float_moved = true;

        match Dispatch::call_async(DispatchType::ToggleFloating(Some(
            WindowIdentifier::Address(address.clone()),
        )))
        .await
        {
            Ok(_) => (),
            Err(_) => {
                program.float_moved = false;
                return Ok(false);
            }
        }

        match Dispatch::call_async(DispatchType::MoveWindowPixel(
            hyprland::dispatch::Position::Exact(at.0, at.1),
            WindowIdentifier::Address(address.clone()),
        ))
        .await
        {
            Ok(_) => (),
            Err(_) => {
                program.float_moved = false;
                return Ok(false);
            }
        }

        match Dispatch::call_async(DispatchType::ResizeWindowPixel(
            hyprland::dispatch::Position::Exact(size.0, size.1),
            WindowIdentifier::Address(address.clone()),
        ))
        .await
        {
            Ok(_) => Ok(true),
            Err(_) => {
                program.float_moved = false;
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

    pub async fn get_mapped_programs(&self) -> HashMap<String, Program> {
        let programs = self.programs.0.lock().await;
        programs.clone()
    }

    pub fn workspace_changed(&self, id: i32) {
        self.current_workspace.store(id, Ordering::Relaxed);
    }

    pub fn current_workspace(&self) -> i32 {
        self.current_workspace.load(Ordering::Relaxed)
    }
}
