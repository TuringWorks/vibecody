use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::mpsc::Sender;

pub struct TerminalManager {
    ptys: Arc<Mutex<HashMap<u32, Box<dyn portable_pty::MasterPty + Send>>>>,
    next_id: Arc<Mutex<u32>>,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            ptys: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    pub fn spawn(&self, shell: &str, tx: Sender<(u32, String)>) -> Result<u32> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let cmd = CommandBuilder::new(shell);
        let _child = pair.slave.spawn_command(cmd)?;

        let mut reader = pair.master.try_clone_reader()?;
        let master = pair.master;

        let id = {
            let mut next_id = self.next_id.lock().unwrap_or_else(|e| e.into_inner());
            let id = *next_id;
            *next_id += 1;
            id
        };

        {
            let mut ptys = self.ptys.lock().unwrap_or_else(|e| e.into_inner());
            ptys.insert(id, master);
        }

        // Spawn thread to read output
        let tx_clone = tx.clone();
        thread::spawn(move || {
            let mut buffer = [0u8; 1024];
            loop {
                match reader.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        let output = String::from_utf8_lossy(&buffer[..n]).to_string();
                        if tx_clone.blocking_send((id, output)).is_err() {
                            break;
                        }
                    }
                    _ => break,
                }
            }
        });

        Ok(id)
    }

    pub fn write(&self, id: u32, data: &str) -> Result<()> {
        let mut ptys = self.ptys.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(master) = ptys.get_mut(&id) {
            write!(master.take_writer()?, "{}", data)?;
        }
        Ok(())
    }

    pub fn resize(&self, id: u32, rows: u16, cols: u16) -> Result<()> {
        let mut ptys = self.ptys.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(master) = ptys.get_mut(&id) {
            master.resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })?;
        }
        Ok(())
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_terminal_manager_has_empty_ptys() {
        let tm = TerminalManager::new();
        let ptys = tm.ptys.lock().unwrap();
        assert!(ptys.is_empty());
    }

    #[test]
    fn new_terminal_manager_starts_at_id_zero() {
        let tm = TerminalManager::new();
        let next_id = tm.next_id.lock().unwrap();
        assert_eq!(*next_id, 0);
    }

    #[test]
    fn default_is_same_as_new() {
        let tm = TerminalManager::default();
        let ptys = tm.ptys.lock().unwrap();
        let next_id = tm.next_id.lock().unwrap();
        assert!(ptys.is_empty());
        assert_eq!(*next_id, 0);
    }

    #[test]
    fn write_to_nonexistent_pty_is_ok() {
        // Writing to an ID that does not exist should not panic; it silently does nothing.
        let tm = TerminalManager::new();
        let result = tm.write(999, "hello");
        assert!(result.is_ok());
    }

    #[test]
    fn resize_nonexistent_pty_is_ok() {
        // Resizing a PTY that does not exist should not panic.
        let tm = TerminalManager::new();
        let result = tm.resize(42, 40, 120);
        assert!(result.is_ok());
    }

    #[test]
    fn ptys_map_is_shared_across_clones() {
        // The Arc<Mutex<HashMap>> should be the same object across field access.
        let tm = TerminalManager::new();
        let ptys1 = Arc::clone(&tm.ptys);
        let ptys2 = Arc::clone(&tm.ptys);
        assert!(Arc::ptr_eq(&ptys1, &ptys2));
    }

    #[test]
    fn next_id_is_shared_across_clones() {
        let tm = TerminalManager::new();
        let id1 = Arc::clone(&tm.next_id);
        let id2 = Arc::clone(&tm.next_id);
        assert!(Arc::ptr_eq(&id1, &id2));
    }

    #[test]
    fn multiple_writes_to_missing_pty_all_succeed() {
        let tm = TerminalManager::new();
        for i in 0..10 {
            assert!(tm.write(i, &format!("data {}", i)).is_ok());
        }
    }

    #[test]
    fn multiple_resizes_to_missing_pty_all_succeed() {
        let tm = TerminalManager::new();
        for id in 0..5 {
            assert!(tm.resize(id, 24, 80).is_ok());
            assert!(tm.resize(id, 50, 200).is_ok());
        }
    }

    #[test]
    fn spawn_with_invalid_shell_returns_error() {
        let tm = TerminalManager::new();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        // A shell path that does not exist should fail
        let result = tm.spawn("/nonexistent/shell/path", tx);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn spawn_valid_shell_returns_id() {
        let tm = TerminalManager::new();
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        // Use /bin/sh which should exist on all Unix systems
        let result = tm.spawn("/bin/sh", tx);
        assert!(result.is_ok());
        let id = result.unwrap();
        assert_eq!(id, 0); // First spawn should get ID 0
    }

    #[tokio::test]
    async fn spawn_increments_ids() {
        let tm = TerminalManager::new();
        let (tx1, _rx1) = tokio::sync::mpsc::channel(100);
        let (tx2, _rx2) = tokio::sync::mpsc::channel(100);
        let id1 = tm.spawn("/bin/sh", tx1).unwrap();
        let id2 = tm.spawn("/bin/sh", tx2).unwrap();
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
    }

    #[tokio::test]
    async fn spawn_adds_to_ptys_map() {
        let tm = TerminalManager::new();
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        let id = tm.spawn("/bin/sh", tx).unwrap();
        let ptys = tm.ptys.lock().unwrap();
        assert!(ptys.contains_key(&id));
    }

    #[tokio::test]
    async fn write_to_spawned_pty() {
        let tm = TerminalManager::new();
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        let id = tm.spawn("/bin/sh", tx).unwrap();
        // Writing to a valid PTY should succeed
        let result = tm.write(id, "echo hello\n");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn resize_spawned_pty() {
        let tm = TerminalManager::new();
        let (tx, _rx) = tokio::sync::mpsc::channel(100);
        let id = tm.spawn("/bin/sh", tx).unwrap();
        let result = tm.resize(id, 50, 120);
        assert!(result.is_ok());
    }
}
