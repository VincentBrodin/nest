use std::{fmt::Display, str::FromStr};

use crate::state::{FloatingWindow, ParseError, Workspace};

#[derive(Clone, Debug)]
pub struct Program {
    pub class: String,
    pub workspaces: Vec<Workspace>,
    pub floating_window: Option<FloatingWindow>,
    pub moved: bool,
    pub float_moved: bool,
}

impl Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:[", self.class)?;
        for (i, workspace) in self.workspaces.iter().enumerate() {
            write!(f, "{}", workspace)?;
            if i != self.workspaces.len() - 1 {
                write!(f, ",")?;
            }
        }
        match &self.floating_window {
            Some(floating_window) => write!(f, "]&[{}]", floating_window),
            None => write!(f, "]&[]"),
        }
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
            .trim_matches(['[', ']'])
            .split(',')
            .collect();

        let mut workspaces: Vec<Workspace> = Vec::with_capacity(workspaces_str.len());
        for workspace_str in workspaces_str.iter() {
            let workspace = Workspace::from_str(workspace_str)?;
            workspaces.push(workspace);
        }

        let window_str = data.last().unwrap_or(&"0").trim().trim_matches(['[', ']']);

        let floating_window = FloatingWindow::from_str(window_str).ok();

        Ok(Program {
            class: class.to_string(),
            workspaces,
            floating_window,
            moved: false,
            float_moved: false,
        })
    }
}
