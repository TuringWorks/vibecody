/**
 * VS Code hook executor — parity with the CLI/Tauri hook protocol
 * (`vibecli-cli/src/hook_abort.rs`) and the JetBrains plugin's
 * `HookExecutor.kt`.
 *
 * Hooks are configured under `vibecli.hooks` in user/workspace
 * settings as an array of `{ name, event, command, enabled }`.
 * Each hook receives the event payload as JSON on stdin, runs its
 * command via `sh -c <command>` (POSIX) or `cmd /c <command>`
 * (Windows), and signals a decision via exit code:
 *
 * | Exit code | Decision                                            |
 * |:---------:|----------------------------------------------------|
 * |   `0`     | Allow — continue.                                  |
 * |   `2`     | Block — halt; surface stderr to the user.          |
 * | other     | Generic error — non-blocking; logged as a warning. |
 *
 * A structured JSON object on stdout
 * `{ "action": "allow"|"block"|"modify", "reason": "...", "message": "..." }`
 * overrides the exit code when present.
 *
 * Multi-hook chains run in declaration order; the first non-ALLOW
 * decision short-circuits the rest.
 */

import * as vscode from 'vscode';
import { spawn } from 'child_process';

export type HookAction = 'allow' | 'block' | 'modify';

export interface HookDecision {
  action: HookAction;
  reason?: string;
  message?: string;
  exit_code: number;
}

export interface HookConfig {
  name: string;
  event: string;
  command: string;
  enabled: boolean;
}

/** Allow-list of event kinds matching `plugin_manifest::ALLOWED_HOOK_EVENTS`
 *  on the CLI side and `HookExecutor.ALLOWED_EVENTS` on JetBrains. */
export const ALLOWED_EVENTS = [
  'PreToolUse',
  'PostToolUse',
  'UserPromptSubmit',
  'Stop',
  'SubagentStop',
  'Notification',
  'PreCompact',
] as const;

export type AllowedEvent = (typeof ALLOWED_EVENTS)[number];

const HOOK_TIMEOUT_MS = 30_000;

/**
 * Read the configured hook chain from the workspace's `vibecli.hooks`
 * setting. Filters out disabled rows and rows targeting other events.
 */
function configuredHooks(event: string): HookConfig[] {
  const config = vscode.workspace.getConfiguration('vibecli');
  const raw = config.get<HookConfig[]>('hooks', []) ?? [];
  return raw.filter((h) => h && h.enabled !== false && h.event === event);
}

/**
 * Fire `event` through every configured hook of that kind, in order.
 * Returns the first non-ALLOW decision, or `{ action: 'allow' }` when
 * every hook permitted the action (including the empty-chain case).
 */
export async function fireHook(event: string, payload: unknown): Promise<HookDecision> {
  const hooks = configuredHooks(event);
  if (hooks.length === 0) return { action: 'allow', exit_code: 0 };

  const payloadJson = JSON.stringify(payload);
  for (const hook of hooks) {
    const decision = await runOne(hook, payloadJson);
    if (decision.action !== 'allow') return decision;
  }
  return { action: 'allow', exit_code: 0 };
}

async function runOne(hook: HookConfig, payloadJson: string): Promise<HookDecision> {
  const command = (hook.command ?? '').trim();
  if (!command) return { action: 'allow', exit_code: 0 };

  const isWindows = process.platform === 'win32';
  const [bin, ...args] = isWindows ? ['cmd', '/c', command] : ['sh', '-c', command];

  return new Promise<HookDecision>((resolve) => {
    let resolved = false;
    const proc = spawn(bin, args, { stdio: ['pipe', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';

    proc.stdout.on('data', (d: Buffer) => (stdout += d.toString()));
    proc.stderr.on('data', (d: Buffer) => (stderr += d.toString()));

    const timer = setTimeout(() => {
      if (resolved) return;
      resolved = true;
      proc.kill('SIGKILL');
      resolve({
        action: 'block',
        reason: `hook timeout after ${HOOK_TIMEOUT_MS / 1000}s`,
        exit_code: -1,
      });
    }, HOOK_TIMEOUT_MS);

    proc.on('error', (err) => {
      if (resolved) return;
      resolved = true;
      clearTimeout(timer);
      // Spawn failure is non-blocking (matches CLI/JetBrains).
      console.warn(`[vibecli] hook "${hook.name}" failed to spawn:`, err.message);
      resolve({
        action: 'allow',
        message: `hook ${hook.name} failed to spawn: ${err.message}`,
        exit_code: -1,
      });
    });

    proc.on('close', (code) => {
      if (resolved) return;
      resolved = true;
      clearTimeout(timer);
      const exitCode = code ?? -1;

      // Try the structured-decision path first.
      const structured = parseStructured(stdout, exitCode, stderr);
      if (structured) {
        resolve(structured);
        return;
      }

      // Fall back to exit-code semantics.
      let action: HookAction;
      if (exitCode === 0) action = 'allow';
      else if (exitCode === 2) action = 'block';
      else action = 'allow'; // generic error — non-blocking, matches CLI

      resolve({
        action,
        reason: action === 'block' ? (stderr.trim() || `blocked by hook ${hook.name}`) : undefined,
        message: stderr.trim() || undefined,
        exit_code: exitCode,
      });
    });

    // Pipe the event payload to the hook on stdin and close.
    proc.stdin.write(payloadJson);
    proc.stdin.end();
  });
}

/** Try to interpret stdout as a JSON decision object. Returns
 *  undefined when stdout is empty, isn't JSON, or doesn't have an
 *  `action` field — caller falls back to exit-code semantics. */
function parseStructured(
  stdout: string,
  exitCode: number,
  stderr: string,
): HookDecision | undefined {
  const trimmed = stdout.trim();
  if (!trimmed.startsWith('{') || !trimmed.endsWith('}')) return undefined;
  try {
    const obj = JSON.parse(trimmed) as Record<string, unknown>;
    const actionStr = typeof obj.action === 'string' ? obj.action.toLowerCase() : '';
    if (actionStr !== 'allow' && actionStr !== 'block' && actionStr !== 'modify') {
      return undefined;
    }
    return {
      action: actionStr as HookAction,
      reason: typeof obj.reason === 'string' ? obj.reason : undefined,
      message:
        typeof obj.message === 'string' ? obj.message : stderr.trim() || undefined,
      exit_code: exitCode,
    };
  } catch {
    return undefined;
  }
}

/**
 * Convenience: fire UserPromptSubmit and return `true` when the
 * action should proceed. On BLOCK, surfaces a warning toast and
 * returns `false`. Use this at command entry points where the user
 * has just typed a prompt and we want a single-line gate.
 */
export async function gatePromptSubmission(
  prompt: string,
  source: string,
): Promise<boolean> {
  const decision = await fireHook('UserPromptSubmit', {
    event: 'UserPromptSubmit',
    source,
    prompt,
  });
  if (decision.action === 'block') {
    const reason = decision.reason?.trim() || 'policy';
    await vscode.window.showWarningMessage(
      `VibeCLI: ${source} blocked by hook — ${reason}`,
    );
    return false;
  }
  return true;
}
