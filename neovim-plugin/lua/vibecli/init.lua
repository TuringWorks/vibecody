--- VibeCLI Neovim Plugin
--- Connects to the VibeCLI daemon (vibecli serve) for AI-assisted coding.
---
--- Installation (lazy.nvim):
---   { dir = "path/to/vibecody/neovim-plugin", config = true }
---
--- Installation (packer):
---   use { "path/to/vibecody/neovim-plugin", config = function() require("vibecli").setup() end }
---
--- Default config:
---   require("vibecli").setup({
---     daemon_url = "http://localhost:7878",
---     provider   = "claude",
---     approval   = "suggest",
---     auto_open  = true,   -- open result buffer after task
---   })
---
--- Commands:
---   :VibeCLI <task>          — Submit a task to the daemon
---   :VibeCLIJob              — Show recent background jobs
---   :VibeCLIAsk              — Prompt and submit via input dialog
---   :VibeCLIInline           — Send selected lines as context + ask question
---   :VibeCLIVoice            — Dictate: run once to start, again to stop
---                              (:VibeCLIVoice! submits the transcript as a task)
---                              Requires SoX (`brew install sox`).

local M = {}

-- ── Default configuration ─────────────────────────────────────────────────────

M.config = {
  daemon_url = "http://localhost:7878",
  -- Bearer token for the daemon. Leave nil for the zero-config path: the
  -- plugin reads ~/.vibecli/daemon.token, where `vibecli --serve` writes it.
  token      = nil,
  provider   = "claude",
  approval   = "suggest",
  auto_open  = true,
}

-- ── Utilities ─────────────────────────────────────────────────────────────────

--- Execute a shell command and return stdout as a string (or nil + error).
local function sh(cmd)
  local handle = io.popen(cmd .. " 2>&1")
  if not handle then return nil, "popen failed" end
  local out = handle:read("*a")
  handle:close()
  return out
end

--- URL-encode a string.
local function urlencode(s)
  return s:gsub("[^%w%-_%.~]", function(c)
    return string.format("%%%02X", c:byte())
  end)
end

--- Resolve the daemon bearer token.
---
--- Nearly every daemon route is behind `require_auth`; only `/health` and a
--- handful of others are public. Without this the plugin's `/agent`, `/jobs`
--- and `/voice/*` calls all return 401 — which they silently did, since curl
--- was never given a credential.
---
--- Order matches every other client: config, then `VIBECLI_TOKEN`, then
--- `VIBECLI_DAEMON_TOKEN`, then `~/.vibecli/daemon.token` where
--- `vibecli --serve` writes it. nil is legitimate (a daemon may run open).
local function daemon_token()
  if M.config.token and M.config.token ~= "" then return M.config.token end
  for _, name in ipairs({ "VIBECLI_TOKEN", "VIBECLI_DAEMON_TOKEN" }) do
    local env = vim.env[name]
    if env and env ~= "" then return env end
  end
  local path = vim.fn.expand("~/.vibecli/daemon.token")
  if vim.fn.filereadable(path) == 1 then
    local first = (vim.fn.readfile(path) or {})[1]
    if first then
      local trimmed = vim.trim(first)
      if trimmed ~= "" then return trimmed end
    end
  end
  return nil
end

--- `-H 'Authorization: Bearer …'` for curl, or "" when there is no token.
local function auth_header()
  local token = daemon_token()
  if not token then return "" end
  return string.format("-H 'Authorization: Bearer %s' ", token:gsub("'", "'\\''"))
end

--- POST JSON to the daemon using curl. Returns response body string or nil.
local function post_json(path, body_table)
  local json_str = vim.fn.json_encode(body_table)
  -- Escape single quotes for shell
  json_str = json_str:gsub("'", "'\\''")
  local cmd = string.format(
    "curl -s -X POST '%s%s' %s-H 'Content-Type: application/json' -d '%s'",
    M.config.daemon_url, path, auth_header(), json_str
  )
  return sh(cmd)
end

--- GET from the daemon. Returns response body string or nil.
local function get_json(path)
  local cmd = string.format("curl -s %s'%s%s'", auth_header(), M.config.daemon_url, path)
  return sh(cmd)
end

--- Check if the daemon is reachable.
local function daemon_ok()
  local out = sh(string.format("curl -s -o /dev/null -w '%%{http_code}' '%s/health'", M.config.daemon_url))
  return out and out:match("^2%d%d") ~= nil
end

-- ── Result buffer ─────────────────────────────────────────────────────────────

--- Open (or reuse) a scratch buffer named *VibeCLI* and write lines to it.
local function open_result_buf(lines)
  -- Find or create the buffer
  local bufnr = nil
  for _, b in ipairs(vim.api.nvim_list_bufs()) do
    if vim.api.nvim_buf_get_name(b):match("%*VibeCLI%*$") then
      bufnr = b
      break
    end
  end
  if not bufnr then
    bufnr = vim.api.nvim_create_buf(false, true)
    vim.api.nvim_buf_set_name(bufnr, "*VibeCLI*")
    vim.bo[bufnr].buftype  = "nofile"
    vim.bo[bufnr].bufhidden = "hide"
    vim.bo[bufnr].swapfile = false
    vim.bo[bufnr].filetype = "markdown"
  end

  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)

  -- Show in a split if not already visible
  local found = false
  for _, win in ipairs(vim.api.nvim_list_wins()) do
    if vim.api.nvim_win_get_buf(win) == bufnr then
      found = true
      break
    end
  end
  if not found then
    vim.cmd("botright split")
    vim.api.nvim_win_set_buf(0, bufnr)
    vim.api.nvim_win_set_height(0, 16)
  end
  return bufnr
end

-- ── Core: submit a task ───────────────────────────────────────────────────────

--- Submit a task string to the VibeCLI daemon.
--- @param task string  The natural-language task description
--- @param extra_context string|nil  Optional extra context prepended to task
function M.submit_task(task, extra_context)
  if not daemon_ok() then
    vim.notify(
      "[VibeCLI] Daemon not reachable at " .. M.config.daemon_url ..
      "\nRun: vibecli serve --port 7878",
      vim.log.levels.ERROR
    )
    return
  end

  local full_task = task
  if extra_context and extra_context ~= "" then
    full_task = extra_context .. "\n\n" .. task
  end

  local resp = post_json("/agent", {
    task     = full_task,
    provider = M.config.provider,
    approval = M.config.approval,
  })

  if not resp then
    vim.notify("[VibeCLI] No response from daemon", vim.log.levels.ERROR)
    return
  end

  local ok, data = pcall(vim.fn.json_decode, resp)
  if not ok or not data then
    vim.notify("[VibeCLI] Daemon error: " .. resp, vim.log.levels.ERROR)
    return
  end

  local session_id = data.session_id
  vim.notify("[VibeCLI] Job started — session: " .. (session_id or "?"), vim.log.levels.INFO)

  if not M.config.auto_open or not session_id then return end

  -- Stream SSE events into the result buffer
  M._stream_session(session_id)
end

-- ── SSE streaming into buffer ─────────────────────────────────────────────────

--- Stream a running session into the *VibeCLI* buffer using curl + SSE.
--- @param session_id string
function M._stream_session(session_id)
  -- `?token=` rather than a header: the daemon accepts either, and this keeps
  -- the streaming curl invocation a plain argv list with no header plumbing.
  local url = M.config.daemon_url .. "/stream/" .. session_id
  local stream_token = daemon_token()
  if stream_token then url = url .. "?token=" .. urlencode(stream_token) end
  local lines = { "# VibeCLI — session " .. session_id, "" }
  local bufnr = open_result_buf(lines)

  -- Use jobstart to stream curl in background
  local chunk_buf = ""
  vim.fn.jobstart({ "curl", "-sN", url }, {
    on_stdout = function(_, data, _)
      for _, raw in ipairs(data) do
        chunk_buf = chunk_buf .. raw
        -- SSE lines look like: data: {"type":"chunk","content":"..."}
        for line in chunk_buf:gmatch("[^\n]+") do
          if line:match("^data: ") then
            local json_str = line:sub(7)
            local ok2, ev = pcall(vim.fn.json_decode, json_str)
            if ok2 and ev then
              if ev.type == "chunk" and ev.content then
                -- Append to last line or add new line
                local last = lines[#lines]
                if last == "" or last:match("\n$") then
                  table.insert(lines, ev.content)
                else
                  lines[#lines] = last .. ev.content
                end
                vim.schedule(function()
                  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
                end)
              elseif ev.type == "complete" then
                table.insert(lines, "")
                table.insert(lines, "---")
                table.insert(lines, "✅ Complete: " .. (ev.content or "done"))
                vim.schedule(function()
                  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
                  vim.notify("[VibeCLI] Task complete.", vim.log.levels.INFO)
                end)
              elseif ev.type == "partial" then
                -- Terminal, but the agent stopped with planned work left.
                -- Reported as a warning, never as "✅ Complete".
                -- `%d` throws on a non-integral float, and a throw in this
                -- handler kills the whole stream — floor what JSON gave us.
                local done = math.floor(tonumber(ev.steps_completed) or 0)
                local planned = math.floor(tonumber(ev.steps_planned) or 0)
                table.insert(lines, "")
                table.insert(lines, "---")
                table.insert(lines, ("⚠ Incomplete — %d/%d planned steps done"):format(done, planned))
                if ev.content and ev.content ~= "" then
                  table.insert(lines, ev.content)
                end
                for i, item in ipairs(ev.remaining_plan or {}) do
                  table.insert(lines, ("   %d. %s"):format(done + i, item))
                end
                vim.schedule(function()
                  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
                  vim.notify(
                    ("[VibeCLI] Task incomplete — %d/%d steps done."):format(done, planned),
                    vim.log.levels.WARN
                  )
                end)
              elseif ev.type == "retry" then
                -- Non-terminal: without this the buffer just stops updating
                -- for the whole backoff and looks hung.
                table.insert(lines, ("⟳ Retrying (%d/%d) in %.1fs — %s"):format(
                  math.floor(tonumber(ev.attempt) or 0) + 1,
                  math.floor(tonumber(ev.max_attempts) or 0),
                  (tonumber(ev.backoff_ms) or 0) / 1000, ev.content or ""
                ))
                vim.schedule(function()
                  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
                end)
              elseif ev.type == "error" then
                table.insert(lines, "")
                table.insert(lines, "❌ Error: " .. (ev.content or "unknown"))
                vim.schedule(function()
                  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
                  vim.notify("[VibeCLI] Task failed: " .. (ev.content or ""), vim.log.levels.ERROR)
                end)
              end
            end
          end
        end
        -- Keep only the part after the last newline for next iteration
        chunk_buf = chunk_buf:match("[^\n]*$") or ""
      end
    end,
    on_exit = function(_, code, _)
      if code ~= 0 then
        vim.schedule(function()
          vim.notify("[VibeCLI] Stream ended (curl exit " .. code .. ")", vim.log.levels.WARN)
        end)
      end
    end,
    stdout_buffered = false,
  })
end

-- ── Commands ──────────────────────────────────────────────────────────────────

--- :VibeCLI <task>
local function cmd_vibecli(opts)
  local task = opts.args or ""
  if task == "" then
    vim.notify("[VibeCLI] Usage: :VibeCLI <task description>", vim.log.levels.WARN)
    return
  end
  M.submit_task(task)
end

--- :VibeCLIAsk  — opens a floating prompt, then submits.
local function cmd_ask(_opts)
  vim.ui.input({ prompt = "VibeCLI task: " }, function(input)
    if input and input ~= "" then
      M.submit_task(input)
    end
  end)
end

--- :VibeCLIInline  — sends visually selected lines as context + a follow-up question.
local function cmd_inline(_opts)
  -- Get selected lines (works in both normal and visual mode)
  local start_line = vim.fn.line("'<")
  local end_line   = vim.fn.line("'>")
  local sel_lines  = vim.api.nvim_buf_get_lines(0, start_line - 1, end_line, false)
  local filename   = vim.api.nvim_buf_get_name(0)
  local context    = string.format(
    "File: %s (lines %d-%d)\n```\n%s\n```",
    filename, start_line, end_line, table.concat(sel_lines, "\n")
  )

  vim.ui.input({ prompt = "Ask about selection: " }, function(input)
    if input and input ~= "" then
      M.submit_task(input, context)
    end
  end)
end

--- :VibeCLIJob  — show recent jobs table in the result buffer.
local function cmd_jobs(_opts)
  if not daemon_ok() then
    vim.notify("[VibeCLI] Daemon not reachable — run: vibecli serve", vim.log.levels.ERROR)
    return
  end

  local resp = get_json("/jobs")
  if not resp then
    vim.notify("[VibeCLI] No response from daemon", vim.log.levels.ERROR)
    return
  end

  local ok, jobs = pcall(vim.fn.json_decode, resp)
  if not ok or type(jobs) ~= "table" then
    vim.notify("[VibeCLI] Could not parse jobs: " .. resp, vim.log.levels.ERROR)
    return
  end

  local lines = { "# VibeCLI — Background Jobs", "" }
  if #jobs == 0 then
    table.insert(lines, "_No jobs found._")
  else
    table.insert(lines, string.format("%-36s  %-9s  %s", "SESSION ID", "STATUS", "TASK"))
    table.insert(lines, string.rep("-", 80))
    for _, j in ipairs(jobs) do
      local status_icon = j.status == "complete" and "✅"
        or j.status == "running"  and "🟡"
        or j.status == "failed"   and "❌"
        or j.status == "cancelled" and "⛔"
        or "❓"
      local task_preview = (j.task or ""):sub(1, 60)
      if #(j.task or "") > 60 then task_preview = task_preview .. "…" end
      table.insert(lines, string.format(
        "%-36s  %s %-7s  %s",
        j.session_id or "?", status_icon, j.status or "?", task_preview
      ))
    end
  end

  open_result_buf(lines)
end

-- ── Voice input ───────────────────────────────────────────────────────────────
--
-- Capture goes through SoX's `rec`, the same strategy VoiceDispatcher::listen
-- uses in the CLI and the VS Code / JetBrains clients use — one recording path
-- for every non-browser client, and one documented dependency.

--- Install guidance, mirroring `voice.rs`.
local SOX_INSTALL_HINT = table.concat({
  "[VibeCLI] Voice input needs SoX. Install it:",
  "  macOS:   brew install sox",
  "  Linux:   sudo apt install sox",
  "  Windows: choco install sox",
}, "\n")

--- Active recording: `{ job = <job id>, path = <wav path> }`, or nil when idle.
local recording = nil

--- Send a WAV to the daemon and hand the transcript to `on_text`.
local function transcribe_wav(path, on_text)
  local cmd = string.format(
    "curl -s -w '\\n%%{http_code}' -X POST '%s/voice/transcribe' %s-H 'Content-Type: audio/wav' --data-binary @'%s'",
    M.config.daemon_url, auth_header(), path
  )
  local out = sh(cmd)
  os.remove(path)
  if not out then
    vim.notify("[VibeCLI] Transcription request failed", vim.log.levels.ERROR)
    return
  end

  -- curl's -w appends the status on its own last line.
  local body, status = out:match("^(.*)\n(%d+)%s*$")
  if not status then
    vim.notify("[VibeCLI] Unreadable transcription response: " .. out, vim.log.levels.ERROR)
    return
  end

  local ok, decoded = pcall(vim.fn.json_decode, body)
  if status ~= "200" then
    -- The daemon's voice errors are setup guidance ("run /voice download
    -- base", "set GROQ_API_KEY"); show them rather than the status code.
    local msg = (ok and type(decoded) == "table" and decoded.error)
      or ("HTTP " .. status)
    vim.notify("[VibeCLI] " .. msg, vim.log.levels.ERROR)
    return
  end
  if not ok or type(decoded) ~= "table" or type(decoded.text) ~= "string" then
    vim.notify("[VibeCLI] Daemon returned no transcript", vim.log.levels.ERROR)
    return
  end

  local text = vim.trim(decoded.text)
  if text == "" then
    vim.notify("[VibeCLI] No speech was recognised", vim.log.levels.WARN)
    return
  end
  on_text(text)
end

--- :VibeCLIVoice  — start dictating, or stop and transcribe if already recording.
---
--- With `!`, the transcript is submitted as a task instead of inserted at the
--- cursor.
local function cmd_voice(opts)
  local as_task = opts and opts.bang

  if recording then
    local active = recording
    recording = nil
    -- SIGINT, not kill: SoX finalises the WAV header on SIGINT. Killed
    -- outright it leaves a header claiming zero frames, which decodes as an
    -- empty file.
    vim.fn.jobstop(active.job)
    vim.notify("[VibeCLI] Transcribing…", vim.log.levels.INFO)
    -- Give SoX a moment to flush and close the file before reading it.
    vim.defer_fn(function()
      transcribe_wav(active.path, function(text)
        if as_task then
          M.submit_task(text)
        else
          vim.api.nvim_put({ text }, "c", true, true)
        end
      end)
    end, 300)
    return
  end

  if vim.fn.executable("rec") == 0 then
    vim.notify(SOX_INSTALL_HINT, vim.log.levels.ERROR)
    return
  end
  if not daemon_ok() then
    vim.notify("[VibeCLI] Daemon not reachable — run: vibecli serve", vim.log.levels.ERROR)
    return
  end

  -- PID-suffixed: a fixed /tmp path collides between concurrent nvim instances.
  local path = string.format("%s/vibecli-voice-%d-%d.wav", vim.fn.stdpath("cache"), vim.fn.getpid(), os.time())
  -- 16 kHz mono is what every whisper backend resamples to anyway; `trim` caps
  -- a forgotten recording at five minutes.
  local job = vim.fn.jobstart(
    { "rec", path, "rate", "16000", "channels", "1", "trim", "0", "300" },
    { on_exit = function() end }
  )
  if job <= 0 then
    vim.notify(SOX_INSTALL_HINT, vim.log.levels.ERROR)
    return
  end
  recording = { job = job, path = path }
  vim.notify("[VibeCLI] Listening… run :VibeCLIVoice again to stop.", vim.log.levels.INFO)
end

-- ── Setup ─────────────────────────────────────────────────────────────────────

--- Initialize the plugin. Call require("vibecli").setup(opts) in your config.
--- @param opts table|nil  Partial config override
function M.setup(opts)
  M.config = vim.tbl_deep_extend("force", M.config, opts or {})

  -- Auto-register nvim-cmp source if nvim-cmp is installed
  require("cmp_vibecli").register_if_available()

  -- Register user commands
  vim.api.nvim_create_user_command("VibeCLI",       cmd_vibecli, { nargs = "+", desc = "Submit task to VibeCLI daemon" })
  vim.api.nvim_create_user_command("VibeCLIAsk",    cmd_ask,     { nargs = 0,  desc = "Prompt and submit task" })
  vim.api.nvim_create_user_command("VibeCLIInline", cmd_inline,  { nargs = 0, range = true, desc = "Send selection + question to VibeCLI" })
  vim.api.nvim_create_user_command("VibeCLIJob",    cmd_jobs,    { nargs = 0,  desc = "List background VibeCLI jobs" })
  vim.api.nvim_create_user_command("VibeCLIVoice",  cmd_voice,   { nargs = 0, bang = true, desc = "Dictate: run once to start, again to stop (! submits as a task)" })

  -- Optional default keymaps (only if not already mapped)
  local function map(mode, lhs, rhs, desc)
    if vim.fn.mapcheck(lhs, mode) == "" then
      vim.keymap.set(mode, lhs, rhs, { silent = true, desc = desc })
    end
  end
  map("n", "<leader>va", ":VibeCLIAsk<CR>",    "VibeCLI: Ask a task")
  map("n", "<leader>vj", ":VibeCLIJob<CR>",    "VibeCLI: Show jobs")
  map("v", "<leader>vi", ":VibeCLIInline<CR>", "VibeCLI: Inline question on selection")
end

return M
