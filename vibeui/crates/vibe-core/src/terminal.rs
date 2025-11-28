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
            let mut next_id = self.next_id.lock().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        {
            let mut ptys = self.ptys.lock().unwrap();
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
        let mut ptys = self.ptys.lock().unwrap();
        if let Some(master) = ptys.get_mut(&id) {
            write!(master.take_writer()?, "{}", data)?;
        }
        Ok(())
    }

    pub fn resize(&self, id: u32, rows: u16, cols: u16) -> Result<()> {
        let mut ptys = self.ptys.lock().unwrap();
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
