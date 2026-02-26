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

local M = {}

-- ── Default configuration ─────────────────────────────────────────────────────

M.config = {
  daemon_url = "http://localhost:7878",
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

--- POST JSON to the daemon using curl. Returns response body string or nil.
local function post_json(path, body_table)
  local json_str = vim.fn.json_encode(body_table)
  -- Escape single quotes for shell
  json_str = json_str:gsub("'", "'\\''")
  local cmd = string.format(
    "curl -s -X POST '%s%s' -H 'Content-Type: application/json' -d '%s'",
    M.config.daemon_url, path, json_str
  )
  return sh(cmd)
end

--- GET from the daemon. Returns response body string or nil.
local function get_json(path)
  local cmd = string.format("curl -s '%s%s'", M.config.daemon_url, path)
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
  local url = M.config.daemon_url .. "/stream/" .. session_id
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
                table.insert(lines, "✅ Complete: " .. (ev.summary or "done"))
                vim.schedule(function()
                  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
                  vim.notify("[VibeCLI] Task complete.", vim.log.levels.INFO)
                end)
              elseif ev.type == "error" then
                table.insert(lines, "")
                table.insert(lines, "❌ Error: " .. (ev.message or "unknown"))
                vim.schedule(function()
                  vim.api.nvim_buf_set_lines(bufnr, 0, -1, false, lines)
                  vim.notify("[VibeCLI] Task failed: " .. (ev.message or ""), vim.log.levels.ERROR)
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
