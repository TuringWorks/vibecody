#!/usr/bin/env python3
"""Second pass: prose corrections where the docs contradicted the implementation.

Each fix is traced to the source file that establishes ground truth.
"""
import os
import sys

R = []


def fix(path, old, new):
    R.append((path, old, new))


# ---------------------------------------------------------------- OpenMemory
# Source: vibecli/vibecli-cli/src/open_memory.rs:1219-1230
#   pub fn encrypt(&self, plaintext: &str) -> Vec<u8> {
#       ... ciphertext.push(byte ^ key_byte ^ nonce_byte);
# The module header at :10 already says "Encryption at rest | Not implemented".
# The user-facing docs claimed AES-256-GCM in 10 places.

fix("docs/memory-guide.md",
    "Enable AES-256-GCM encryption at rest. All existing memories are re-encrypted in place. "
    "The key is stored at `~/.local/share/vibecli/openmemory/.key` (mode 0600). To use a passphrase instead:",

    "Enable at-rest obfuscation. All existing memories are re-encoded in place. The key is stored at "
    "`~/.local/share/vibecli/openmemory/.key` (mode 0600). To use a passphrase instead:\n\n"
    "> ### ⚠️ This is not encryption\n"
    ">\n"
    "> `open_memory.rs` XORs each byte against a repeating 32-byte key and a 12-byte nonce\n"
    "> (`byte ^ key[i % 32] ^ nonce[i % 12]`). A repeating-key XOR is recoverable from known\n"
    "> plaintext and is unauthenticated — it raises the bar against casual disk inspection and\n"
    "> nothing more.\n"
    ">\n"
    "> **Do not store secrets in memory, and do not cite this flag as a compliance control.**\n"
    "> AES-256-GCM is on the roadmap; until it lands, the honest word is obfuscation.\n"
    ">\n"
    "> For anything that actually needs to stay secret, use the ProfileStore\n"
    "> (`~/.vibecli/profile_settings.db`) or the WorkspaceStore\n"
    "> (`<workspace>/.vibecli/workspace.db`) — both are ChaCha20-Poly1305 AEAD with a random\n"
    "> 12-byte nonce per record (`crates/vibe-profile-store/src/lib.rs`).")

fix("docs/memory-guide.md",
    "**What's the encryption overhead?**\n"
    "The current implementation uses XOR-based stream encryption (lightweight, ~5% throughput overhead). "
    "AES-256-GCM is on the roadmap — when it lands, expect ~10-15% throughput overhead and no measurable recall impact.",

    "**What's the encryption overhead?**\n"
    "There is no encryption today, so the overhead question is moot. `/openmemory encrypt` applies a "
    "repeating-key XOR (~5% throughput overhead) which is obfuscation, not a cryptographic guarantee — "
    "see the warning under `/openmemory encrypt` above. AES-256-GCM is on the roadmap; when it lands, "
    "expect ~10-15% throughput overhead and no measurable recall impact.")

fix("docs/configuration.md",
    "### Encryption\n\n"
    "To enable AES-256-GCM encryption at rest, run `/openmemory encrypt` in the REPL or set "
    "`encryption = true` in `config.toml`. The key is stored at `<store>/.key` (mode 0600).",

    "### At-rest obfuscation (not encryption)\n\n"
    "Run `/openmemory encrypt` in the REPL or set `encryption = true` in `config.toml`. The key is "
    "stored at `<store>/.key` (mode 0600).\n\n"
    "> ⚠️ **This is not encryption.** The implementation is a repeating-key XOR "
    "(`open_memory.rs`), not AES-256-GCM. It deters casual disk inspection and provides no "
    "cryptographic or integrity guarantee. Keep secrets in the ProfileStore instead — see "
    "[memory-guide.md](./memory-guide.md) and [security.md](./security.md).")

# ------------------------------------------------------------- Agent: Retry
# Source: vibecoder/src/components/AgentPanel.tsx:296 — retry() calls
# start_agent_task with the original task and no checkpointId, i.e. from scratch.
# resumeAgent() at :338 calls resume_agent_task with lastCheckpointId.
# agent-panel.md:68 and :146 were already correct; the status table at :43 was not.

fix("docs/agent-panel.md",
    "| `error` | Retry button (preserves completed steps) + Reset |",
    "| `error` | Retry button (re-runs the task from the start; the step feed stays on screen for reference) + Reset |")

# --------------------------------------------------------- API key storage
# Source: crates/vibe-profile-store/src/lib.rs:17
#   "Encryption: ChaCha20-Poly1305 (AEAD) — random 12-byte nonce prepended"
# security.md's ranking predated the ProfileStore migration and still listed
# config.toml as a supported option.

fix("docs/security.md",
    "### Storage Options\n\n"
    "API keys can be provided through multiple mechanisms, listed from most secure to least:\n\n"
    "1. **api_key_helper** — A command that returns the key on stdout. Integrates with system keychains, Vault, or AWS Secrets Manager:\n\n"
    "```toml\n"
    "[provider]\n"
    "api_key_helper = \"security find-generic-password -s vibecody-anthropic -w\"\n"
    "```\n\n"
    "1. **Environment variables** — Set in your shell profile or CI environment:\n\n"
    "```bash\n"
    "export ANTHROPIC_API_KEY=\"sk-ant-...\"\n"
    "```\n\n"
    "1. **Config file** — Stored in `~/.vibecli/config.toml`. Ensure the file has restrictive permissions:\n\n"
    "```bash\n"
    "chmod 600 ~/.vibecli/config.toml\n"
    "```",

    "### Storage Options\n\n"
    "> **Never put an API key in `config.toml`, `.env`, or any other plaintext file.** Earlier\n"
    "> revisions of this page listed the config file as a supported option. It is not one — the\n"
    "> encrypted stores replaced it, and any key still sitting in a config file should be moved\n"
    "> and then rotated.\n\n"
    "API keys can be provided through three mechanisms, listed from most secure to least:\n\n"
    "1. **api_key_helper** — a command that returns the key on stdout, so VibeCody persists\n"
    "   nothing at all. Integrates with system keychains, Vault, or AWS Secrets Manager:\n\n"
    "```toml\n"
    "[provider]\n"
    "api_key_helper = \"security find-generic-password -s vibecody-anthropic -w\"\n"
    "```\n\n"
    "2. **Encrypted stores** — the default persistent path, and where `vibecli set-key` and the\n"
    "   VibeCoder Settings → Keys tab write:\n\n"
    "   | Store | Path | Scope |\n"
    "   |---|---|---|\n"
    "   | ProfileStore | `~/.vibecli/profile_settings.db` | account-level keys |\n"
    "   | WorkspaceStore | `<workspace>/.vibecli/workspace.db` | project secrets |\n\n"
    "   Both use **ChaCha20-Poly1305 (AEAD)** with a random 12-byte nonce prepended per record\n"
    "   (`crates/vibe-profile-store/src/lib.rs`). The ProfileStore is machine-bound by design —\n"
    "   do not commit it.\n\n"
    "```bash\n"
    "vibecli set-key anthropic sk-ant-...\n"
    "```\n\n"
    "3. **Environment variables** — a compatibility fallback for CI and containers. Readable by\n"
    "   any process in the same environment and visible in process listings on some platforms,\n"
    "   so prefer one of the two options above on a workstation:\n\n"
    "```bash\n"
    "export ANTHROPIC_API_KEY=\"sk-ant-...\"\n"
    "```")

fix("docs/security.md",
    "- [ ] Use `api_key_helper` instead of plaintext API keys in config files.\n"
    "- [ ] Set `chmod 600` on `~/.vibecli/config.toml`.",

    "- [ ] Use `api_key_helper`, or `vibecli set-key` to write into the encrypted ProfileStore.\n"
    "- [ ] Confirm no API key remains in `~/.vibecli/config.toml` or any `.env` file — and rotate any key that was ever stored there.\n"
    "- [ ] Keep `~/.vibecli/profile_settings.db` out of version control (it is machine-bound).")

fix("docs/security.md",
    "- Use environment variables or `api_key_helper` for API keys rather than hardcoding them in config files.\n"
    "- Avoid committing `.vibecli/config.toml` to version control if it contains API keys.",

    "- Use `api_key_helper` or the encrypted ProfileStore for API keys. Never hardcode them in config files.\n"
    "- `.vibecli/config.toml` should contain no secrets at all, so committing it is safe; `profile_settings.db` and `workspace.db` never belong in version control.")

# ----------------------------------------------------------- Two sandboxes
# Both subsystems exist and are separate:
#   container: vibecli/vibecli-cli/src/{container_runtime,docker_runtime,podman_runtime,
#              container_tool_executor,opensandbox_client}.rs + SandboxConfig in config.rs
#   native  : vibecli/vibecli-cli/src/{sandbox_bwrap,sandbox_windows,sandbox_policy}.rs
# Each page described its own as "the sandbox", which read as a contradiction.

fix("docs/security.md",
    "## Sandbox Isolation\n\n"
    "The sandbox executes agent commands inside a container, preventing access to the host system.\n\n"
    "### Container Runtimes",

    "## Sandbox Isolation\n\n"
    "> **VibeCody has two separate sandbox subsystems. This page documents the opt-in container\n"
    "> sandbox. It is not the same thing as the always-on Tier-0 native sandbox.**\n"
    ">\n"
    "> | | Container sandbox (this page) | Tier-0 native sandbox |\n"
    "> |---|---|---|\n"
    "> | Enabled | opt-in via `[sandbox] enabled = true` | always on for daemon-mediated tool calls |\n"
    "> | Isolation | full container namespace | per-OS process isolation |\n"
    "> | Requires | Docker, Podman, or OpenSandbox | nothing (`bwrap` on Linux for filesystem isolation) |\n"
    "> | Documented in | this page | [sandbox.md](./sandbox.md) |\n"
    ">\n"
    "> Tier-0 applies whether or not you enable the container sandbox, and its coverage is\n"
    "> uneven by platform — **macOS gets network isolation only**. Read\n"
    "> [sandbox.md](./sandbox.md) before relying on either.\n\n"
    "The container sandbox executes agent commands inside a container, preventing access to the host system.\n\n"
    "### Container Runtimes")


def main():
    root = os.path.expanduser(sys.argv[1] if len(sys.argv) > 1 else ".")
    os.chdir(root)
    cache, applied, missed = {}, 0, []
    for path, old, new in R:
        if path not in cache:
            if not os.path.exists(path):
                missed.append((path, "FILE MISSING"))
                cache[path] = None
                continue
            cache[path] = open(path, encoding="utf-8", errors="ignore").read()
        s = cache[path]
        if s is None:
            continue
        if old not in s:
            missed.append((path, old.split("\n")[0][:90]))
            continue
        cache[path] = s.replace(old, new, 1)
        applied += 1
    written = 0
    for p, s in cache.items():
        if s is None:
            continue
        if open(p, encoding="utf-8", errors="ignore").read() != s:
            open(p, "w", encoding="utf-8").write(s)
            written += 1
    print(f"prose fixes applied : {applied}/{len(R)}")
    print(f"files written       : {written}")
    for m in missed:
        print("  MISS", m[0], "|", m[1])


if __name__ == "__main__":
    main()
