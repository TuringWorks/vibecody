#!/usr/bin/env node
// Run the Tauri CLI with `~/.cargo/bin` on PATH, on every platform.
//
// The npm scripts in all three desktop shells used to spell this inline as
// `PATH="$HOME/.cargo/bin:$PATH" tauri dev`. That is bash syntax, and npm runs
// scripts through `cmd.exe` on Windows, where `PATH` is a *builtin command*
// rather than an assignment: cmd set PATH to the literal string
// `="$HOME/.cargo/bin:$PATH" tauri dev`, never launched the Tauri CLI, and
// exited 0. A build that does nothing and reports success is worse than one
// that fails, so the prefix moved here where it can be written once and be
// correct everywhere.
//
// Usage: node ../scripts/tauri.mjs <dev|build|...> [args...]

import { spawn } from 'node:child_process'
import { homedir } from 'node:os'
import { join, delimiter } from 'node:path'

const cargoBin = join(homedir(), '.cargo', 'bin')

// Prepend rather than replace: a developer who keeps cargo somewhere else still
// has their own PATH behind ours, and a duplicate entry is harmless.
const env = { ...process.env }

// Windows environment variables are case-insensitive, and Node surfaces the
// variable as `Path` there. Writing a second `PATH` key alongside it hands the
// child two spellings of the same variable and lets the OS pick — so replace
// whichever key is already present instead of adding one.
const pathKey = Object.keys(env).find((k) => k.toUpperCase() === 'PATH') ?? 'PATH'
env[pathKey] = [cargoBin, env[pathKey] ?? ''].filter(Boolean).join(delimiter)

// npm puts the local .bin on PATH for scripts it runs, so a bare `tauri`
// resolves — except on Windows, where the executable is `tauri.cmd` and only a
// shell lookup finds it. `shell: true` covers both without hardcoding paths.
const child = spawn('tauri', process.argv.slice(2), { stdio: 'inherit', shell: true, env })

child.on('error', (err) => {
  console.error(`failed to launch the Tauri CLI: ${err.message}`)
  process.exit(1)
})

// A signal-terminated child reports a null code. Report the conventional 128+n
// so a Ctrl-C is not read as success by a CI runner; SIGINT is the only signal
// that realistically lands here, and 130 is what a shell would have returned.
child.on('exit', (code, signal) => {
  if (code !== null) process.exit(code)
  process.exit(signal === 'SIGINT' ? 130 : signal === 'SIGTERM' ? 143 : 1)
})
