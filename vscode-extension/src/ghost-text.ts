/**
 * Ghost text — explicit-trigger inline completion for VS Code.
 *
 * # The gate
 *
 * VS Code calls an inline completion provider for both automatic (typing) and
 * explicit (user gesture) triggers. This provider answers only the explicit
 * one. That check is the entire reason this surface is not the keystroke-driven
 * one removed in `5a7eef7c`, so it does not get relaxed.
 *
 * **The enum is inverted between our two editors.** VS Code has
 * `InlineCompletionTriggerKind = { Invoke: 0, Automatic: 1 }`; Monaco has
 * `{ Automatic: 0, Explicit: 1 }`. The same names carry opposite numbers, so a
 * literal `=== 1` copied from the VibeCoder provider (`src/lib/ghostText.ts`)
 * would gate on exactly the wrong half and fire on every keystroke. Always
 * compare against the named member.
 *
 * There is no debounce timer and no edit-history buffer here — one request per
 * user gesture, carrying only the window around the cursor.
 */
import * as vscode from 'vscode';
import { VibeCLIClient } from './api-client';

/** Lines of context sent before the cursor. */
export const PREFIX_LINES = 160;
/** Lines of context sent after the cursor. */
export const SUFFIX_LINES = 60;

/** The bounded window around a cursor position. */
export interface ContextWindow {
  prefix: string;
  suffix: string;
  /** Text between the cursor and the end of its line. */
  restOfLine: string;
}

/**
 * The slice of `vscode.TextDocument` this module needs. Narrowed so the pure
 * helpers below can be tested without a live editor.
 */
export interface WindowableDocument {
  lineCount: number;
  getText(range?: vscode.Range): string;
  lineAt(line: number): { range: vscode.Range; text: string };
}

/** Slice the bounded window around the cursor. */
export function windowContext(
  document: WindowableDocument,
  position: vscode.Position,
  makeRange: (sl: number, sc: number, el: number, ec: number) => vscode.Range,
): ContextWindow {
  const firstLine = Math.max(0, position.line - PREFIX_LINES);
  const lastLine = Math.min(document.lineCount - 1, position.line + SUFFIX_LINES);
  const lineEnd = document.lineAt(position.line).range.end;
  const lastLineEnd = document.lineAt(lastLine).range.end;

  return {
    prefix: document.getText(
      makeRange(firstLine, 0, position.line, position.character),
    ),
    suffix: document.getText(
      makeRange(position.line, position.character, lastLine, lastLineEnd.character),
    ),
    restOfLine: document.getText(
      makeRange(position.line, position.character, position.line, lineEnd.character),
    ),
  };
}

/**
 * Constrain a completion so it renders correctly at the cursor.
 *
 * A multi-line suggestion cannot be shown when real code follows the cursor on
 * the same line, so it is clipped to its first line. Returns `null` when
 * nothing renderable remains.
 */
export function fitCompletionToLine(
  completion: string,
  restOfLine: string,
): string | null {
  if (completion.length === 0) return null;
  if (restOfLine.trim().length === 0) return completion;

  const firstLine = completion.split('\n')[0];
  return firstLine.trim().length === 0 ? null : firstLine;
}

/** What the provider needs from the extension host. */
export interface GhostTextDeps {
  client: () => VibeCLIClient | null;
  /** `vibecli.provider` / `vibecli.model` settings, empty when unset. */
  getProvider: () => string;
  getModel: () => string;
  showError: (message: string) => void;
  showInfo: (message: string) => void;
}

/**
 * Build the provider. Exported separately from `registerGhostText` so tests
 * can drive `provideInlineCompletionItems` directly.
 */
export function createGhostTextProvider(
  deps: GhostTextDeps,
): vscode.InlineCompletionItemProvider {
  return {
    async provideInlineCompletionItems(document, position, context, token) {
      // ── The gate. Read this file's header before changing it. ──
      if (context.triggerKind !== vscode.InlineCompletionTriggerKind.Invoke) {
        return undefined;
      }

      const client = deps.client();
      if (!client) {
        deps.showError('Not connected to the VibeCLI daemon.');
        return undefined;
      }

      const { prefix, suffix, restOfLine } = windowContext(
        document,
        position,
        (sl, sc, el, ec) => new vscode.Range(sl, sc, el, ec),
      );

      let response;
      try {
        response = await client.ghostComplete({
          filePath: document.uri.fsPath,
          language: document.languageId,
          prefix,
          suffix,
          provider: deps.getProvider() || undefined,
          model: deps.getModel() || undefined,
        });
      } catch (error) {
        deps.showError(
          error instanceof Error ? error.message : String(error),
        );
        return undefined;
      }

      if (token.isCancellationRequested) return undefined;

      // An empty completion is the model declining, not a failure.
      const text = fitCompletionToLine(response.completion, restOfLine);
      if (text === null) return undefined;

      if (response.truncated) {
        // The cap lives in `vibe_ai::ghost`; don't restate the number, it
        // would go stale silently.
        deps.showInfo('Suggestion was clipped — accept it and re-trigger for more.');
      }

      return [
        new vscode.InlineCompletionItem(
          text,
          new vscode.Range(position, position),
        ),
      ];
    },
  };
}

/**
 * Register the provider for every file and wire the explicit trigger command.
 *
 * Registering on `'*'` is safe precisely because of the trigger gate: the
 * provider is consulted constantly and answers only when asked.
 */
export function registerGhostText(
  context: vscode.ExtensionContext,
  deps: GhostTextDeps,
): void {
  context.subscriptions.push(
    vscode.languages.registerInlineCompletionItemProvider(
      { pattern: '**' },
      createGhostTextProvider(deps),
    ),
    vscode.commands.registerCommand('vibecli.ghostComplete', async () => {
      // The built-in trigger action is what reaches the provider with
      // `Invoke`; calling the provider ourselves would bypass VS Code's
      // ghost-text rendering and its Tab-to-accept binding.
      await vscode.commands.executeCommand(
        'editor.action.inlineSuggest.trigger',
      );
    }),
  );
}
