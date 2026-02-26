# VibeCLI — Neovim Plugin

AI-assisted coding in Neovim via the [VibeCLI](../vibecli) daemon.

## Requirements

- Neovim ≥ 0.9
- `vibecli serve` running locally (default port 7878)
- `curl` available in `$PATH`

## Installation

### lazy.nvim

```lua
{
  dir = "~/path/to/vibecody/neovim-plugin",
  config = function()
    require("vibecli").setup({
      daemon_url = "http://localhost:7878",
      provider   = "claude",   -- or "openai", "ollama", etc.
      approval   = "suggest",  -- or "full-auto", "manual"
      auto_open  = true,
    })
  end,
}
```

### packer.nvim

```lua
use {
  "~/path/to/vibecody/neovim-plugin",
  config = function()
    require("vibecli").setup()
  end,
}
```

## Commands

| Command | Description |
|---------|-------------|
| `:VibeCLI <task>` | Submit a task to the daemon |
| `:VibeCLIAsk` | Open an input dialog, then submit |
| `:VibeCLIInline` | Send visually-selected lines + a question |
| `:VibeCLIJob` | List recent background jobs |

## Default Keymaps

| Key | Mode | Action |
|-----|------|--------|
| `<leader>va` | Normal | `:VibeCLIAsk` |
| `<leader>vj` | Normal | `:VibeCLIJob` |
| `<leader>vi` | Visual | `:VibeCLIInline` |

Override by mapping before calling `setup()`, or disable by remapping after.

## Starting the Daemon

```bash
# Using an AI provider
vibecli serve --port 7878 --provider claude

# Or Ollama (no API key required)
vibecli serve --port 7878 --provider ollama
```

## Result Buffer

Responses stream into a `*VibeCLI*` split buffer rendered as Markdown. The buffer is
reused across calls; close it with `:q` or `:bd`.
