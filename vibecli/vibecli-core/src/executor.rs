//! Command execution with safety checks

use anyhow::Result;
use std::process::{Command, Output};

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn execute(command: &str) -> Result<Output> {
        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", command])
                .output()?
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()?
        };
        
        Ok(output)
    }

    pub fn is_safe_command(command: &str) -> bool {
        let dangerous_commands = [
            "rm -rf",
            "del /f",
            "format",
            "mkfs",
            "dd if=",
            ":(){ :|:& };:",
        ];
        
        for dangerous in &dangerous_commands {
            if command.contains(dangerous) {
                return false;
            }
        }
        
        true
    }

    pub fn execute_with_approval(command: &str, auto_approve: bool) -> Result<Output> {
        if !Self::is_safe_command(command) && !auto_approve {
            anyhow::bail!("Command requires manual approval: {}", command);
        }
        
        Self::execute(command)
    }
}
