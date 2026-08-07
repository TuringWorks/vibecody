/**
 * LSP ↔ Monaco bridge — IntelliSense for the editor.
 *
 * The Rust side (`vibe-lsp`) owns the language-server connections; this module
 * owns everything Monaco needs to consume them. Four things here are load
 * bearing, and each one on its own is enough to make IntelliSense look broken:
 *
 * 1. **URIs must match.** The document URI in a completion request has to be
 *    the one we sent with `didOpen`. Monaco models created without a `path`
 *    get `inmemory://model/1`, which no server has ever heard of — every
 *    request then returns nothing, for every language. {@link fileUri} is the
 *    single encoder, mirroring `vibe_lsp::path_to_uri` byte for byte.
 * 2. **Edits must be pushed.** LSP is stateful: the server answers against the
 *    text it was last told about. Without `didChange` the server keeps
 *    answering against the file as it was when you opened it, so nothing you
 *    just typed can be completed. Requests {@link LspBridge.flush} pending
 *    changes first, so a completion can never race its own edit.
 * 3. **Kinds must be translated.** LSP and Monaco both call the enum
 *    `CompletionItemKind` and neither agrees on a single value: LSP `Text` is 1,
 *    Monaco's `Text` is 18 and `Method` is 0. Passing LSP values through
 *    unmapped mislabels every suggestion. Mapping is by *name*, since Monaco
 *    renumbers (this version's `Snippet` is 28, not 27).
 * 4. **Trigger characters must be registered.** Monaco only re-triggers
 *    completion on characters given at registration time. With none, `foo.`
 *    shows nothing while mid-identifier completion appears to work — the
 *    classic "IntelliSense is half-broken" report.
 */

import type * as Monaco from "monaco-editor";

// ── Monaco surface we depend on ─────────────────────────────────────────────
// Narrow structural types instead of the full `monaco` namespace: everything
// here stays unit-testable against `standaloneEnums.js`, which imports no DOM.

/** Monaco's `CompletionItemKind`, by the names we map onto. */
export interface MonacoCompletionKinds {
  readonly Method: number;
  readonly Function: number;
  readonly Constructor: number;
  readonly Field: number;
  readonly Variable: number;
  readonly Class: number;
  readonly Struct: number;
  readonly Interface: number;
  readonly Module: number;
  readonly Property: number;
  readonly Event: number;
  readonly Operator: number;
  readonly Unit: number;
  readonly Value: number;
  readonly Constant: number;
  readonly Enum: number;
  readonly EnumMember: number;
  readonly Keyword: number;
  readonly Text: number;
  readonly Color: number;
  readonly File: number;
  readonly Reference: number;
  readonly Folder: number;
  readonly TypeParameter: number;
  readonly Snippet: number;
}

export interface MonacoLspEnums {
  readonly completionKinds: MonacoCompletionKinds;
  readonly insertAsSnippet: number;
  readonly deprecatedTag: number;
  readonly markerSeverity: {
    readonly Error: number;
    readonly Warning: number;
    readonly Info: number;
    readonly Hint: number;
  };
}

/** Read the enums we need off a live Monaco instance. */
export function enumsFromMonaco(monaco: typeof Monaco): MonacoLspEnums {
  return {
    completionKinds: monaco.languages.CompletionItemKind,
    insertAsSnippet:
      monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
    deprecatedTag: monaco.languages.CompletionItemTag.Deprecated,
    markerSeverity: monaco.MarkerSeverity,
  };
}

// ── LSP wire types ──────────────────────────────────────────────────────────

export interface LspPosition {
  line: number;
  character: number;
}
export interface LspRange {
  start: LspPosition;
  end: LspPosition;
}
export interface LspTextEdit {
  range: LspRange;
  newText: string;
}
export interface LspInsertReplaceEdit {
  newText: string;
  insert: LspRange;
  replace: LspRange;
}
export interface LspMarkupContent {
  kind: "markdown" | "plaintext";
  value: string;
}
export type LspDocumentation = string | LspMarkupContent;

export interface LspCompletionItem {
  label: string;
  labelDetails?: { detail?: string; description?: string };
  kind?: number;
  tags?: number[];
  detail?: string;
  documentation?: LspDocumentation;
  deprecated?: boolean;
  preselect?: boolean;
  sortText?: string;
  filterText?: string;
  insertText?: string;
  /** 1 = plain text, 2 = snippet syntax. */
  insertTextFormat?: number;
  textEdit?: LspTextEdit | LspInsertReplaceEdit;
  additionalTextEdits?: LspTextEdit[];
  commitCharacters?: string[];
  /** Opaque server payload; must be returned verbatim when resolving. */
  data?: unknown;
}

export interface LspCompletionList {
  isIncomplete?: boolean;
  items: LspCompletionItem[];
  itemDefaults?: {
    editRange?: LspRange | { insert: LspRange; replace: LspRange };
    insertTextFormat?: number;
    commitCharacters?: string[];
  };
}

export type LspCompletionResponse = LspCompletionItem[] | LspCompletionList;

export interface LspDiagnostic {
  range: LspRange;
  /** 1 = error, 2 = warning, 3 = information, 4 = hint. */
  severity?: number;
  code?: string | number;
  source?: string;
  message: string;
  tags?: number[];
}

export type LspHoverContents =
  | string
  | LspMarkupContent
  | Array<string | { language?: string; value: string }>;

export interface LspHover {
  contents: LspHoverContents;
  range?: LspRange;
}

export interface LspLocation {
  uri: string;
  range: LspRange;
}
export interface LspLocationLink {
  targetUri: string;
  targetRange: LspRange;
  targetSelectionRange?: LspRange;
}
export type LspDefinitionResponse =
  | LspLocation
  | LspLocation[]
  | LspLocationLink[]
  | null;

export interface LspParameterInformation {
  label: string | [number, number];
  documentation?: LspDocumentation;
}
export interface LspSignatureInformation {
  label: string;
  documentation?: LspDocumentation;
  parameters?: LspParameterInformation[];
  activeParameter?: number;
}
export interface LspSignatureHelp {
  signatures: LspSignatureInformation[];
  activeSignature?: number;
  activeParameter?: number;
}

export type LspLanguageState =
  | "running"
  | "available"
  | "not_installed"
  | "unconfigured"
  | "failed";

export interface LspLanguageSupport {
  language: string;
  state: LspLanguageState;
  detail: string;
  supported: boolean;
  completionTriggerCharacters: string[];
  signatureHelpTriggerCharacters: string[];
}

// ── Paths and URIs ──────────────────────────────────────────────────────────

/**
 * `file://` URI for a filesystem path.
 *
 * Must agree exactly with `vibe_lsp::path_to_uri`: this string is the key the
 * server files the document under, and a mismatch means every request for it
 * silently returns nothing.
 */
export function fileUri(path: string): string {
  const unreserved = /[A-Za-z0-9\-_.~:/]/;
  const encoder = new TextEncoder();
  const encoded = Array.from(path)
    .map((char) => {
      if (unreserved.test(char)) return char;
      return Array.from(encoder.encode(char))
        .map((byte) => `%${byte.toString(16).toUpperCase().padStart(2, "0")}`)
        .join("");
    })
    .join("");
  return `file://${encoded}`;
}

/** Directory part of a path, used as a fallback workspace root. */
export function parentDirectory(path: string): string {
  const index = path.lastIndexOf("/");
  return index > 0 ? path.slice(0, index) : "/";
}

/**
 * Which language server handles this file.
 *
 * Deliberately separate from `detectLanguage` (Monaco's *highlighting* id).
 * Those two disagree on purpose: Monaco has no Zig grammar, so `.zig`
 * highlights as `cpp` — but routing a `.zig` file to `clangd` because of that
 * would produce confidently wrong C++ completions in a Zig file.
 */
export function lspLanguageForPath(path: string): string | null {
  const name = path.split("/").pop() ?? path;
  const exact = EXACT_FILENAME_LANGUAGE[name.toLowerCase()];
  if (exact) return exact;
  const dotted = name.split(".");
  if (dotted.length < 2) return null;
  return EXTENSION_LANGUAGE[dotted[dotted.length - 1].toLowerCase()] ?? null;
}

/** Filenames with no useful extension. */
const EXACT_FILENAME_LANGUAGE: Record<string, string> = {
  dockerfile: "dockerfile",
  "cargo.toml": "toml",
  "cargo.lock": "toml",
};

/**
 * Extension → LSP language id. Ids match `LspManager::server_configs`; an
 * extension absent here gets no LSP providers at all, which is the correct
 * outcome for `.txt` or `.log`.
 */
const EXTENSION_LANGUAGE: Record<string, string> = {
  // Web
  ts: "typescript",
  tsx: "typescript",
  mts: "typescript",
  cts: "typescript",
  js: "javascript",
  jsx: "javascript",
  mjs: "javascript",
  cjs: "javascript",
  html: "html",
  htm: "html",
  css: "css",
  scss: "css",
  less: "css",
  json: "json",
  jsonc: "json",
  // Systems
  rs: "rust",
  c: "c",
  h: "c",
  cpp: "cpp",
  cc: "cpp",
  cxx: "cpp",
  hpp: "cpp",
  hh: "cpp",
  hxx: "cpp",
  go: "go",
  zig: "zig",
  nim: "nim",
  nims: "nim",
  d: "d",
  v: "v",
  vala: "vala",
  // JVM
  java: "java",
  kt: "kotlin",
  kts: "kotlin",
  scala: "scala",
  sc: "scala",
  groovy: "groovy",
  clj: "clojure",
  cljs: "clojure",
  cljc: "clojure",
  edn: "clojure",
  // .NET
  cs: "csharp",
  fs: "fsharp",
  fsx: "fsharp",
  vb: "vb",
  // Scripting
  py: "python",
  pyw: "python",
  pyi: "python",
  rb: "ruby",
  erb: "ruby",
  rake: "ruby",
  gemspec: "ruby",
  php: "php",
  pl: "perl",
  pm: "perl",
  lua: "lua",
  r: "r",
  // Functional
  hs: "haskell",
  lhs: "haskell",
  ex: "elixir",
  exs: "elixir",
  erl: "erlang",
  hrl: "erlang",
  ml: "ocaml",
  mli: "ocaml",
  rkt: "racket",
  lisp: "lisp",
  cl: "lisp",
  // Apple / mobile
  swift: "swift",
  dart: "dart",
  // Other compiled
  cr: "crystal",
  f: "fortran",
  f90: "fortran",
  f95: "fortran",
  f03: "fortran",
  f08: "fortran",
  pas: "pascal",
  pp: "pascal",
  jl: "julia",
  pro: "prolog",
  // Markup / config
  yaml: "yaml",
  yml: "yaml",
  toml: "toml",
  md: "markdown",
  markdown: "markdown",
  sql: "sql",
  graphql: "graphql",
  gql: "graphql",
};

/**
 * Monaco languages that already have a full in-browser language service (its
 * TypeScript, CSS, HTML and JSON web workers).
 *
 * We deliberately do **not** attach LSP providers to these. Monaco's built-in
 * providers cannot be unregistered, so adding ours would show every suggestion
 * twice — worse than either source alone. Those workers also became far more
 * capable in the same change that fixed IntelliSense: each file now has its own
 * model at a real `file://` URI, so Monaco's TypeScript service can finally
 * resolve imports *across* open files instead of seeing one shared
 * `inmemory://model/1`.
 */
const MONACO_SERVICED_LANGUAGES = new Set([
  "typescript",
  "javascript",
  "json",
  "jsonc",
  "css",
  "scss",
  "less",
  "html",
  "handlebars",
  "razor",
]);

/** Does Monaco already provide IntelliSense for this (Monaco) language id? */
export function hasBuiltinLanguageService(monacoLanguage: string): boolean {
  return MONACO_SERVICED_LANGUAGES.has(monacoLanguage);
}

/**
 * Characters that re-trigger completion when a server advertises none.
 *
 * Monaco takes single characters only, so a multi-character server trigger
 * (`::`, `->`) contributes its characters individually.
 */
const FALLBACK_TRIGGER_CHARACTERS = [".", ":", ">", "@", "/", '"'];

export function triggerCharacters(advertised: readonly string[]): string[] {
  const characters = advertised.flatMap((trigger) => Array.from(trigger));
  const chosen = characters.length > 0 ? characters : FALLBACK_TRIGGER_CHARACTERS;
  return Array.from(new Set(chosen));
}

// ── Converters ──────────────────────────────────────────────────────────────

/**
 * LSP `CompletionItemKind` (1-25) → Monaco's, by name.
 *
 * Unknown kinds become `Text` — visually neutral, and never a wrong icon.
 */
export function toMonacoCompletionKind(
  lspKind: number | undefined,
  kinds: MonacoCompletionKinds,
): number {
  const byLspValue: Record<number, number> = {
    1: kinds.Text,
    2: kinds.Method,
    3: kinds.Function,
    4: kinds.Constructor,
    5: kinds.Field,
    6: kinds.Variable,
    7: kinds.Class,
    8: kinds.Interface,
    9: kinds.Module,
    10: kinds.Property,
    11: kinds.Unit,
    12: kinds.Value,
    13: kinds.Enum,
    14: kinds.Keyword,
    15: kinds.Snippet,
    16: kinds.Color,
    17: kinds.File,
    18: kinds.Reference,
    19: kinds.Folder,
    20: kinds.EnumMember,
    21: kinds.Constant,
    22: kinds.Struct,
    23: kinds.Event,
    24: kinds.Operator,
    25: kinds.TypeParameter,
  };
  if (lspKind === undefined) return kinds.Text;
  return byLspValue[lspKind] ?? kinds.Text;
}

/** LSP severity (1-4) → Monaco `MarkerSeverity`. Absent means error, per spec. */
export function toMonacoSeverity(
  lspSeverity: number | undefined,
  severities: MonacoLspEnums["markerSeverity"],
): number {
  switch (lspSeverity) {
    case 2:
      return severities.Warning;
    case 3:
      return severities.Info;
    case 4:
      return severities.Hint;
    default:
      return severities.Error;
  }
}

/** LSP range (0-based) → Monaco range (1-based). */
export function toMonacoRange(range: LspRange): Monaco.IRange {
  return {
    startLineNumber: range.start.line + 1,
    startColumn: range.start.character + 1,
    endLineNumber: range.end.line + 1,
    endColumn: range.end.character + 1,
  };
}

/** Monaco position (1-based) → LSP position (0-based). */
export function toLspPosition(position: {
  lineNumber: number;
  column: number;
}): LspPosition {
  return { line: position.lineNumber - 1, character: position.column - 1 };
}

function toMarkdown(
  documentation: LspDocumentation | undefined,
): string | Monaco.IMarkdownString | undefined {
  if (documentation === undefined) return undefined;
  if (typeof documentation === "string") {
    return documentation.length > 0 ? documentation : undefined;
  }
  if (documentation.value.length === 0) return undefined;
  return documentation.kind === "markdown"
    ? { value: documentation.value, isTrusted: false }
    : documentation.value;
}

function isInsertReplaceEdit(
  edit: LspTextEdit | LspInsertReplaceEdit,
): edit is LspInsertReplaceEdit {
  return "insert" in edit && "replace" in edit;
}

/** The range a completion should replace, given the item and its context. */
function completionRange(
  item: LspCompletionItem,
  itemDefaults: LspCompletionList["itemDefaults"],
  fallback: Monaco.IRange,
): Monaco.IRange | { insert: Monaco.IRange; replace: Monaco.IRange } {
  if (item.textEdit) {
    return isInsertReplaceEdit(item.textEdit)
      ? {
          insert: toMonacoRange(item.textEdit.insert),
          replace: toMonacoRange(item.textEdit.replace),
        }
      : toMonacoRange(item.textEdit.range);
  }
  const defaultRange = itemDefaults?.editRange;
  if (defaultRange) {
    return "insert" in defaultRange
      ? {
          insert: toMonacoRange(defaultRange.insert),
          replace: toMonacoRange(defaultRange.replace),
        }
      : toMonacoRange(defaultRange);
  }
  // Monaco *requires* a range. Without one it drops the suggestion list.
  return fallback;
}

/** A Monaco completion item that remembers the LSP item it came from. */
export type BridgedCompletionItem = Monaco.languages.CompletionItem & {
  /** Kept so `resolveCompletionItem` can hand the server its own item back. */
  __lsp: LspCompletionItem;
  __language: string;
  __rootPath: string;
};

export function toMonacoCompletionItem(
  item: LspCompletionItem,
  context: {
    itemDefaults: LspCompletionList["itemDefaults"];
    fallbackRange: Monaco.IRange;
    enums: MonacoLspEnums;
    language: string;
    rootPath: string;
  },
): BridgedCompletionItem {
  const { enums } = context;
  const insertTextFormat =
    item.insertTextFormat ?? context.itemDefaults?.insertTextFormat;
  const newText =
    item.textEdit?.newText ?? item.insertText ?? item.label;

  const deprecated =
    item.deprecated === true || item.tags?.includes(1) === true;

  // `labelDetails` is where 3.17 servers put the signature; fall back to it so
  // the suggestion list still shows types when `detail` is absent.
  const labelDetail = [
    item.labelDetails?.detail,
    item.labelDetails?.description,
  ]
    .filter((part): part is string => Boolean(part))
    .join(" ");
  const detail = item.detail ?? (labelDetail.length > 0 ? labelDetail : undefined);

  return {
    label: item.label,
    kind: toMonacoCompletionKind(item.kind, enums.completionKinds),
    insertText: newText,
    // Snippet syntax (`call(${1:arg})`) must be declared or Monaco inserts the
    // placeholder markup literally.
    ...(insertTextFormat === 2
      ? { insertTextRules: enums.insertAsSnippet }
      : {}),
    range: completionRange(item, context.itemDefaults, context.fallbackRange),
    detail,
    documentation: toMarkdown(item.documentation),
    sortText: item.sortText,
    filterText: item.filterText,
    preselect: item.preselect,
    commitCharacters:
      item.commitCharacters ?? context.itemDefaults?.commitCharacters,
    ...(deprecated ? { tags: [enums.deprecatedTag] } : {}),
    additionalTextEdits: item.additionalTextEdits?.map((edit) => ({
      range: toMonacoRange(edit.range),
      text: edit.newText,
    })),
    __lsp: item,
    __language: context.language,
    __rootPath: context.rootPath,
  };
}

export function toMonacoCompletionList(
  response: LspCompletionResponse | null,
  context: {
    fallbackRange: Monaco.IRange;
    enums: MonacoLspEnums;
    language: string;
    rootPath: string;
  },
): Monaco.languages.CompletionList {
  if (!response) return { suggestions: [] };
  const isList = !Array.isArray(response);
  const items = isList ? response.items : response;
  const itemDefaults = isList ? response.itemDefaults : undefined;
  return {
    incomplete: isList ? response.isIncomplete === true : false,
    suggestions: (items ?? []).map((item) =>
      toMonacoCompletionItem(item, { ...context, itemDefaults }),
    ),
  };
}

export function toMonacoHover(
  hover: LspHover | null,
): Monaco.languages.Hover | null {
  if (!hover || hover.contents === undefined || hover.contents === null) {
    return null;
  }
  const { contents } = hover;
  const parts: Monaco.IMarkdownString[] = [];

  if (typeof contents === "string") {
    parts.push({ value: contents });
  } else if (Array.isArray(contents)) {
    for (const part of contents) {
      if (typeof part === "string") {
        parts.push({ value: part });
      } else if (part.language) {
        // MarkedString: a code block in the given language.
        parts.push({
          value: `\`\`\`${part.language}\n${part.value}\n\`\`\``,
        });
      } else {
        parts.push({ value: part.value });
      }
    }
  } else {
    parts.push({ value: contents.value });
  }

  const nonEmpty = parts.filter((part) => part.value.trim().length > 0);
  if (nonEmpty.length === 0) return null;
  return {
    contents: nonEmpty,
    ...(hover.range ? { range: toMonacoRange(hover.range) } : {}),
  };
}

function isLocationLink(
  location: LspLocation | LspLocationLink,
): location is LspLocationLink {
  return "targetUri" in location;
}

export function toMonacoLocations(
  response: LspDefinitionResponse,
  parseUri: (uri: string) => Monaco.Uri,
): Monaco.languages.Location[] {
  if (!response) return [];
  const locations = Array.isArray(response) ? response : [response];
  return locations.map((location) =>
    isLocationLink(location)
      ? {
          uri: parseUri(location.targetUri),
          range: toMonacoRange(
            location.targetSelectionRange ?? location.targetRange,
          ),
        }
      : { uri: parseUri(location.uri), range: toMonacoRange(location.range) },
  );
}

export function toMonacoMarkers(
  diagnostics: readonly LspDiagnostic[],
  enums: MonacoLspEnums,
): Monaco.editor.IMarkerData[] {
  return diagnostics.map((diagnostic) => {
    const range = toMonacoRange(diagnostic.range);
    return {
      ...range,
      // A zero-width marker draws nothing; widen it by one so the squiggle is
      // visible where a server reports a point (missing semicolon, EOF).
      endColumn:
        range.startLineNumber === range.endLineNumber &&
        range.endColumn === range.startColumn
          ? range.endColumn + 1
          : range.endColumn,
      message: diagnostic.message,
      severity: toMonacoSeverity(diagnostic.severity, enums.markerSeverity),
      source: diagnostic.source,
      code:
        diagnostic.code === undefined ? undefined : String(diagnostic.code),
      // LSP tag 2 = deprecated, 1 = unnecessary → both render as faded text.
      tags: diagnostic.tags?.includes(1) ? [1] : undefined,
    };
  });
}

export function toMonacoSignatureHelp(
  help: LspSignatureHelp | null,
): Monaco.languages.SignatureHelp | null {
  if (!help || help.signatures.length === 0) return null;
  return {
    activeSignature: help.activeSignature ?? 0,
    activeParameter: help.activeParameter ?? 0,
    signatures: help.signatures.map((signature) => ({
      label: signature.label,
      documentation: toMarkdown(signature.documentation),
      parameters: (signature.parameters ?? []).map((parameter) => ({
        label: parameter.label,
        documentation: toMarkdown(parameter.documentation),
      })),
      activeParameter: signature.activeParameter,
    })),
  };
}

// ── The bridge ──────────────────────────────────────────────────────────────

export type InvokeFn = <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

export interface LspBridgeOptions {
  invoke: InvokeFn;
  /**
   * Current workspace root. A getter, not a value: providers are registered
   * once at editor mount, and a captured root would still be `""` for anyone
   * who opens a folder afterwards — which is every user who restores a session.
   */
  getWorkspaceRoot: () => string;
  /** Milliseconds of typing quiet before a `didChange` is pushed. */
  changeDebounceMs?: number;
  /** Called when a language has no usable server, once per language. */
  onLanguageUnavailable?: (support: LspLanguageSupport) => void;
}

interface TrackedDocument {
  path: string;
  uri: string;
  /** LSP language id — picks the server. */
  language: string;
  /** Monaco language id — picks which providers see this model. */
  monacoLanguage: string;
  rootPath: string;
  pendingText: string | null;
  changeTimer: number | null;
  diagnosticTimers: number[];
}

/** Marker owner. Namespaced so we never clear Monaco's own TS/JSON markers. */
export const MARKER_OWNER = "vibecoder-lsp";

/** Delays after a sync at which we re-read diagnostics, in ms.
 *
 * Deliberately a short bounded burst rather than a standing interval: a poll
 * that keeps running while the user reads code costs CPU forever and learns
 * nothing — diagnostics only change after an edit. */
const DIAGNOSTIC_POLL_DELAYS = [250, 900, 2200, 5000];

export interface LspBridge {
  /** Open (or resync) a document and make sure its providers are registered. */
  openDocument(path: string, monacoLanguage: string, text: string): Promise<void>;
  /** Queue the document's new text; pushed after the debounce or on `flush`. */
  changeDocument(path: string, text: string): void;
  /** Push any queued text for a document immediately. */
  flush(path: string): Promise<void>;
  saveDocument(path: string): Promise<void>;
  closeDocument(path: string): Promise<void>;
  /** Which language a tracked path is served by, if any. */
  languageFor(path: string): string | undefined;
  dispose(): void;
}

export function createLspBridge(
  monaco: typeof Monaco,
  options: LspBridgeOptions,
): LspBridge {
  const {
    invoke,
    getWorkspaceRoot,
    changeDebounceMs = 250,
    onLanguageUnavailable,
  } = options;
  const enums = enumsFromMonaco(monaco);

  /** Tracked documents, by absolute path. */
  const documents = new Map<string, TrackedDocument>();
  /** Model URI → path, so a provider can tell which file it was invoked for. */
  const pathByModelUri = new Map<string, string>();
  /** Monaco language id → the providers registered for it. */
  const registrations = new Map<
    string,
    { triggers: Set<string>; disposables: Monaco.IDisposable[] }
  >();
  /** Languages with no server. Cached so we ask the backend once, not per key. */
  const unsupported = new Set<string>();
  /** In-flight support lookups, so N tabs opening at once make one call. */
  const supportProbes = new Map<string, Promise<LspLanguageSupport | null>>();
  let disposed = false;

  const rootFor = (path: string): string =>
    // A lone file opened with no folder still deserves IntelliSense; its own
    // directory is a perfectly good single-file workspace root.
    getWorkspaceRoot() || parentDirectory(path);

  const documentFor = (model: Monaco.editor.ITextModel):
    | TrackedDocument
    | undefined => {
    const path = pathByModelUri.get(model.uri.toString());
    return path === undefined ? undefined : documents.get(path);
  };

  const clearTimers = (document: TrackedDocument) => {
    if (document.changeTimer !== null) {
      window.clearTimeout(document.changeTimer);
      document.changeTimer = null;
    }
    document.diagnosticTimers.forEach(window.clearTimeout);
    document.diagnosticTimers = [];
  };

  // ── Diagnostics ───────────────────────────────────────────────────────────

  const refreshDiagnostics = async (document: TrackedDocument) => {
    const model = monaco.editor.getModel(monaco.Uri.parse(document.uri));
    if (!model || model.isDisposed()) return;
    try {
      const diagnostics = await invoke<LspDiagnostic[] | null>(
        "lsp_diagnostics",
        {
          language: document.language,
          rootPath: document.rootPath,
          uri: document.uri,
        },
      );
      // `null` = the server has published nothing for this file yet. Clearing
      // markers on that would erase real errors every time we poll early.
      if (diagnostics === null || diagnostics === undefined) return;
      if (model.isDisposed()) return;
      monaco.editor.setModelMarkers(
        model,
        MARKER_OWNER,
        toMonacoMarkers(diagnostics, enums),
      );
    } catch {
      // A diagnostics poll is never worth surfacing: the server may simply not
      // be up yet, and the next sync schedules another look.
    }
  };

  const scheduleDiagnostics = (document: TrackedDocument) => {
    document.diagnosticTimers.forEach(window.clearTimeout);
    document.diagnosticTimers = DIAGNOSTIC_POLL_DELAYS.map((delay) =>
      window.setTimeout(() => void refreshDiagnostics(document), delay),
    );
  };

  // ── Provider registration ─────────────────────────────────────────────────

  const probeSupport = (
    language: string,
    rootPath: string,
  ): Promise<LspLanguageSupport | null> => {
    const existing = supportProbes.get(language);
    if (existing) return existing;
    const probe = invoke<LspLanguageSupport>("lsp_language_support", {
      language,
      rootPath,
    })
      .catch(() => null)
      .finally(() => supportProbes.delete(language));
    supportProbes.set(language, probe);
    return probe;
  };

  const registerProviders = (monacoLanguage: string, triggers: Set<string>) => {
    const existing = registrations.get(monacoLanguage);
    if (existing) {
      const grown = Array.from(triggers).some(
        (trigger) => !existing.triggers.has(trigger),
      );
      if (!grown) return;
      // Monaco has no way to add a trigger character to a live provider, so a
      // wider set means re-registering. Happens at most a few times per session.
      existing.disposables.forEach((disposable) => disposable.dispose());
      triggers = new Set([...existing.triggers, ...triggers]);
      registrations.delete(monacoLanguage);
    }

    const triggerList = Array.from(triggers);
    const disposables: Monaco.IDisposable[] = [
      monaco.languages.registerCompletionItemProvider(monacoLanguage, {
        triggerCharacters: triggerList,
        provideCompletionItems: async (model, position) => {
          const document = documentFor(model);
          if (!document) return { suggestions: [] };
          // The edit that triggered this completion may still be queued; the
          // server must see it before it answers, or the symbol just typed is
          // invisible to it.
          await flush(document.path);
          const word = model.getWordUntilPosition(position);
          const fallbackRange: Monaco.IRange = {
            startLineNumber: position.lineNumber,
            endLineNumber: position.lineNumber,
            startColumn: word.startColumn,
            endColumn: word.endColumn,
          };
          try {
            const response = await invoke<LspCompletionResponse | null>(
              "lsp_completion",
              {
                language: document.language,
                rootPath: document.rootPath,
                params: {
                  textDocument: { uri: document.uri },
                  position: toLspPosition(position),
                  context: { triggerKind: 1 },
                },
              },
            );
            return toMonacoCompletionList(response, {
              fallbackRange,
              enums,
              language: document.language,
              rootPath: document.rootPath,
            });
          } catch (error) {
            console.warn(
              `LSP completion failed for ${document.language}:`,
              error,
            );
            return { suggestions: [] };
          }
        },
        resolveCompletionItem: async (item) => {
          const bridged = item as BridgedCompletionItem;
          if (!bridged.__lsp) return item;
          // Already has everything the server would add.
          if (bridged.documentation !== undefined) return item;
          try {
            const resolved = await invoke<LspCompletionItem>(
              "lsp_resolve_completion",
              {
                language: bridged.__language,
                rootPath: bridged.__rootPath,
                item: bridged.__lsp,
              },
            );
            return {
              ...item,
              detail: resolved.detail ?? item.detail,
              documentation: toMarkdown(resolved.documentation),
              additionalTextEdits:
                resolved.additionalTextEdits?.map((edit) => ({
                  range: toMonacoRange(edit.range),
                  text: edit.newText,
                })) ?? item.additionalTextEdits,
            };
          } catch {
            // Resolve is an enrichment; the unresolved item is still usable.
            return item;
          }
        },
      }),

      monaco.languages.registerHoverProvider(monacoLanguage, {
        provideHover: async (model, position) => {
          const document = documentFor(model);
          if (!document) return null;
          await flush(document.path);
          try {
            const hover = await invoke<LspHover | null>("lsp_hover", {
              language: document.language,
              rootPath: document.rootPath,
              params: {
                textDocument: { uri: document.uri },
                position: toLspPosition(position),
              },
            });
            return toMonacoHover(hover);
          } catch (error) {
            console.warn(`LSP hover failed for ${document.language}:`, error);
            return null;
          }
        },
      }),

      monaco.languages.registerDefinitionProvider(monacoLanguage, {
        provideDefinition: async (model, position) => {
          const document = documentFor(model);
          if (!document) return null;
          await flush(document.path);
          try {
            const response = await invoke<LspDefinitionResponse>(
              "lsp_goto_definition",
              {
                language: document.language,
                rootPath: document.rootPath,
                params: {
                  textDocument: { uri: document.uri },
                  position: toLspPosition(position),
                },
              },
            );
            const locations = toMonacoLocations(response, (uri) =>
              monaco.Uri.parse(uri),
            );
            return locations.length > 0 ? locations : null;
          } catch (error) {
            console.warn(
              `LSP go-to-definition failed for ${document.language}:`,
              error,
            );
            return null;
          }
        },
      }),

      monaco.languages.registerSignatureHelpProvider(monacoLanguage, {
        signatureHelpTriggerCharacters: ["(", ","],
        signatureHelpRetriggerCharacters: [")"],
        provideSignatureHelp: async (model, position) => {
          const document = documentFor(model);
          if (!document) return null;
          await flush(document.path);
          try {
            const help = await invoke<LspSignatureHelp | null>(
              "lsp_signature_help",
              {
                language: document.language,
                rootPath: document.rootPath,
                params: {
                  textDocument: { uri: document.uri },
                  position: toLspPosition(position),
                },
              },
            );
            const converted = toMonacoSignatureHelp(help);
            return converted
              ? { value: converted, dispose: () => {} }
              : null;
          } catch {
            return null;
          }
        },
      }),
    ];

    registrations.set(monacoLanguage, { triggers, disposables });
  };

  /**
   * Make sure `monacoLanguage` has providers wired to `language`'s server.
   *
   * Registration is per *Monaco* language because that is what decides which
   * models a provider sees; the server is chosen per *document* at request
   * time. The two differ whenever Monaco lacks a grammar (`.zig` highlights as
   * `cpp`), and conflating them would send C++ files to a Zig server.
   */
  const ensureProviders = async (
    language: string,
    monacoLanguage: string,
    rootPath: string,
  ) => {
    if (unsupported.has(language)) return;
    const support = await probeSupport(language, rootPath);
    if (disposed) return;
    if (!support || !support.supported) {
      unsupported.add(language);
      if (support) onLanguageUnavailable?.(support);
      return;
    }
    if (support.state === "failed" || support.state === "not_installed") {
      // Still register: the user may install the server and the next request
      // (after `lsp_restart_language`) should just work. But tell them why
      // IntelliSense is quiet right now.
      onLanguageUnavailable?.(support);
    }
    registerProviders(
      monacoLanguage,
      new Set(triggerCharacters(support.completionTriggerCharacters)),
    );
  };

  // ── Document lifecycle ────────────────────────────────────────────────────

  const flush = async (path: string): Promise<void> => {
    const document = documents.get(path);
    if (!document) return;
    if (document.changeTimer !== null) {
      window.clearTimeout(document.changeTimer);
      document.changeTimer = null;
    }
    const text = document.pendingText;
    if (text === null) return;
    document.pendingText = null;
    try {
      await invoke("lsp_did_change", {
        language: document.language,
        rootPath: document.rootPath,
        uri: document.uri,
        text,
      });
      scheduleDiagnostics(document);
    } catch (error) {
      console.warn(`LSP didChange failed for ${document.path}:`, error);
    }
  };

  return {
    async openDocument(path, monacoLanguage, text) {
      const language = lspLanguageForPath(path);
      if (!language) return;
      // Starting a server whose suggestions we would refuse to show is pure
      // cost — several hundred MB for tsserver on a large repo.
      if (hasBuiltinLanguageService(monacoLanguage)) return;
      const rootPath = rootFor(path);
      const uri = fileUri(path);

      const existing = documents.get(path);
      const document: TrackedDocument = existing ?? {
        path,
        uri,
        language,
        monacoLanguage,
        rootPath,
        pendingText: null,
        changeTimer: null,
        diagnosticTimers: [],
      };
      document.monacoLanguage = monacoLanguage;
      document.rootPath = rootPath;
      documents.set(path, document);
      pathByModelUri.set(monaco.Uri.parse(uri).toString(), path);

      try {
        await invoke("lsp_did_open", {
          language,
          rootPath,
          uri,
          text,
        });
      } catch (error) {
        console.warn(`No IntelliSense for ${path}:`, error);
        // Fall through: providers may still be worth registering so that a
        // later retry (server installed, workspace opened) works without a
        // reload.
      }
      if (disposed) return;
      await ensureProviders(language, monacoLanguage, rootPath);
      if (disposed) return;
      scheduleDiagnostics(document);
    },

    changeDocument(path, text) {
      const document = documents.get(path);
      if (!document) return;
      document.pendingText = text;
      if (document.changeTimer !== null) {
        window.clearTimeout(document.changeTimer);
      }
      document.changeTimer = window.setTimeout(() => {
        document.changeTimer = null;
        void flush(path);
      }, changeDebounceMs);
    },

    flush,

    async saveDocument(path) {
      const document = documents.get(path);
      if (!document) return;
      await flush(path);
      try {
        await invoke("lsp_did_save", {
          language: document.language,
          rootPath: document.rootPath,
          uri: document.uri,
        });
        scheduleDiagnostics(document);
      } catch (error) {
        console.warn(`LSP didSave failed for ${path}:`, error);
      }
    },

    async closeDocument(path) {
      const document = documents.get(path);
      if (!document) return;
      clearTimers(document);
      documents.delete(path);
      pathByModelUri.delete(monaco.Uri.parse(document.uri).toString());
      const model = monaco.editor.getModel(monaco.Uri.parse(document.uri));
      if (model && !model.isDisposed()) {
        monaco.editor.setModelMarkers(model, MARKER_OWNER, []);
      }
      try {
        await invoke("lsp_did_close", {
          language: document.language,
          rootPath: document.rootPath,
          uri: document.uri,
        });
      } catch {
        // The server may already be gone; nothing to recover.
      }
    },

    languageFor(path) {
      return documents.get(path)?.language;
    },

    dispose() {
      disposed = true;
      documents.forEach(clearTimers);
      documents.clear();
      pathByModelUri.clear();
      registrations.forEach(({ disposables }) =>
        disposables.forEach((disposable) => disposable.dispose()),
      );
      registrations.clear();
    },
  };
}
