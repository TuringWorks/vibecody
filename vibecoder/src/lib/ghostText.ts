/**
 * Ghost text — explicit-trigger inline completion for Monaco.
 *
 * # The gate
 *
 * The previous ghost-text surface was removed in `5a7eef7c` because it was
 * keystroke-driven. This one is not, and the single line that guarantees it is
 * the `triggerKind` check in `provideInlineCompletions`: Monaco calls the
 * provider for *both* `Automatic` (typing) and `Explicit` (the user asked), and
 * we answer only the second.
 *
 * `InlineCompletionTriggerKind` is `{ Automatic: 0, Explicit: 1 }` in Monaco and
 * `{ Invoke: 0, Automatic: 1 }` in VS Code — the *same names mean opposite
 * numbers*. Compare against the named enum member, never a literal, and never
 * copy this check between the two hosts. The VS Code side lives in
 * `vscode-extension/src/ghost-text.ts` and has its own gate.
 *
 * There is deliberately no debounce timer and no edit-history buffer here. The
 * request carries the window around the cursor and nothing else.
 */
import type * as Monaco from "monaco-editor";

/** Lines of context sent before the cursor. */
export const PREFIX_LINES = 160;
/** Lines of context sent after the cursor. */
export const SUFFIX_LINES = 60;

export interface GhostResponse {
  completion: string;
  model_name: string;
  truncated: boolean;
}

export interface GhostTextDeps {
  invoke: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
  /** Toolbar provider selection. Empty string means "nothing selected". */
  getProvider: () => string;
  /** Toolbar model selection. */
  getModel: () => string;
  /** Absolute path of the active file, for the prompt header. */
  getFilePath: () => string;
  /** Surfaced to the user; called for a failed request, not for an empty one. */
  onError: (message: string) => void;
  /** Called when the model's answer was clipped by the backend line cap. */
  onTruncated?: () => void;
}

/** The prefix/suffix window around a cursor position. */
export interface ContextWindow {
  prefix: string;
  suffix: string;
  /** Text between the cursor and the end of its line. */
  restOfLine: string;
}

/**
 * Slice the bounded window around the cursor.
 *
 * Exported for tests: the window is the entire hidden state this surface has,
 * so what goes in it is worth pinning.
 */
export function windowContext(
  model: Pick<Monaco.editor.ITextModel, "getLineCount" | "getValueInRange" | "getLineMaxColumn">,
  position: Pick<Monaco.Position, "lineNumber" | "column">,
): ContextWindow {
  const lineCount = model.getLineCount();
  const firstLine = Math.max(1, position.lineNumber - PREFIX_LINES);
  const lastLine = Math.min(lineCount, position.lineNumber + SUFFIX_LINES);
  const endColumn = model.getLineMaxColumn(lastLine);
  const lineEndColumn = model.getLineMaxColumn(position.lineNumber);

  return {
    prefix: model.getValueInRange({
      startLineNumber: firstLine,
      startColumn: 1,
      endLineNumber: position.lineNumber,
      endColumn: position.column,
    }),
    suffix: model.getValueInRange({
      startLineNumber: position.lineNumber,
      startColumn: position.column,
      endLineNumber: lastLine,
      endColumn,
    }),
    restOfLine: model.getValueInRange({
      startLineNumber: position.lineNumber,
      startColumn: position.column,
      endLineNumber: position.lineNumber,
      endColumn: lineEndColumn,
    }),
  };
}

/** How a completion must be shaped to render correctly at the cursor. */
export interface FittedCompletion {
  text: string;
  /**
   * When true the replacement range runs to the end of the line, which is what
   * Monaco requires of a multi-line `insertText`. Only safe when the text being
   * swallowed is whitespace.
   */
  extendToEndOfLine: boolean;
}

/**
 * Constrain a completion so Monaco can render it at the cursor.
 *
 * Monaco requires that a multi-line `insertText` end its range at the end of a
 * line. When real code follows the cursor on the same line we cannot extend the
 * range without eating it, so the suggestion is clipped to its first line.
 * Returns `null` when nothing renderable remains.
 */
export function fitCompletionToLine(
  completion: string,
  restOfLine: string,
): FittedCompletion | null {
  if (completion.length === 0) return null;

  // Trailing whitespace after the cursor can be swallowed; real text cannot.
  if (restOfLine.trim().length === 0) {
    return { text: completion, extendToEndOfLine: restOfLine.length > 0 };
  }

  const firstLine = completion.split("\n")[0];
  if (firstLine.trim().length === 0) return null;
  return { text: firstLine, extendToEndOfLine: false };
}

export interface GhostTextHandle {
  dispose: () => void;
  /**
   * Ask Monaco for an explicit inline suggestion. This is the only path that
   * reaches the provider with `Explicit`, so it is the only path that produces
   * a suggestion.
   */
  trigger: (editor: Monaco.editor.ICodeEditor) => void;
}

/**
 * Register the explicit-trigger inline completion provider for all languages.
 *
 * Registering on `"*"` is safe precisely because of the trigger gate: the
 * provider is consulted constantly but answers only when asked.
 */
export function registerGhostText(
  monaco: typeof Monaco,
  deps: GhostTextDeps,
): GhostTextHandle {
  const registration = monaco.languages.registerInlineCompletionsProvider("*", {
    provideInlineCompletions: async (model, position, context, token) => {
      // ── The gate. See the module header before touching this. ──
      if (
        context.triggerKind !==
        monaco.languages.InlineCompletionTriggerKind.Explicit
      ) {
        return { items: [] };
      }

      const provider = deps.getProvider();
      const selectedModel = deps.getModel();
      if (!provider || !selectedModel) {
        // Matches the provider-agnostic rule: no toolbar selection means no
        // request, never a silent default to one vendor.
        deps.onError("Select a provider and model in the toolbar first.");
        return { items: [] };
      }

      const { prefix, suffix, restOfLine } = windowContext(model, position);

      let response: GhostResponse;
      try {
        response = await deps.invoke<GhostResponse>("ghost_complete", {
          filePath: deps.getFilePath(),
          language: model.getLanguageId(),
          prefix,
          suffix,
          provider,
          model: selectedModel,
        });
      } catch (error) {
        deps.onError(
          `Inline completion failed: ${
            error instanceof Error ? error.message : String(error)
          }`,
        );
        return { items: [] };
      }

      // The user moved on while the request was in flight.
      if (token.isCancellationRequested) return { items: [] };

      // An empty completion is the model declining, not a failure — say
      // nothing rather than reporting an error the user cannot act on.
      const fitted = fitCompletionToLine(response.completion, restOfLine);
      if (!fitted) return { items: [] };

      if (response.truncated) deps.onTruncated?.();

      const endColumn = fitted.extendToEndOfLine
        ? model.getLineMaxColumn(position.lineNumber)
        : position.column;

      return {
        items: [
          {
            insertText: fitted.text,
            range: {
              startLineNumber: position.lineNumber,
              startColumn: position.column,
              endLineNumber: position.lineNumber,
              endColumn,
            },
          },
        ],
      };
    },
    // Nothing is retained per request, so there is nothing to release.
    disposeInlineCompletions: () => {},
  });

  return {
    dispose: () => registration.dispose(),
    trigger: (editor) => {
      editor.trigger("ghost-text", "editor.action.inlineSuggest.trigger", {});
    },
  };
}
