use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::mpsc::Sender;

/// Holds a PTY master, its writer (taken once at spawn time), and the shell it
/// is driving — the child is retained so `close` can terminate it rather than
/// leaving an orphaned process behind.
struct PtyHandle {
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

pub struct TerminalManager {
    ptys: Arc<Mutex<HashMap<u32, PtyHandle>>>,
    next_id: Arc<Mutex<u32>>,
}

/// Declare the terminal the frontend actually is, for every spawned shell.
///
/// The child inherits our environment, and a Finder/Dock-launched `.app` has no
/// `TERM` at all — only a shell-launched build does. Without `TERM` zsh's line
/// editor has no terminfo to drive the cursor: Delete emits a bare `' '` instead
/// of `"\x08 \x08"`, so the last character is overwritten but never erased, and
/// zle cannot move the cursor back to redraw a line in place, so it re-emits the
/// whole line after the existing echo — a pasted command appears twice.
/// xterm.js is an xterm-256color emulator with 24-bit colour; say so.
fn apply_pty_env(cmd: &mut CommandBuilder) {
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
}

/// Decode every complete UTF-8 character in `carry`, leaving only a trailing
/// incomplete sequence behind for the next read to finish.
///
/// PTY reads land on arbitrary byte boundaries, so a multi-byte character is
/// routinely split across two chunks. Decoding each chunk independently turns
/// both halves into U+FFFD, which corrupts every box-drawing glyph, powerline
/// separator and emoji a prompt emits. Carrying the partial tail across reads
/// is what makes the stream lossless.
fn drain_utf8(carry: &mut Vec<u8>) -> String {
    let mut out = String::new();
    loop {
        match std::str::from_utf8(carry) {
            Ok(s) => {
                out.push_str(s);
                carry.clear();
                return out;
            }
            Err(e) => {
                let valid = e.valid_up_to();
                out.push_str(std::str::from_utf8(&carry[..valid]).unwrap_or_default());
                match e.error_len() {
                    // Truncated at the end — could still complete; keep it.
                    None => {
                        carry.drain(..valid);
                        return out;
                    }
                    // Genuinely malformed — emit one U+FFFD and step over it.
                    Some(bad) => {
                        out.push('\u{FFFD}');
                        carry.drain(..valid + bad);
                    }
                }
            }
        }
    }
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            ptys: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    pub fn spawn(&self, shell: &str, tx: Sender<(u32, String)>) -> Result<u32> {
        self.spawn_in(shell, None, tx)
    }

    /// Spawn a new terminal, optionally starting in `cwd`.
    pub fn spawn_in(
        &self,
        shell: &str,
        cwd: Option<&std::path::Path>,
        tx: Sender<(u32, String)>,
    ) -> Result<u32> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new(shell);
        apply_pty_env(&mut cmd);
        if let Some(dir) = cwd {
            cmd.cwd(dir);
        }
        let child = pair.slave.spawn_command(cmd)?;

        let mut reader = pair.master.try_clone_reader()?;
        // Take the writer ONCE — reusing it for all subsequent writes prevents
        // each keystroke from being treated as separate input.
        let writer = pair.master.take_writer()?;
        let master = pair.master;

        let id = {
            let mut next_id = self.next_id.lock().unwrap_or_else(|e| e.into_inner());
            let id = *next_id;
            *next_id += 1;
            id
        };

        {
            let mut ptys = self.ptys.lock().unwrap_or_else(|e| e.into_inner());
            ptys.insert(
                id,
                PtyHandle {
                    master,
                    writer,
                    child,
                },
            );
        }

        // Spawn thread to read output
        let tx_clone = tx.clone();
        thread::spawn(move || {
            let mut buffer = [0u8; 1024];
            // Holds at most a 3-byte truncated sequence between reads.
            let mut carry: Vec<u8> = Vec::new();
            loop {
                match reader.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        carry.extend_from_slice(&buffer[..n]);
                        let output = drain_utf8(&mut carry);
                        // A read that lands mid-character yields nothing to send
                        // yet; wait for the bytes that complete it.
                        if !output.is_empty() && tx_clone.blocking_send((id, output)).is_err() {
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
        if let Some(handle) = ptys.get_mut(&id) {
            handle.writer.write_all(data.as_bytes())?;
            handle.writer.flush()?;
        }
        Ok(())
    }

    /// Terminate a terminal and release its PTY.
    ///
    /// Without this, closing the panel left the shell running for the life of
    /// the app: nothing removed the entry from `ptys`, so the master fd stayed
    /// open and the reader thread never saw EOF. Killing the child first stops
    /// the shell; dropping the handle then closes the fd, which ends the reader
    /// thread. `wait` reaps the process rather than leaving a zombie, and only
    /// runs after the map lock is released.
    pub fn close(&self, id: u32) -> Result<()> {
        let handle = {
            let mut ptys = self.ptys.lock().unwrap_or_else(|e| e.into_inner());
            ptys.remove(&id)
        };
        if let Some(mut handle) = handle {
            let _ = handle.child.kill();
            let _ = handle.child.wait();
        }
        Ok(())
    }

    pub fn resize(&self, id: u32, rows: u16, cols: u16) -> Result<()> {
        let mut ptys = self.ptys.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(handle) = ptys.get_mut(&id) {
            handle.master.resize(PtySize {
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

    /// Regression: a Finder-launched .app inherits no TERM, which left zsh's line
    /// editor without terminfo — Delete stopped erasing the last character and a
    /// pasted command echoed twice. env_clear() reproduces that bare environment,
    /// so this asserts our own code supplies TERM rather than the test runner's
    /// shell happening to export one.
    #[test]
    fn pty_env_declares_a_terminal_type_even_with_no_inherited_env() {
        let mut cmd = CommandBuilder::new("zsh");
        cmd.env_clear();
        assert_eq!(cmd.get_env("TERM"), None, "precondition: no inherited TERM");

        apply_pty_env(&mut cmd);

        assert_eq!(
            cmd.get_env("TERM").and_then(|v| v.to_str()),
            Some("xterm-256color")
        );
        assert_eq!(
            cmd.get_env("COLORTERM").and_then(|v| v.to_str()),
            Some("truecolor")
        );
    }

    #[test]
    fn drain_utf8_passes_through_complete_input() {
        let mut carry = b"hello \xe2\x94\x82 world".to_vec();
        assert_eq!(drain_utf8(&mut carry), "hello │ world");
        assert!(carry.is_empty());
    }

    /// The bug this replaced: a character split across two 1024-byte PTY reads
    /// was decoded as two independent chunks, turning both halves into U+FFFD.
    #[test]
    fn drain_utf8_rejoins_a_character_split_across_reads() {
        let glyph = "│".as_bytes(); // 3 bytes: e2 94 82
        let mut carry = vec![b'a'];
        carry.push(glyph[0]);

        // First read ends mid-character: emit only what is complete, keep the tail.
        assert_eq!(drain_utf8(&mut carry), "a");
        assert_eq!(carry, vec![glyph[0]], "partial sequence must be retained");

        // The rest arrives; the character comes back whole, not as replacements.
        carry.extend_from_slice(&glyph[1..]);
        carry.push(b'b');
        assert_eq!(drain_utf8(&mut carry), "│b");
        assert!(carry.is_empty());
    }

    #[test]
    fn drain_utf8_emits_one_replacement_for_genuinely_invalid_bytes() {
        // 0xff can never begin a UTF-8 sequence, so it cannot be a truncated tail.
        let mut carry = vec![b'a', 0xff, b'b'];
        assert_eq!(drain_utf8(&mut carry), "a\u{FFFD}b");
        assert!(carry.is_empty());
    }

    #[test]
    fn drain_utf8_never_accumulates_more_than_a_partial_sequence() {
        // A 4-byte emoji fed one byte at a time: carry must stay bounded and the
        // character must surface exactly once, on the byte that completes it.
        let bytes = "🚀".as_bytes();
        let mut carry = Vec::new();
        let mut seen = String::new();
        for b in bytes {
            carry.push(*b);
            seen.push_str(&drain_utf8(&mut carry));
            assert!(carry.len() < 4, "carry grew to {} bytes", carry.len());
        }
        assert_eq!(seen, "🚀");
        assert!(carry.is_empty());
    }

    /// Regression: closing the panel used to leave the shell running for the life
    /// of the app. Asserts the process is actually gone, not merely forgotten.
    #[test]
    fn close_kills_the_shell_and_is_idempotent() {
        let tm = TerminalManager::new();
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        let id = tm.spawn("sh", tx).expect("spawn sh");

        let pid = {
            let ptys = tm.ptys.lock().unwrap();
            ptys.get(&id)
                .and_then(|h| h.child.process_id())
                .expect("spawned shell should report a pid")
        };
        assert!(process_is_alive(pid), "precondition: shell is running");

        assert!(tm.close(id).is_ok());

        assert!(
            !tm.ptys.lock().unwrap().contains_key(&id),
            "close must drop the handle so the master fd shuts the reader thread down"
        );
        // close() waits on the child, so the process is reaped by the time it
        // returns — no sleep or retry needed here.
        assert!(!process_is_alive(pid), "shell survived close(): pid {pid}");

        // Closing an already-closed (or never-known) id is a no-op, not an error.
        assert!(tm.close(id).is_ok());
        assert!(tm.close(4242).is_ok());
    }

    fn process_is_alive(pid: u32) -> bool {
        std::process::Command::new("ps")
            .args(["-p", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

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
