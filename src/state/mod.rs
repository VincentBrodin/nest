use crate::config::{Config, FilterMode};
use chrono::Utc;
use hyprland::{
    dispatch::{Dispatch, DispatchType, WindowIdentifier, WorkspaceIdentifierWithSpecial},
    error::HyprError,
    shared::Address,
};
use log::{debug, info};
use std::{
    collections::HashMap,
    num::ParseIntError,
    str::ParseBoolError,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI32, Ordering},
    },
};
use thiserror::Error;

mod safemap;
pub use safemap::SafeMap;

mod program;
pub use program::Program;

mod window;
pub use window::Window;

mod workspace;
pub use workspace::Workspace;

mod floatingwindow;
pub use floatingwindow::FloatingWindow;

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
    Int(#[from] ParseIntError),
    #[error("could parse bool: {0}")]
    Bool(#[from] ParseBoolError),
}

#[derive(Clone)]
pub struct State {
    addresses: SafeMap<Address, Window>,
    programs: SafeMap<String, Program>,
    current_workspace: Arc<AtomicI32>,
    workspace_list: Arc<[String]>,
    workspace_mode: FilterMode,
    workspace_buffer: usize,
    floating_list: Arc<[String]>,
    floating_mode: FilterMode,
    restore_list: Arc<[String]>,
    restore_mode: FilterMode,
    restore_timeout: i64,
    pub changed: Arc<AtomicBool>,
}

pub type WorkspaceConfig = (Arc<[String]>, FilterMode, usize);
pub type FloatingConfig = (Arc<[String]>, FilterMode);
pub type RestoreConfig = (Arc<[String]>, FilterMode, i64);

impl State {
    pub fn new(
        workspace_config: WorkspaceConfig,
        floating_config: FloatingConfig,
        restore_config: RestoreConfig,
    ) -> Self {
        Self {
            addresses: SafeMap::new(),
            programs: SafeMap::new(),
            workspace_list: workspace_config.0,
            workspace_mode: workspace_config.1,
            workspace_buffer: workspace_config.2,
            floating_list: floating_config.0,
            floating_mode: floating_config.1,
            restore_list: restore_config.0,
            restore_mode: restore_config.1,
            restore_timeout: restore_config.2,
            current_workspace: Arc::new(AtomicI32::new(1)),
            changed: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn load(programs: Vec<Program>, config: Config) -> Self {
        let state = Self::new(
            (
                config.workspace.filter.programs.into(),
                config.workspace.filter.mode,
                config.workspace.buffer,
            ),
            (
                config.floating.filter.programs.into(),
                config.floating.filter.mode,
            ),
            (
                config.restore.filter.programs.into(),
                config.restore.filter.mode,
                config.restore.timeout,
            ),
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
                let positions = vec![Workspace {
                    workspace_id: self.current_workspace.load(Ordering::Relaxed),
                    timestamp: Utc::now().timestamp(),
                }];
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
            let window = Window {
                class: class.clone(),
                timestamp: Utc::now(),
                origin: self.current_workspace(),
            };
            let mut addresses = self.addresses.0.lock().await;
            addresses.insert(address.clone(), window);
        }
        debug!("Window {address} of type {class} added");
    }

    // Removes mapping between window and program, it will never remove a programs state
    pub async fn remove_window(&self, address: Address) -> Result<(), Error> {
        let mut addresses = self.addresses.0.lock().await;
        if let Some(window) = addresses.remove(&address) {
            let diff = Utc::now() - window.timestamp;
            let is_in_list = self.restore_list.contains(&window.class);
            if self.restore_timeout >= diff.num_seconds()
                && ((is_in_list && self.restore_mode == FilterMode::Include)
                    || (!is_in_list && self.restore_mode == FilterMode::Exclude))
            {
                Dispatch::call_async(DispatchType::Workspace(WorkspaceIdentifierWithSpecial::Id(
                    window.origin,
                )))
                .await?;
            }
            debug!(
                "Window {address} of type {} removed after {}s",
                window.class,
                diff.num_seconds()
            );
        }
        Ok(())
    }

    pub async fn window_moved(&self, address: Address, workspace_id: i32) -> Result<(), Error> {
        let addresses = self.addresses.0.lock().await;
        let window = match addresses.get(&address) {
            Some(val) => val,
            None => {
                return Err(Error::BlankAddress);
            }
        };

        let mut programs = self.programs.0.lock().await;
        let program = match programs.get_mut(&window.class) {
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
            workspace_id,
            timestamp: Utc::now().timestamp(),
        };
        program.workspaces.push(position);

        while program.workspaces.len() > self.workspace_buffer {
            program.workspaces.remove(0);
        }

        self.changed.store(true, Ordering::Relaxed);
        info!(
            "Program of type {} got moved to workspace {}",
            window.class, workspace_id
        );

        Ok(())
    }

    pub async fn move_window(&self, address: &Address, workspace_id: i32) -> Result<bool, Error> {
        let addresses = self.addresses.0.lock().await;
        let mut programs = self.programs.0.lock().await;

        let window = match addresses.get(address) {
            Some(val) => val,
            None => return Err(Error::BlankAddress),
        };

        let is_in_list = self.workspace_list.contains(&window.class);
        if (!is_in_list && self.workspace_mode == FilterMode::Include)
            || (is_in_list && self.workspace_mode == FilterMode::Exclude)
        {
            return Ok(false);
        }

        let program = match programs.get_mut(&window.class) {
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

    pub async fn add_floating_window(
        &self,
        class: &str,
        window: FloatingWindow,
    ) -> Result<(), Error> {
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

        let window = match addresses.get(address) {
            Some(val) => val,
            None => return Err(Error::BlankAddress),
        };

        let is_in_list = self.floating_list.contains(&window.class);
        if (!is_in_list && self.floating_mode == FilterMode::Include)
            || (is_in_list && self.floating_mode == FilterMode::Exclude)
        {
            return Ok(false);
        }

        let program = match programs.get_mut(&window.class) {
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
