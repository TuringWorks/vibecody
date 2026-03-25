//! Browser-based zero-install web client for VibeCody.
//!
//! Serves a self-contained single-page application that connects to the
//! `vibecli serve` REST/SSE API, allowing users to interact with VibeCody
//! from any browser without installing the desktop app.
//!
//! Similar in spirit to Bolt.new, v0, or Replit's web interface — but fully
//! air-gap safe: **no external CDN links, no external JS/CSS**.

use serde::{Deserialize, Serialize};

// ── Configuration ───────────────────────────────────────────────────────────

/// Configuration for the browser-based web client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebClientConfig {
    /// Whether the web client is enabled.
    pub enabled: bool,
    /// Page title shown in the browser tab.
    pub title: String,
    /// Color theme — "dark" or "light".
    pub theme: String,
    /// Maximum number of messages to keep in the UI history.
    pub max_message_history: usize,
    /// Allow users to upload files as context.
    pub enable_file_upload: bool,
    /// Allow agent mode (autonomous tool use).
    pub enable_agent_mode: bool,
}

impl Default for WebClientConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            title: "VibeCody Web".to_string(),
            theme: "dark".to_string(),
            max_message_history: 100,
            enable_file_upload: true,
            enable_agent_mode: true,
        }
    }
}

// ── Favicon ─────────────────────────────────────────────────────────────────

/// Returns an SVG favicon — a simple "V" in the accent color.
pub fn web_client_favicon_svg() -> &'static str {
    r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
  <rect width="64" height="64" rx="12" fill="#7c5dfa"/>
  <text x="32" y="46" font-family="system-ui,sans-serif" font-size="40" font-weight="bold" fill="#fff" text-anchor="middle">V</text>
</svg>"##
}

// ── Static assets ───────────────────────────────────────────────────────────

/// Returns a list of `(path, content_type, content)` tuples for static assets
/// that should be served alongside the main HTML page.
pub fn web_client_assets() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        (
            "/favicon.svg",
            "image/svg+xml",
            web_client_favicon_svg(),
        ),
        (
            "/manifest.json",
            "application/json",
            r##"{
  "name": "VibeCody Web",
  "short_name": "VibeCody",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#1a1a2e",
  "theme_color": "#7c5dfa",
  "icons": [
    {
      "src": "/favicon.svg",
      "sizes": "any",
      "type": "image/svg+xml"
    }
  ]
}"##,
        ),
    ]
}

// ── Main HTML generator ─────────────────────────────────────────────────────

/// Generates the complete self-contained single-page HTML application.
///
/// The returned string contains all CSS and JavaScript inline — no external
/// dependencies whatsoever. Safe for air-gapped deployments.
pub fn web_client_html(config: &WebClientConfig) -> String {
    let title = html_escape(&config.title);
    let theme = html_escape(&config.theme);
    let max_history = config.max_message_history;
    let file_upload_enabled = config.enable_file_upload;
    let agent_mode_enabled = config.enable_agent_mode;

    format!(
        r##"<!DOCTYPE html>
<html lang="en" data-theme="{theme}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; img-src 'self' data:;">
<title>{title}</title>
<link rel="icon" type="image/svg+xml" href="/favicon.svg">
<link rel="manifest" href="/manifest.json">
<style>
/* ── Reset & Base ─────────────────────────────────────────────────── */
*,*::before,*::after{{box-sizing:border-box;margin:0;padding:0}}
:root{{
  --bg:#1a1a2e;--bg-card:#16213e;--bg-input:#0f3460;
  --accent:#7c5dfa;--accent-hover:#6c4de6;
  --text:#e2e8f0;--text-muted:#94a3b8;--text-code:#a5f3fc;
  --border:#334155;--danger:#ef4444;--success:#22c55e;
  --radius:8px;--font-sans:system-ui,-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;
  --font-mono:'SF Mono',SFMono-Regular,ui-monospace,Menlo,Consolas,monospace;
  --font-size:14px;
}}
html[data-theme="light"]{{
  --bg:#f8fafc;--bg-card:#ffffff;--bg-input:#e2e8f0;
  --text:#1e293b;--text-muted:#64748b;--text-code:#0369a1;
  --border:#cbd5e1;
}}
html,body{{height:100%;font-family:var(--font-sans);font-size:var(--font-size);
  color:var(--text);background:var(--bg);overflow:hidden}}

/* ── Layout ───────────────────────────────────────────────────────── */
#app{{display:flex;flex-direction:column;height:100vh}}
#topbar{{display:flex;align-items:center;gap:12px;padding:8px 16px;
  background:var(--bg-card);border-bottom:1px solid var(--border);flex-shrink:0}}
#topbar .logo{{font-weight:700;font-size:18px;color:var(--accent)}}
#topbar .spacer{{flex:1}}
#status-dot{{width:10px;height:10px;border-radius:50%;background:var(--danger);
  transition:background .3s}}
#status-dot.connected{{background:var(--success)}}
#settings-btn{{background:none;border:none;color:var(--text-muted);cursor:pointer;
  font-size:18px;padding:4px 8px}}
#settings-btn:hover{{color:var(--text)}}

#main{{display:flex;flex:1;overflow:hidden}}

/* ── Sidebar ──────────────────────────────────────────────────────── */
#sidebar{{width:260px;background:var(--bg-card);border-right:1px solid var(--border);
  display:flex;flex-direction:column;flex-shrink:0}}
#sidebar .hdr{{padding:12px 16px;font-weight:600;font-size:13px;
  color:var(--text-muted);text-transform:uppercase;letter-spacing:.05em}}
#new-chat-btn{{margin:8px 12px;padding:8px 12px;border-radius:var(--radius);
  background:var(--accent);color:#fff;border:none;cursor:pointer;font-weight:600;
  font-size:13px}}
#new-chat-btn:hover{{background:var(--accent-hover)}}
#history-list{{flex:1;overflow-y:auto;padding:4px 8px}}
.history-item{{padding:8px 12px;border-radius:var(--radius);cursor:pointer;
  font-size:13px;color:var(--text-muted);white-space:nowrap;overflow:hidden;
  text-overflow:ellipsis;margin-bottom:2px}}
.history-item:hover,.history-item.active{{background:var(--bg-input);color:var(--text)}}

/* ── Chat area ────────────────────────────────────────────────────── */
#chat-area{{flex:1;display:flex;flex-direction:column;overflow:hidden}}
#messages{{flex:1;overflow-y:auto;padding:16px;display:flex;flex-direction:column;gap:12px}}
.msg{{max-width:85%;padding:10px 14px;border-radius:var(--radius);
  line-height:1.6;word-wrap:break-word}}
.msg.user{{align-self:flex-end;background:var(--accent);color:#fff}}
.msg.assistant{{align-self:flex-start;background:var(--bg-card);border:1px solid var(--border)}}
.msg.system{{align-self:center;color:var(--text-muted);font-size:12px;font-style:italic}}
.msg h1,.msg h2,.msg h3,.msg h4{{margin:8px 0 4px;font-weight:700}}
.msg h1{{font-size:1.4em}}.msg h2{{font-size:1.2em}}.msg h3{{font-size:1.05em}}
.msg ul,.msg ol{{margin:4px 0 4px 20px}}
.msg a{{color:var(--accent);text-decoration:underline}}
.msg strong{{font-weight:700}}.msg em{{font-style:italic}}
.msg code{{font-family:var(--font-mono);background:var(--bg-input);padding:1px 5px;
  border-radius:3px;font-size:0.92em;color:var(--text-code)}}
.msg pre{{margin:8px 0;padding:12px;background:#0d1117;border-radius:var(--radius);
  overflow-x:auto}}
.msg pre code{{background:none;padding:0;color:#e6edf3;display:block;
  white-space:pre;font-size:13px;line-height:1.5}}
.msg pre code .kw{{color:#ff7b72}}.msg pre code .str{{color:#a5d6ff}}
.msg pre code .cm{{color:#8b949e;font-style:italic}}.msg pre code .num{{color:#79c0ff}}
.msg pre code .fn{{color:#d2a8ff}}.msg pre code .op{{color:#ff7b72}}

.streaming-cursor::after{{content:'▍';animation:blink 1s step-end infinite;color:var(--accent)}}
@keyframes blink{{50%{{opacity:0}}}}

/* ── Input area ───────────────────────────────────────────────────── */
#input-area{{padding:12px 16px;border-top:1px solid var(--border);background:var(--bg-card);
  display:flex;flex-direction:column;gap:8px;flex-shrink:0}}
#input-row{{display:flex;gap:8px;align-items:flex-end}}
#msg-input{{flex:1;resize:none;padding:10px 14px;border-radius:var(--radius);
  border:1px solid var(--border);background:var(--bg-input);color:var(--text);
  font-family:var(--font-sans);font-size:var(--font-size);min-height:42px;
  max-height:180px;outline:none;line-height:1.5}}
#msg-input:focus{{border-color:var(--accent)}}
#send-btn{{padding:10px 20px;border-radius:var(--radius);background:var(--accent);
  color:#fff;border:none;cursor:pointer;font-weight:600;font-size:14px;
  white-space:nowrap;height:42px}}
#send-btn:hover{{background:var(--accent-hover)}}
#send-btn:disabled{{opacity:.5;cursor:not-allowed}}
#toolbar{{display:flex;gap:8px;align-items:center;font-size:12px;color:var(--text-muted)}}
#mode-toggle{{display:flex;gap:4px;align-items:center}}
#mode-toggle label{{cursor:pointer;padding:3px 10px;border-radius:4px;
  border:1px solid var(--border);transition:all .2s}}
#mode-toggle label.active{{background:var(--accent);color:#fff;border-color:var(--accent)}}
#upload-btn{{background:none;border:1px solid var(--border);color:var(--text-muted);
  border-radius:4px;padding:3px 10px;cursor:pointer;font-size:12px}}
#upload-btn:hover{{color:var(--text);border-color:var(--text-muted)}}
#file-input{{display:none}}
.shortcut-hint{{margin-left:auto;color:var(--text-muted);font-size:11px}}

/* ── Responsive ───────────────────────────────────────────────────── */
@media(max-width:768px){{
  #sidebar{{display:none}}
  .msg{{max-width:95%}}
  #topbar .logo{{font-size:15px}}
}}
@media(max-width:480px){{
  #input-area{{padding:8px}}
  #msg-input{{font-size:16px}} /* prevent iOS zoom */
  .shortcut-hint{{display:none}}
}}
</style>
</head>
<body>
<div id="app">
  <!-- Top bar -->
  <header id="topbar">
    <span class="logo">{title}</span>
    <span class="spacer"></span>
    <span id="status-dot" title="Disconnected"></span>
    <button id="settings-btn" title="Settings" aria-label="Settings">&#9881;</button>
  </header>

  <div id="main">
    <!-- Sidebar -->
    <aside id="sidebar">
      <div class="hdr">History</div>
      <button id="new-chat-btn">+ New Chat</button>
      <div id="history-list"></div>
    </aside>

    <!-- Chat area -->
    <section id="chat-area">
      <div id="messages"></div>
      <div id="input-area">
        <div id="input-row">
          <textarea id="msg-input" placeholder="Send a message…" rows="1" aria-label="Message input"></textarea>
          <button id="send-btn" aria-label="Send message">Send</button>
        </div>
        <div id="toolbar">
          <div id="mode-toggle">
            <label class="active" data-mode="chat">Chat</label>
            {agent_label}
          </div>
          {upload_btn}
          <input type="file" id="file-input" aria-label="Upload file">
          <span class="shortcut-hint">Enter send · Shift+Enter newline · Ctrl+/ toggle</span>
        </div>
      </div>
    </section>
  </div>
</div>

<script>
"use strict";
(function(){{
  // ── Config (injected from Rust) ──────────────────────────────────
  var CFG = {{
    maxHistory: {max_history},
    fileUpload: {file_upload_enabled},
    agentMode: {agent_mode_enabled},
    theme: "{theme}"
  }};

  // ── State ────────────────────────────────────────────────────────
  var baseUrl = location.origin;
  var currentMode = "chat";
  var currentSession = null;
  var isStreaming = false;
  var messages = [];
  var eventSource = null;

  // ── DOM refs ─────────────────────────────────────────────────────
  var $msgs     = document.getElementById("messages");
  var $input    = document.getElementById("msg-input");
  var $sendBtn  = document.getElementById("send-btn");
  var $dot      = document.getElementById("status-dot");
  var $history  = document.getElementById("history-list");
  var $newChat  = document.getElementById("new-chat-btn");
  var $settings = document.getElementById("settings-btn");
  var $modeLabels = document.querySelectorAll("#mode-toggle label");
  var $uploadBtn  = document.getElementById("upload-btn");
  var $fileInput  = document.getElementById("file-input");

  // ── Utilities ────────────────────────────────────────────────────
  function escapeHtml(s) {{
    var d = document.createElement("div");
    d.appendChild(document.createTextNode(s));
    return d.innerHTML;
  }}

  // ── Markdown renderer ────────────────────────────────────────────
  function renderMarkdown(text) {{
    if (!text) return "";
    // Code blocks first (``` ... ```)
    text = text.replace(/```(\w*)\n([\s\S]*?)```/g, function(_, lang, code) {{
      return '<pre><code class="lang-' + escapeHtml(lang) + '">' + renderCodeBlock(code, lang) + '</code></pre>';
    }});
    // Inline code
    text = text.replace(/`([^`\n]+)`/g, '<code>$1</code>');
    // Headers
    text = text.replace(/^#### (.+)$/gm, '<h4>$1</h4>');
    text = text.replace(/^### (.+)$/gm, '<h3>$1</h3>');
    text = text.replace(/^## (.+)$/gm, '<h2>$1</h2>');
    text = text.replace(/^# (.+)$/gm, '<h1>$1</h1>');
    // Bold / italic
    text = text.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
    text = text.replace(/\*(.+?)\*/g, '<em>$1</em>');
    text = text.replace(/_(.+?)_/g, '<em>$1</em>');
    // Links
    text = text.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>');
    // Unordered lists
    text = text.replace(/^[\-\*] (.+)$/gm, '<li>$1</li>');
    text = text.replace(/(<li>.*<\/li>\n?)+/g, '<ul>$&</ul>');
    // Ordered lists
    text = text.replace(/^\d+\. (.+)$/gm, '<li>$1</li>');
    // Line breaks (double newline → paragraph)
    text = text.replace(/\n{{2,}}/g, '<br><br>');
    text = text.replace(/\n/g, '<br>');
    return text;
  }}

  // ── Syntax highlighting (basic keyword coloring) ─────────────────
  function renderCodeBlock(code, lang) {{
    var escaped = escapeHtml(code);
    // Keywords
    var kw = /\b(function|const|let|var|if|else|for|while|return|import|export|from|class|def|fn|pub|use|struct|impl|trait|match|async|await|try|catch|throw|new|this|self|true|false|null|None|undefined|yield|break|continue|switch|case|default|do|in|of|as|type|interface|enum|extends|implements|super|package|static|final|abstract|void|int|string|bool|float|double)\b/g;
    escaped = escaped.replace(kw, '<span class="kw">$1</span>');
    // Strings
    escaped = escaped.replace(/(["'])(?:(?!\1|\\).|\\.)*?\1/g, '<span class="str">$&</span>');
    // Comments (// and #)
    escaped = escaped.replace(/(\/\/.*|#.*)/g, '<span class="cm">$1</span>');
    // Numbers
    escaped = escaped.replace(/\b(\d+\.?\d*)\b/g, '<span class="num">$1</span>');
    return escaped;
  }}

  // ── Render a chat message ────────────────────────────────────────
  function renderMessage(role, content, streaming) {{
    var div = document.createElement("div");
    div.className = "msg " + role;
    if (streaming) div.classList.add("streaming-cursor");
    div.innerHTML = renderMarkdown(content);
    $msgs.appendChild(div);
    $msgs.scrollTop = $msgs.scrollHeight;
    return div;
  }}

  function updateLastAssistant(content, streaming) {{
    var nodes = $msgs.querySelectorAll(".msg.assistant");
    var last = nodes[nodes.length - 1];
    if (last) {{
      last.innerHTML = renderMarkdown(content);
      if (streaming) last.classList.add("streaming-cursor");
      else last.classList.remove("streaming-cursor");
      $msgs.scrollTop = $msgs.scrollHeight;
    }}
  }}

  // ── Connection status ────────────────────────────────────────────
  function connectToServer(url) {{
    baseUrl = url || location.origin;
    checkStatus();
    setInterval(checkStatus, 10000);
  }}

  function checkStatus() {{
    fetch(baseUrl + "/health", {{ method: "GET", signal: AbortSignal.timeout(5000) }})
      .then(function(r) {{
        $dot.className = r.ok ? "connected" : "";
        $dot.title = r.ok ? "Connected" : "Disconnected";
      }})
      .catch(function() {{
        $dot.className = "";
        $dot.title = "Disconnected";
      }});
  }}

  // ── Send message ─────────────────────────────────────────────────
  function sendMessage(text, mode) {{
    if (!text.trim() || isStreaming) return;
    text = text.trim();
    messages.push({{ role: "user", content: text }});
    trimHistory();
    renderMessage("user", text);
    $input.value = "";
    autoResize();
    isStreaming = true;
    $sendBtn.disabled = true;

    if (mode === "agent" && CFG.agentMode) {{
      sendAgentMessage(text);
    }} else {{
      sendChatMessage(text);
    }}
  }}

  function sendChatMessage(text) {{
    var accum = "";
    var msgDiv = renderMessage("assistant", "", true);
    fetch(baseUrl + "/chat/stream", {{
      method: "POST",
      headers: {{ "Content-Type": "application/json" }},
      body: JSON.stringify({{ prompt: text }})
    }}).then(function(res) {{
      if (!res.ok) throw new Error("HTTP " + res.status);
      var reader = res.body.getReader();
      var decoder = new TextDecoder();
      function read() {{
        reader.read().then(function(result) {{
          if (result.done) {{
            finishStream(accum);
            return;
          }}
          var chunk = decoder.decode(result.value, {{ stream: true }});
          var lines = chunk.split("\n");
          for (var i = 0; i < lines.length; i++) {{
            var line = lines[i];
            if (line.startsWith("data: ")) {{
              var payload = line.slice(6);
              if (payload === "[DONE]") {{
                finishStream(accum);
                return;
              }}
              try {{
                var ev = JSON.parse(payload);
                if (ev.token) accum += ev.token;
                else if (ev.text) accum += ev.text;
                else if (ev.delta) accum += ev.delta;
                else if (typeof ev === "string") accum += ev;
              }} catch(e) {{
                accum += payload;
              }}
              updateLastAssistant(accum, true);
            }}
          }}
          read();
        }});
      }}
      read();
    }}).catch(function(err) {{
      finishStream("");
      renderMessage("system", "Error: " + err.message);
    }});
  }}

  function sendAgentMessage(text) {{
    fetch(baseUrl + "/agent", {{
      method: "POST",
      headers: {{ "Content-Type": "application/json" }},
      body: JSON.stringify({{ task: text }})
    }}).then(function(res) {{
      if (!res.ok) throw new Error("HTTP " + res.status);
      return res.json();
    }}).then(function(data) {{
      currentSession = data.session_id;
      streamResponse(data.session_id);
    }}).catch(function(err) {{
      finishStream("");
      renderMessage("system", "Agent error: " + err.message);
    }});
  }}

  function streamResponse(sessionId) {{
    var accum = "";
    var msgDiv = renderMessage("assistant", "", true);
    if (eventSource) eventSource.close();
    eventSource = new EventSource(baseUrl + "/stream/" + sessionId);
    eventSource.addEventListener("token", function(e) {{
      accum += e.data;
      updateLastAssistant(accum, true);
    }});
    eventSource.addEventListener("tool_call", function(e) {{
      try {{
        var d = JSON.parse(e.data);
        accum += "\n\n**Tool:** " + (d.tool || d.name || "unknown") + "\n";
        updateLastAssistant(accum, true);
      }} catch(ex) {{}}
    }});
    eventSource.addEventListener("done", function(e) {{
      eventSource.close();
      eventSource = null;
      finishStream(accum);
    }});
    eventSource.addEventListener("error_event", function(e) {{
      eventSource.close();
      eventSource = null;
      finishStream(accum);
      renderMessage("system", "Stream error: " + (e.data || "unknown"));
    }});
    eventSource.onerror = function() {{
      eventSource.close();
      eventSource = null;
      finishStream(accum);
    }};
  }}

  function finishStream(content) {{
    isStreaming = false;
    $sendBtn.disabled = false;
    if (content) {{
      messages.push({{ role: "assistant", content: content }});
      trimHistory();
      updateLastAssistant(content, false);
    }}
    $input.focus();
  }}

  function trimHistory() {{
    while (messages.length > CFG.maxHistory) messages.shift();
  }}

  // ── File upload ──────────────────────────────────────────────────
  function uploadFile(file) {{
    var reader = new FileReader();
    reader.onload = function(e) {{
      var content = e.target.result;
      var ctx = "**File: " + escapeHtml(file.name) + "**\n```\n" + content + "\n```";
      sendMessage(ctx, currentMode);
    }};
    reader.readAsText(file);
  }}

  // ── History sidebar ──────────────────────────────────────────────
  function loadHistory() {{
    fetch(baseUrl + "/sessions.json")
      .then(function(r) {{ return r.json(); }})
      .then(function(sessions) {{
        $history.innerHTML = "";
        sessions.forEach(function(s) {{
          var div = document.createElement("div");
          div.className = "history-item";
          div.textContent = s.task || s.id || "Session";
          div.title = s.id;
          div.onclick = function() {{ loadSession(s.id); }};
          $history.appendChild(div);
        }});
      }})
      .catch(function() {{ /* silent */ }});
  }}

  function loadSession(id) {{
    fetch(baseUrl + "/sessions/" + id + ".json")
      .then(function(r) {{ return r.json(); }})
      .then(function(data) {{
        $msgs.innerHTML = "";
        messages = [];
        currentSession = id;
        (data.messages || []).forEach(function(m) {{
          messages.push(m);
          renderMessage(m.role, m.content);
        }});
        // Mark active
        var items = $history.querySelectorAll(".history-item");
        items.forEach(function(el) {{ el.classList.remove("active"); }});
        items.forEach(function(el) {{ if (el.title === id) el.classList.add("active"); }});
      }})
      .catch(function() {{ renderMessage("system", "Could not load session."); }});
  }}

  // ── Auto-resize textarea ─────────────────────────────────────────
  function autoResize() {{
    $input.style.height = "auto";
    $input.style.height = Math.min($input.scrollHeight, 180) + "px";
  }}

  // ── Preferences (localStorage) ───────────────────────────────────
  function savePrefs() {{
    try {{ localStorage.setItem("vibecody-prefs", JSON.stringify({{ mode: currentMode, theme: CFG.theme }})); }} catch(e) {{}}
  }}
  function loadPrefs() {{
    try {{
      var p = JSON.parse(localStorage.getItem("vibecody-prefs"));
      if (p) {{
        if (p.mode && CFG.agentMode) currentMode = p.mode;
        if (p.theme) {{
          CFG.theme = p.theme;
          document.documentElement.setAttribute("data-theme", p.theme);
        }}
      }}
    }} catch(e) {{}}
  }}

  // ── Mode toggle ──────────────────────────────────────────────────
  function setMode(mode) {{
    currentMode = mode;
    $modeLabels.forEach(function(l) {{
      l.classList.toggle("active", l.getAttribute("data-mode") === mode);
    }});
    savePrefs();
  }}

  // ── Event listeners ──────────────────────────────────────────────
  $sendBtn.addEventListener("click", function() {{
    sendMessage($input.value, currentMode);
  }});

  $input.addEventListener("keydown", function(e) {{
    if (e.key === "Enter" && !e.shiftKey) {{
      e.preventDefault();
      sendMessage($input.value, currentMode);
    }}
  }});

  $input.addEventListener("input", autoResize);

  document.addEventListener("keydown", function(e) {{
    if (e.ctrlKey && e.key === "/") {{
      e.preventDefault();
      var next = currentMode === "chat" ? "agent" : "chat";
      if (next === "agent" && !CFG.agentMode) return;
      setMode(next);
    }}
  }});

  $modeLabels.forEach(function(l) {{
    l.addEventListener("click", function() {{
      setMode(l.getAttribute("data-mode"));
    }});
  }});

  $newChat.addEventListener("click", function() {{
    $msgs.innerHTML = "";
    messages = [];
    currentSession = null;
    $input.focus();
    var items = $history.querySelectorAll(".history-item");
    items.forEach(function(el) {{ el.classList.remove("active"); }});
  }});

  $settings.addEventListener("click", function() {{
    var newTheme = CFG.theme === "dark" ? "light" : "dark";
    CFG.theme = newTheme;
    document.documentElement.setAttribute("data-theme", newTheme);
    savePrefs();
  }});

  if ($uploadBtn) {{
    $uploadBtn.addEventListener("click", function() {{ $fileInput.click(); }});
  }}
  if ($fileInput) {{
    $fileInput.addEventListener("change", function() {{
      if ($fileInput.files.length > 0) uploadFile($fileInput.files[0]);
      $fileInput.value = "";
    }});
  }}

  // ── Init ─────────────────────────────────────────────────────────
  loadPrefs();
  setMode(currentMode);
  connectToServer(baseUrl);
  loadHistory();
  $input.focus();
}})();
</script>
</body>
</html>"##,
        title = title,
        theme = theme,
        max_history = max_history,
        file_upload_enabled = if file_upload_enabled { "true" } else { "false" },
        agent_mode_enabled = if agent_mode_enabled { "true" } else { "false" },
        agent_label = if agent_mode_enabled {
            r#"<label data-mode="agent">Agent</label>"#
        } else {
            ""
        },
        upload_btn = if file_upload_enabled {
            r#"<button id="upload-btn" aria-label="Upload file">&#128206; Attach</button>"#
        } else {
            ""
        },
    )
}

/// Minimal HTML escaping for attribute/content injection.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Config tests ───────────────────────────────────────────────

    #[test]
    fn test_config_defaults() {
        let cfg = WebClientConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.title, "VibeCody Web");
        assert_eq!(cfg.theme, "dark");
        assert_eq!(cfg.max_message_history, 100);
        assert!(cfg.enable_file_upload);
        assert!(cfg.enable_agent_mode);
    }

    #[test]
    fn test_config_serialize_roundtrip() {
        let cfg = WebClientConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        let cfg2: WebClientConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg.title, cfg2.title);
        assert_eq!(cfg.theme, cfg2.theme);
        assert_eq!(cfg.max_message_history, cfg2.max_message_history);
        assert_eq!(cfg.enabled, cfg2.enabled);
        assert_eq!(cfg.enable_file_upload, cfg2.enable_file_upload);
        assert_eq!(cfg.enable_agent_mode, cfg2.enable_agent_mode);
    }

    #[test]
    fn test_config_deserialize_custom() {
        let json = r#"{
            "enabled": false,
            "title": "My Tool",
            "theme": "light",
            "max_message_history": 50,
            "enable_file_upload": false,
            "enable_agent_mode": false
        }"#;
        let cfg: WebClientConfig = serde_json::from_str(json).unwrap();
        assert!(!cfg.enabled);
        assert_eq!(cfg.title, "My Tool");
        assert_eq!(cfg.theme, "light");
        assert_eq!(cfg.max_message_history, 50);
        assert!(!cfg.enable_file_upload);
        assert!(!cfg.enable_agent_mode);
    }

    // ── HTML generation tests ──────────────────────────────────────

    #[test]
    fn test_html_contains_title() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("<title>VibeCody Web</title>"));
    }

    #[test]
    fn test_html_custom_title() {
        let mut cfg = WebClientConfig::default();
        cfg.title = "My Custom Title".to_string();
        let html = web_client_html(&cfg);
        assert!(html.contains("<title>My Custom Title</title>"));
        assert!(html.contains("My Custom Title"));
    }

    #[test]
    fn test_html_contains_chat_input() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("id=\"msg-input\""));
        assert!(html.contains("id=\"send-btn\""));
    }

    #[test]
    fn test_html_contains_sse_connection_code() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("EventSource"));
        assert!(html.contains("/stream/"));
        assert!(html.contains("/chat/stream"));
    }

    #[test]
    fn test_html_contains_markdown_renderer() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("renderMarkdown"));
        assert!(html.contains("renderCodeBlock"));
    }

    #[test]
    fn test_html_contains_theme_colors() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("#1a1a2e")); // background
        assert!(html.contains("#16213e")); // card
        assert!(html.contains("#7c5dfa")); // accent
    }

    #[test]
    fn test_html_has_dark_theme_attribute() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains(r#"data-theme="dark""#));
    }

    #[test]
    fn test_html_light_theme() {
        let mut cfg = WebClientConfig::default();
        cfg.theme = "light".to_string();
        let html = web_client_html(&cfg);
        assert!(html.contains(r#"data-theme="light""#));
    }

    #[test]
    fn test_html_contains_viewport_meta() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("viewport"));
        assert!(html.contains("width=device-width"));
    }

    #[test]
    fn test_html_contains_csp_meta() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("Content-Security-Policy"));
        assert!(html.contains("default-src 'self'"));
    }

    #[test]
    fn test_html_no_external_cdn() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        // No external script or link tags referencing CDNs
        assert!(!html.contains("https://cdn"));
        assert!(!html.contains("https://unpkg"));
        assert!(!html.contains("https://cdnjs"));
        assert!(!html.contains("https://jsdelivr"));
        // No external stylesheet links (check for https:// URLs, not http-equiv attributes)
        assert!(!html.contains("https://"), "HTML should not reference external URLs");
    }

    #[test]
    fn test_html_is_self_contained() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("<style>"));
        assert!(html.contains("<script>"));
        assert!(html.contains("</style>"));
        assert!(html.contains("</script>"));
    }

    #[test]
    fn test_html_contains_sidebar() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("id=\"sidebar\""));
        assert!(html.contains("id=\"history-list\""));
        assert!(html.contains("id=\"new-chat-btn\""));
    }

    #[test]
    fn test_html_contains_mode_toggle() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("id=\"mode-toggle\""));
        assert!(html.contains(r#"data-mode="chat""#));
        assert!(html.contains(r#"data-mode="agent""#));
    }

    #[test]
    fn test_html_agent_mode_disabled() {
        let mut cfg = WebClientConfig::default();
        cfg.enable_agent_mode = false;
        let html = web_client_html(&cfg);
        // Should still have chat label but NOT agent label
        assert!(html.contains(r#"data-mode="chat""#));
        assert!(!html.contains(r#"data-mode="agent""#));
    }

    #[test]
    fn test_html_file_upload_enabled() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("id=\"upload-btn\""));
        assert!(html.contains("id=\"file-input\""));
    }

    #[test]
    fn test_html_file_upload_disabled() {
        let mut cfg = WebClientConfig::default();
        cfg.enable_file_upload = false;
        let html = web_client_html(&cfg);
        assert!(!html.contains("id=\"upload-btn\""));
    }

    #[test]
    fn test_html_contains_status_indicator() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("id=\"status-dot\""));
        assert!(html.contains("/health"));
    }

    #[test]
    fn test_html_contains_keyboard_shortcuts() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        // Ctrl+/ shortcut handler
        assert!(html.contains(r#"e.key === "/""#));
        // Enter to send
        assert!(html.contains(r#"e.key === "Enter""#));
        // Shift+Enter for newline
        assert!(html.contains("e.shiftKey"));
    }

    #[test]
    fn test_html_contains_auto_scroll() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("scrollTop"));
        assert!(html.contains("scrollHeight"));
    }

    #[test]
    fn test_html_contains_local_storage() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("localStorage"));
        assert!(html.contains("vibecody-prefs"));
    }

    #[test]
    fn test_html_responsive_media_queries() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("@media"));
        assert!(html.contains("768px"));
        assert!(html.contains("480px"));
    }

    #[test]
    fn test_html_max_history_injected() {
        let mut cfg = WebClientConfig::default();
        cfg.max_message_history = 42;
        let html = web_client_html(&cfg);
        assert!(html.contains("maxHistory: 42"));
    }

    // ── Favicon tests ──────────────────────────────────────────────

    #[test]
    fn test_favicon_svg_valid() {
        let svg = web_client_favicon_svg();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("#7c5dfa")); // accent color
        assert!(svg.contains(">V<")); // the V letter
    }

    #[test]
    fn test_favicon_svg_is_self_contained() {
        let svg = web_client_favicon_svg();
        assert!(svg.contains("xmlns"));
        assert!(!svg.contains("http://") || svg.contains("xmlns"));
    }

    // ── Assets tests ───────────────────────────────────────────────

    #[test]
    fn test_assets_includes_favicon() {
        let assets = web_client_assets();
        let favicon = assets.iter().find(|a| a.0 == "/favicon.svg");
        assert!(favicon.is_some());
        let (_, ct, content) = favicon.unwrap();
        assert_eq!(*ct, "image/svg+xml");
        assert!(content.contains("<svg"));
    }

    #[test]
    fn test_assets_includes_manifest() {
        let assets = web_client_assets();
        let manifest = assets.iter().find(|a| a.0 == "/manifest.json");
        assert!(manifest.is_some());
        let (_, ct, content) = manifest.unwrap();
        assert_eq!(*ct, "application/json");
        assert!(content.contains("VibeCody"));
    }

    #[test]
    fn test_assets_count() {
        let assets = web_client_assets();
        assert_eq!(assets.len(), 2); // favicon + manifest
    }

    // ── HTML escaping tests ────────────────────────────────────────

    #[test]
    fn test_html_escape_special_chars() {
        let escaped = html_escape("<script>alert('xss')</script>");
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
        assert!(!escaped.contains('\''));
        assert!(escaped.contains("&lt;"));
        assert!(escaped.contains("&gt;"));
    }

    #[test]
    fn test_config_with_html_in_title() {
        let mut cfg = WebClientConfig::default();
        cfg.title = "<b>Evil</b>".to_string();
        let html = web_client_html(&cfg);
        assert!(!html.contains("<b>Evil</b>"));
        assert!(html.contains("&lt;b&gt;Evil&lt;/b&gt;"));
    }

    #[test]
    fn test_html_is_valid_document() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<html"));
        assert!(html.contains("</html>"));
        assert!(html.contains("<head>"));
        assert!(html.contains("</head>"));
        assert!(html.contains("<body>"));
        assert!(html.contains("</body>"));
    }

    #[test]
    fn test_html_contains_connection_functions() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("connectToServer"));
        assert!(html.contains("sendMessage"));
        assert!(html.contains("streamResponse"));
        assert!(html.contains("renderMessage"));
        assert!(html.contains("uploadFile"));
        assert!(html.contains("loadHistory"));
        assert!(html.contains("loadSession"));
    }

    #[test]
    fn test_html_sessions_json_endpoint() {
        let cfg = WebClientConfig::default();
        let html = web_client_html(&cfg);
        assert!(html.contains("/sessions.json"));
    }
}
