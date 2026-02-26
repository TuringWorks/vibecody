--- cmp_vibecli — nvim-cmp completion source for VibeCLI
---
--- Provides two kinds of completions:
---   1. Slash-command completions (type `/` at the start of a line in the *VibeCLI* buffer)
---   2. @ context completions (@file:, @web:, @docs:, @git) in any buffer
---
--- Installation with lazy.nvim + nvim-cmp:
---   dependencies = { "hrsh7th/nvim-cmp", dir = "path/to/vibecody/neovim-plugin" }
---   config = function()
---     require("vibecli").setup()
---     require("cmp").register_source("vibecli", require("cmp_vibecli").new())
---     -- add "vibecli" to sources for all buffers or just the VibeCLI buffer:
---     require("cmp").setup.buffer({
---       sources = { { name = "vibecli" } },
---     })
---   end
---
--- Or to enable it globally:
---   require("cmp").setup({
---     sources = require("cmp").config.sources({
---       { name = "nvim_lsp" },
---       { name = "buffer" },
---       { name = "vibecli", priority = 100 },
---     }),
---   })

local M = {}

-- ── Slash-command items ───────────────────────────────────────────────────────

local SLASH_COMMANDS = {
  { label = "/agent",         detail = "Run autonomous coding agent",              insert = "/agent " },
  { label = "/chat",          detail = "Single-turn chat with AI",                 insert = "/chat " },
  { label = "/plan",          detail = "Generate a plan, then run agent",          insert = "/plan " },
  { label = "/resume",        detail = "Resume a previous session",                insert = "/resume " },
  { label = "/snippet list",  detail = "List saved code snippets",                 insert = "/snippet list" },
  { label = "/snippet save",  detail = "Save last AI response as snippet",         insert = "/snippet save " },
  { label = "/snippet use",   detail = "Inject snippet as context",                insert = "/snippet use " },
  { label = "/snippet show",  detail = "Display snippet contents",                 insert = "/snippet show " },
  { label = "/linear list",   detail = "List open Linear issues",                  insert = "/linear list" },
  { label = "/linear new",    detail = "Create a new Linear issue",                insert = "/linear new \"" },
  { label = "/linear open",   detail = "Open Linear issue in browser",             insert = "/linear open " },
  { label = "/linear attach", detail = "Link session to Linear issue",             insert = "/linear attach " },
  { label = "/model",         detail = "Switch AI provider / model",               insert = "/model " },
  { label = "/cost",          detail = "Show session token cost",                  insert = "/cost" },
  { label = "/context",       detail = "Show context window usage",                insert = "/context" },
  { label = "/status",        detail = "Show session status",                      insert = "/status" },
  { label = "/fork",          detail = "Fork current conversation",                insert = "/fork " },
  { label = "/rewind",        detail = "Save / restore conversation checkpoint",   insert = "/rewind" },
  { label = "/schedule",      detail = "Schedule recurring tasks",                 insert = "/schedule " },
  { label = "/remind",        detail = "Set a one-time reminder",                  insert = "/remind in " },
  { label = "/index",         detail = "Build semantic codebase index",            insert = "/index" },
  { label = "/qa",            detail = "Q&A over codebase",                        insert = "/qa " },
  { label = "/jobs",          detail = "List background jobs",                     insert = "/jobs" },
  { label = "/help",          detail = "Show help",                                insert = "/help" },
  { label = "/exit",          detail = "Exit VibeCLI",                             insert = "/exit" },
}

-- ── @ context items ───────────────────────────────────────────────────────────

local AT_ITEMS = {
  { label = "@file:",     detail = "Inject file contents",          insert = "@file:" },
  { label = "@web:",      detail = "Fetch and inject a URL",        insert = "@web:" },
  { label = "@docs:",     detail = "Fetch library docs",            insert = "@docs:" },
  { label = "@docs:rs:",  detail = "Rust crate docs (docs.rs)",     insert = "@docs:rs:" },
  { label = "@docs:py:",  detail = "Python package docs (PyPI)",    insert = "@docs:py:" },
  { label = "@docs:npm:", detail = "npm package docs",              insert = "@docs:npm:" },
  { label = "@git",       detail = "Inject git status + log",       insert = "@git" },
}

-- ── Source implementation ─────────────────────────────────────────────────────

---@class CmpVibecliSource
local Source = {}
Source.__index = Source

function Source.new()
  return setmetatable({}, Source)
end

--- Source name displayed by nvim-cmp.
function Source:get_debug_name()
  return "vibecli"
end

--- Trigger characters: `/` for commands, `@` for context refs.
function Source:get_trigger_characters()
  return { "/", "@" }
end

--- Provide completions.
function Source:complete(params, callback)
  local items = {}
  local cursor_before = params.context.cursor_before_line or ""

  -- ── @ completions ─────────────────────────────────────────────────────────
  if cursor_before:match("@[%w_:.]*$") then
    for _, item in ipairs(AT_ITEMS) do
      table.insert(items, {
        label            = item.label,
        detail           = item.detail,
        insertText       = item.insert,
        kind             = require("cmp").lsp.CompletionItemKind.Keyword,
        filterText       = item.label,
        sortText         = "0" .. item.label,
      })
    end
    callback({ items = items, isIncomplete = false })
    return
  end

  -- ── / completions (only at start of line) ─────────────────────────────────
  if cursor_before:match("^%s*/" ) then
    local prefix = cursor_before:match("^%s*(/[%w%s_-]*)$") or "/"
    for _, item in ipairs(SLASH_COMMANDS) do
      if item.label:sub(1, #prefix) == prefix then
        table.insert(items, {
          label            = item.label,
          detail           = item.detail,
          insertText       = item.insert,
          kind             = require("cmp").lsp.CompletionItemKind.Function,
          filterText       = item.label,
          sortText         = "0" .. item.label,
        })
      end
    end
    callback({ items = items, isIncomplete = false })
    return
  end

  -- Not triggered by `/` or `@` — return empty
  callback({ items = {}, isIncomplete = false })
end

--- Resolve additional item details (e.g., documentation).
function Source:resolve(completion_item, callback)
  callback(completion_item)
end

-- ── Module entry point ────────────────────────────────────────────────────────

M.new = Source.new

--- Auto-register when nvim-cmp is available (called by vibecli.setup if user
--- has nvim-cmp installed).
function M.register_if_available()
  local ok, cmp = pcall(require, "cmp")
  if ok then
    cmp.register_source("vibecli", Source.new())
  end
end

return M
