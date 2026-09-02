//! Running a POSIX shell command, including on Windows.
//!
//! The implementation lives in [`vibe_core::shell`], because `vibe-core`'s own
//! [`CommandExecutor`](vibe_core::executor::CommandExecutor) runs the agent's
//! shell commands from inside `vibecli` and needs the same answer. One
//! resolution, one policy: the command strings are POSIX everywhere, so the
//! shell is `sh` everywhere, and the Windows work is *finding* it — Git for
//! Windows ships `sh.exe` but puts only `<Git>\cmd` on `PATH`.
//!
//! This module stays as the crate-local name because the call sites in
//! `commands.rs` and `agent_executor.rs` spell it `crate::shell::…`, and
//! because it is the obvious place to look from inside this crate.

pub use vibe_core::shell::{explain, sh, sh_async};
