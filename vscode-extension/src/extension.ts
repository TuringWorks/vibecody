/**
 * VibeCLI VS Code Extension — entry point.
 *
 * Architecture:
 * - Communicates with a local VibeCLI daemon (`vibecli serve --port 7878`)
 * - Registers sidebar chat webview, inline completions, and agent commands
 * - Status bar shows provider + daemon status
 */

import * as vscode from 'vscode';
import { VibeCLIClient, type AgentEvent, type JobRecord } from './api-client';

let client: VibeCLIClient;
let statusBarItem: vscode.StatusBarItem;
let daemonConnected = false;

// ── Activation ────────────────────────────────────────────────────────────────

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const config = vscode.workspace.getConfiguration('vibecli');
  const port = config.get<number>('daemonPort', 7878);

  client = new VibeCLIClient({ port });

  // Status bar
  statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  statusBarItem.command = 'vibecli.connectDaemon';
  context.subscriptions.push(statusBarItem);
  updateStatusBar('checking…');
  statusBarItem.show();

  // Try connecting to the daemon
  await tryConnect();

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand('vibecli.connectDaemon', handleConnectDaemon),
    vscode.commands.registerCommand('vibecli.startDaemon', handleStartDaemon),
    vscode.commands.registerCommand('vibecli.startAgent', handleStartAgent),
    vscode.commands.registerCommand('vibecli.chat', handleChat),
    vscode.commands.registerCommand('vibecli.inlineEdit', handleInlineEdit),
    vscode.commands.registerCommand('vibecli.viewJobs', handleViewJobs),
    vscode.commands.registerCommand('vibecli.sendSelection', handleSendSelection),
  );

  // Register inline completion provider
  if (config.get<boolean>('inlineCompletions', true)) {
    const provider = new VibeCLIInlineCompletionProvider();
    context.subscriptions.push(
      vscode.languages.registerInlineCompletionItemProvider(
        { pattern: '**' },
        provider,
      ),
    );
  }

  // Register chat webview
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider('vibecli.chatView', new ChatViewProvider(context)),
  );
}

export function deactivate(): void {
  statusBarItem?.dispose();
}

// ── Helpers ───────────────────────────────────────────────────────────────────

function updateStatusBar(status: string): void {
  const config = vscode.workspace.getConfiguration('vibecli');
  const provider = config.get<string>('provider', 'ollama');
  statusBarItem.text = `$(hubot) VibeCLI [${provider}] ${status}`;
  statusBarItem.tooltip = daemonConnected
    ? 'VibeCLI daemon connected — click to reconnect'
    : 'VibeCLI daemon not connected — click to connect';
}

async function tryConnect(): Promise<void> {
  const alive = await client.isAlive();
  daemonConnected = alive;
  updateStatusBar(alive ? '●' : '○ offline');
}

// ── Command handlers ──────────────────────────────────────────────────────────

async function handleConnectDaemon(): Promise<void> {
  updateStatusBar('connecting…');
  await tryConnect();
  if (!daemonConnected) {
    const choice = await vscode.window.showWarningMessage(
      'VibeCLI daemon is not running.',
      'Start Daemon',
      'Cancel',
    );
    if (choice === 'Start Daemon') {
      await handleStartDaemon();
    }
  } else {
    vscode.window.showInformationMessage('VibeCLI daemon connected.');
  }
}

async function handleStartDaemon(): Promise<void> {
  const config = vscode.workspace.getConfiguration('vibecli');
  const port = config.get<number>('daemonPort', 7878);
  const provider = config.get<string>('provider', 'ollama');

  // Reuse existing daemon terminal if one already exists
  const existing = vscode.window.terminals.find((t) => t.name === 'VibeCLI Daemon');
  if (existing) {
    existing.show();
  } else {
    const terminal = vscode.window.createTerminal('VibeCLI Daemon');
    terminal.sendText(`vibecli serve --port ${port} --provider ${provider}`);
    terminal.show();
  }

  // Wait a moment then re-check
  await new Promise((r) => setTimeout(r, 2000));
  await tryConnect();
}

async function handleStartAgent(): Promise<void> {
  if (!daemonConnected) {
    vscode.window.showWarningMessage('VibeCLI daemon not connected. Run "VibeCLI: Start Daemon" first.');
    return;
  }

  const task = await vscode.window.showInputBox({
    prompt: 'Describe the agent task',
    placeHolder: 'e.g. Fix the failing test in AuthService',
  });
  if (!task) return;

  const config = vscode.workspace.getConfiguration('vibecli');
  const approval = config.get<string>('approval', 'suggest');

  const outputChannel = vscode.window.createOutputChannel('VibeCLI Agent');
  outputChannel.show();
  outputChannel.appendLine(`[agent] Starting: ${task}`);

  try {
    const { sessionId } = await client.startAgent(task, approval);
    outputChannel.appendLine(`[agent] Session: ${sessionId}`);
    updateStatusBar('running');

    for await (const event of client.streamAgent(sessionId)) {
      formatEventToOutput(outputChannel, event);
      if (event.type === 'complete' || event.type === 'error') break;
    }
    updateStatusBar('●');
  } catch (err) {
    outputChannel.appendLine(`[error] ${err}`);
    updateStatusBar('error');
  }
}

async function handleChat(): Promise<void> {
  if (!daemonConnected) {
    vscode.window.showWarningMessage('VibeCLI daemon not connected.');
    return;
  }

  const question = await vscode.window.showInputBox({
    prompt: 'Ask VibeCLI a question',
    placeHolder: 'e.g. How does this function work?',
  });
  if (!question) return;

  // Include selected text as context
  const editor = vscode.window.activeTextEditor;
  const context = editor?.document.getText(editor.selection) ?? '';
  const userContent = context
    ? `Context:\n\`\`\`\n${context}\n\`\`\`\n\n${question}`
    : question;

  const response = await vscode.window.withProgress(
    { location: vscode.ProgressLocation.Notification, title: 'VibeCLI thinking…', cancellable: false },
    async () => {
      try {
        return await client.chat([{ role: 'user', content: userContent }]);
      } catch (e) {
        return `Error: ${e}`;
      }
    },
  );

  const doc = await vscode.workspace.openTextDocument({ content: response, language: 'markdown' });
  vscode.window.showTextDocument(doc, { preview: true });
}

// ── Inline Edit (Cmd+Shift+K) ─────────────────────────────────────────────────

async function handleInlineEdit(): Promise<void> {
  if (!daemonConnected) {
    vscode.window.showWarningMessage('VibeCLI daemon not connected. Run "VibeCLI: Start Daemon" first.');
    return;
  }

  const editor = vscode.window.activeTextEditor;
  const selection = editor?.document.getText(editor.selection) ?? '';
  const filePath = editor?.document.fileName ?? '';
  const lang = editor?.document.languageId ?? '';

  const instruction = await vscode.window.showInputBox({
    prompt: 'Inline edit instruction',
    placeHolder: 'e.g. Add error handling, Rename to camelCase, Add JSDoc',
  });
  if (!instruction) return;

  const task = selection
    ? `File: ${filePath}\n\`\`\`${lang}\n${selection}\n\`\`\`\n\nInstruction: ${instruction}`
    : instruction;

  const config = vscode.workspace.getConfiguration('vibecli');
  const approval = config.get<string>('approval', 'suggest');

  const outputChannel = vscode.window.createOutputChannel('VibeCLI Inline Edit');
  outputChannel.show();
  outputChannel.appendLine(`[inline-edit] ${instruction}`);

  try {
    const { sessionId } = await client.startAgent(task, approval);
    for await (const event of client.streamAgent(sessionId)) {
      formatEventToOutput(outputChannel, event);
      if (event.type === 'complete' || event.type === 'error') break;
    }
  } catch (err) {
    outputChannel.appendLine(`[error] ${err}`);
  }
}

// ── View Jobs ─────────────────────────────────────────────────────────────────

async function handleViewJobs(): Promise<void> {
  if (!daemonConnected) {
    vscode.window.showWarningMessage('VibeCLI daemon not connected.');
    return;
  }

  const jobs = await client.listJobs();
  if (jobs.length === 0) {
    vscode.window.showInformationMessage('No VibeCLI background jobs found.');
    return;
  }

  const statusIcon = (s: JobRecord['status']) =>
    s === 'complete' ? '✅' : s === 'running' ? '🟡' : s === 'failed' ? '❌' : '⛔';

  const picks = jobs.map((j) => ({
    label: `${statusIcon(j.status)} ${j.task.slice(0, 60)}${j.task.length > 60 ? '…' : ''}`,
    description: `${j.status} · ${j.provider}`,
    detail: j.summary ?? `Session: ${j.session_id}`,
    job: j,
  }));

  const selected = await vscode.window.showQuickPick(picks, {
    title: `VibeCLI Jobs (${jobs.length})`,
    placeHolder: 'Select a job to view or cancel',
    matchOnDetail: true,
  });

  if (!selected) return;

  const job = selected.job;

  if (job.status === 'running') {
    const action = await vscode.window.showQuickPick(
      ['Stream live output', 'Cancel job', 'Dismiss'],
      { title: `Job: ${job.task.slice(0, 50)}` },
    );
    if (action === 'Stream live output') {
      const ch = vscode.window.createOutputChannel(`VibeCLI Job: ${job.session_id.slice(0, 8)}`);
      ch.show();
      ch.appendLine(`[streaming] ${job.task}`);
      for await (const event of client.streamAgent(job.session_id)) {
        formatEventToOutput(ch, event);
        if (event.type === 'complete' || event.type === 'error') break;
      }
    } else if (action === 'Cancel job') {
      await client.cancelJob(job.session_id);
      vscode.window.showInformationMessage(`Cancelled job ${job.session_id.slice(0, 8)}`);
    }
  } else if (job.summary) {
    // Show summary in a markdown document
    const doc = await vscode.workspace.openTextDocument({
      content: `# Job: ${job.task}\n\nStatus: **${job.status}**\n\nSession: \`${job.session_id}\`\n\n---\n\n${job.summary}`,
      language: 'markdown',
    });
    vscode.window.showTextDocument(doc, { preview: true });
  }
}

// ── Send Selection to Agent ───────────────────────────────────────────────────

async function handleSendSelection(): Promise<void> {
  if (!daemonConnected) {
    vscode.window.showWarningMessage('VibeCLI daemon not connected.');
    return;
  }

  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    vscode.window.showWarningMessage('No active editor.');
    return;
  }

  const selection = editor.document.getText(editor.selection);
  const filePath = editor.document.fileName;
  const lang = editor.document.languageId;

  if (!selection.trim()) {
    vscode.window.showWarningMessage('Select some code first, then run this command.');
    return;
  }

  const question = await vscode.window.showInputBox({
    prompt: 'What do you want to do with the selection?',
    placeHolder: 'e.g. Explain this, Add tests, Refactor, Fix the bug',
  });
  if (!question) return;

  const task = `File: ${filePath}\n\`\`\`${lang}\n${selection}\n\`\`\`\n\nTask: ${question}`;
  const config = vscode.workspace.getConfiguration('vibecli');
  const approval = config.get<string>('approval', 'suggest');

  const outputChannel = vscode.window.createOutputChannel('VibeCLI');
  outputChannel.show();
  outputChannel.appendLine(`[task] ${question}`);

  try {
    const { sessionId } = await client.startAgent(task, approval);
    for await (const event of client.streamAgent(sessionId)) {
      formatEventToOutput(outputChannel, event);
      if (event.type === 'complete' || event.type === 'error') break;
    }
  } catch (err) {
    outputChannel.appendLine(`[error] ${err}`);
  }
}

function formatEventToOutput(channel: vscode.OutputChannel, event: AgentEvent): void {
  switch (event.type) {
    case 'chunk':
      channel.append(event.content ?? '');
      break;
    case 'step':
      channel.appendLine(`\n[step ${(event.step_num ?? 0) + 1}] ${event.tool_name} → ${event.success ? '✔' : '✘'}`);
      break;
    case 'complete':
      channel.appendLine(`\n[done] ${event.content}`);
      break;
    case 'error':
      channel.appendLine(`\n[error] ${event.content}`);
      break;
  }
}

// ── Inline Completion Provider ─────────────────────────────────────────────────

class VibeCLIInlineCompletionProvider implements vscode.InlineCompletionItemProvider {
  private lastCompletionTime = 0;
  private debounceMs = 500;

  async provideInlineCompletionItems(
    document: vscode.TextDocument,
    position: vscode.Position,
    _context: vscode.InlineCompletionContext,
    token: vscode.CancellationToken,
  ): Promise<vscode.InlineCompletionList | undefined> {
    if (!daemonConnected) return undefined;

    const now = Date.now();
    if (now - this.lastCompletionTime < this.debounceMs) return undefined;
    this.lastCompletionTime = now;

    const linePrefix = document.lineAt(position).text.slice(0, position.character);
    if (linePrefix.trim().length < 3) return undefined;

    // Build context: up to 50 lines before cursor
    const startLine = Math.max(0, position.line - 50);
    const contextText = document.getText(
      new vscode.Range(startLine, 0, position.line, position.character),
    );

    const prompt = `Complete the following ${document.languageId} code. Output ONLY the completion (no explanation):\n\`\`\`${document.languageId}\n${contextText}`;

    try {
      const completion = await client.chat([{ role: 'user', content: prompt }]);
      if (token.isCancellationRequested) return undefined;

      // Extract just the first line or block that makes sense
      const suggestion = extractFirstCompletion(completion, linePrefix);
      if (!suggestion) return undefined;

      return {
        items: [
          new vscode.InlineCompletionItem(suggestion, new vscode.Range(position, position)),
        ],
      };
    } catch {
      return undefined;
    }
  }
}

function extractFirstCompletion(raw: string, _linePrefix: string): string | undefined {
  // Strip markdown code fences if present
  const stripped = raw.replace(/^```[\w]*\n?/, '').replace(/\n?```$/, '').trim();
  if (!stripped) return undefined;
  // Return up to first blank line (one logical block)
  const firstBlock = stripped.split(/\n\n/)[0];
  return firstBlock || undefined;
}

// ── Chat Webview Provider ─────────────────────────────────────────────────────

class ChatViewProvider implements vscode.WebviewViewProvider {
  constructor(private readonly context: vscode.ExtensionContext) {}

  resolveWebviewView(webviewView: vscode.WebviewView): void {
    webviewView.webview.options = { enableScripts: true };
    webviewView.webview.html = getChatHtml(webviewView.webview);

    // Forward messages from webview → daemon (streaming)
    webviewView.webview.onDidReceiveMessage(async (msg: { type: string; content: string }) => {
      if (msg.type !== 'send') return;
      if (!daemonConnected) {
        webviewView.webview.postMessage({ type: 'error', content: 'Daemon not connected.' });
        return;
      }

      // Auto-inject current file context if an editor is active
      const editor = vscode.window.activeTextEditor;
      const fileCtx = editor
        ? `\n\n[Current file: ${editor.document.fileName} (${editor.document.languageId})]`
        : '';
      const userContent = msg.content + fileCtx;

      try {
        let accumulated = '';
        for await (const chunk of client.chatStream([{ role: 'user', content: userContent }])) {
          accumulated += chunk;
          webviewView.webview.postMessage({ type: 'chunk', content: chunk });
        }
        webviewView.webview.postMessage({ type: 'done', content: accumulated });
      } catch {
        // Fall back to non-streaming
        try {
          const response = await client.chat([{ role: 'user', content: userContent }]);
          webviewView.webview.postMessage({ type: 'response', content: response });
        } catch (e2) {
          webviewView.webview.postMessage({ type: 'error', content: String(e2) });
        }
      }
    });
  }
}

function getChatHtml(_webview: vscode.Webview): string {
  return /* html */ `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>VibeCLI Chat</title>
  <style>
    body { font-family: var(--vscode-font-family); font-size: var(--vscode-font-size); color: var(--vscode-foreground); background: var(--vscode-sideBar-background); margin: 0; display: flex; flex-direction: column; height: 100vh; }
    #messages { flex: 1; overflow-y: auto; padding: 10px; display: flex; flex-direction: column; gap: 8px; }
    .msg { padding: 8px 10px; border-radius: 6px; max-width: 90%; white-space: pre-wrap; word-break: break-word; font-size: 12px; }
    .msg.user { background: var(--vscode-button-background); color: var(--vscode-button-foreground); align-self: flex-end; }
    .msg.assistant { background: var(--vscode-editor-inactiveSelectionBackground); align-self: flex-start; }
    .msg.error { color: var(--vscode-errorForeground); }
    #input-area { display: flex; gap: 6px; padding: 8px; border-top: 1px solid var(--vscode-panel-border); }
    #input { flex: 1; padding: 6px 8px; background: var(--vscode-input-background); color: var(--vscode-input-foreground); border: 1px solid var(--vscode-input-border); border-radius: 4px; outline: none; font-size: 12px; resize: none; }
    #send { padding: 6px 12px; background: var(--vscode-button-background); color: var(--vscode-button-foreground); border: none; border-radius: 4px; cursor: pointer; font-size: 12px; }
  </style>
</head>
<body>
  <div id="messages"></div>
  <div id="input-area">
    <textarea id="input" rows="2" placeholder="Ask VibeCLI…"></textarea>
    <button id="send">Send</button>
  </div>
  <script>
    const vscode = acquireVsCodeApi();
    const messages = document.getElementById('messages');
    const input = document.getElementById('input');
    const sendBtn = document.getElementById('send');

    function appendMsg(role, content) {
      const div = document.createElement('div');
      div.className = 'msg ' + role;
      div.textContent = content;
      messages.appendChild(div);
      messages.scrollTop = messages.scrollHeight;
    }

    function send() {
      const text = input.value.trim();
      if (!text) return;
      appendMsg('user', text);
      input.value = '';
      vscode.postMessage({ type: 'send', content: text });
    }

    sendBtn.addEventListener('click', send);
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); send(); }
    });

    let streamingDiv = null;
    window.addEventListener('message', (e) => {
      const msg = e.data;
      if (msg.type === 'response') {
        appendMsg('assistant', msg.content);
        streamingDiv = null;
      } else if (msg.type === 'chunk') {
        if (!streamingDiv) {
          streamingDiv = document.createElement('div');
          streamingDiv.className = 'msg assistant';
          messages.appendChild(streamingDiv);
        }
        streamingDiv.textContent += msg.content;
        messages.scrollTop = messages.scrollHeight;
      } else if (msg.type === 'done') {
        streamingDiv = null;
      } else if (msg.type === 'error') {
        appendMsg('error', 'Error: ' + msg.content);
        streamingDiv = null;
      }
    });
  </script>
</body>
</html>`;
}
