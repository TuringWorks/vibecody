/**
 * useLanguageRegistry — Central registry for all TIOBE top-50 programming languages.
 * Single source of truth used by all panels that deal with language-specific features.
 */

export interface LanguageEntry {
  id: string;           // internal canonical id (e.g. "python", "cpp")
  name: string;         // display name (e.g. "Python", "C++")
  tiobeRank: number;    // rank in TIOBE index April 2026
  monacoId: string;     // Monaco editor language identifier
  color: string;        // brand/highlight color
  extensions: string[]; // file extensions WITHOUT dot
  commentPrefix: string; // single-line comment prefix
  isVisual?: boolean;   // true for visual/block-based languages (no text files)
  tags: string[];       // e.g. ["systems", "scripting", "web", "functional", "data", "embedded", "blockchain"]
}

export const TIOBE_TOP50: LanguageEntry[] = [
  // Rank 1-10
  { id: "python", name: "Python", tiobeRank: 1, monacoId: "python", color: "#4584b6", extensions: ["py","pyw","pyi"], commentPrefix: "#", tags: ["scripting","data","web","ai"] },
  { id: "c", name: "C", tiobeRank: 2, monacoId: "c", color: "#555555", extensions: ["c","h"], commentPrefix: "//", tags: ["systems","embedded"] },
  { id: "cpp", name: "C++", tiobeRank: 3, monacoId: "cpp", color: "#f34b7d", extensions: ["cpp","cc","cxx","hpp","hh","hxx"], commentPrefix: "//", tags: ["systems","embedded"] },
  { id: "java", name: "Java", tiobeRank: 4, monacoId: "java", color: "#b07219", extensions: ["java"], commentPrefix: "//", tags: ["web","enterprise","android"] },
  { id: "csharp", name: "C#", tiobeRank: 5, monacoId: "csharp", color: "#178600", extensions: ["cs"], commentPrefix: "//", tags: ["web","enterprise","game"] },
  { id: "javascript", name: "JavaScript", tiobeRank: 6, monacoId: "javascript", color: "#f7df1e", extensions: ["js","jsx","mjs","cjs"], commentPrefix: "//", tags: ["web","scripting"] },
  { id: "vb", name: "Visual Basic", tiobeRank: 7, monacoId: "vb", color: "#945db7", extensions: ["vb","vbproj"], commentPrefix: "'", tags: ["enterprise","windows"] },
  { id: "sql", name: "SQL", tiobeRank: 8, monacoId: "sql", color: "#e38c00", extensions: ["sql","ddl","dml"], commentPrefix: "--", tags: ["data","database"] },
  { id: "r", name: "R", tiobeRank: 9, monacoId: "r", color: "#276dc3", extensions: ["r","R","rmd","Rmd"], commentPrefix: "#", tags: ["data","statistics"] },
  { id: "delphi", name: "Delphi/Object Pascal", tiobeRank: 10, monacoId: "pascal", color: "#e8274b", extensions: ["pas","pp","dpr","dfm"], commentPrefix: "//", tags: ["windows","enterprise"] },
  // Rank 11-20
  { id: "scratch", name: "Scratch", tiobeRank: 11, monacoId: "plaintext", color: "#f7a800", extensions: ["sb3","sb2","sb"], commentPrefix: "//", isVisual: true, tags: ["education","visual"] },
  { id: "perl", name: "Perl", tiobeRank: 12, monacoId: "perl", color: "#39457e", extensions: ["pl","pm","pod","t"], commentPrefix: "#", tags: ["scripting","text-processing"] },
  { id: "fortran", name: "Fortran", tiobeRank: 13, monacoId: "fortran", color: "#4d41b1", extensions: ["f","f90","f95","f03","f08","for","ftn"], commentPrefix: "!", tags: ["scientific","hpc"] },
  { id: "php", name: "PHP", tiobeRank: 14, monacoId: "php", color: "#777bb3", extensions: ["php","php3","php4","php5","phtml"], commentPrefix: "//", tags: ["web"] },
  { id: "go", name: "Go", tiobeRank: 15, monacoId: "go", color: "#00add8", extensions: ["go"], commentPrefix: "//", tags: ["systems","web","cloud"] },
  { id: "rust", name: "Rust", tiobeRank: 16, monacoId: "rust", color: "#dea584", extensions: ["rs"], commentPrefix: "//", tags: ["systems","web","embedded"] },
  { id: "matlab", name: "MATLAB", tiobeRank: 17, monacoId: "matlab", color: "#e16737", extensions: ["m","mat","mlx","mlapp"], commentPrefix: "%", tags: ["scientific","engineering","data"] },
  { id: "assembly", name: "Assembly", tiobeRank: 18, monacoId: "asm", color: "#6e4c13", extensions: ["asm","s","nasm","masm","S"], commentPrefix: ";", tags: ["systems","embedded","low-level"] },
  { id: "swift", name: "Swift", tiobeRank: 19, monacoId: "swift", color: "#fa7343", extensions: ["swift"], commentPrefix: "//", tags: ["mobile","apple"] },
  { id: "ada", name: "Ada", tiobeRank: 20, monacoId: "ada", color: "#02f88c", extensions: ["adb","ads","ada"], commentPrefix: "--", tags: ["systems","embedded","safety-critical"] },
  // Rank 21-30
  { id: "plsql", name: "PL/SQL", tiobeRank: 21, monacoId: "sql", color: "#da2b2b", extensions: ["pls","plsql","pkb","pks","pck"], commentPrefix: "--", tags: ["data","database","oracle"] },
  { id: "prolog", name: "Prolog", tiobeRank: 22, monacoId: "prolog", color: "#74283c", extensions: ["pl","pro","prolog"], commentPrefix: "%", tags: ["logic","ai"] },
  { id: "cobol", name: "COBOL", tiobeRank: 23, monacoId: "cobol", color: "#005ca5", extensions: ["cob","cbl","cpy","cbo"], commentPrefix: "*", tags: ["enterprise","legacy","finance"] },
  { id: "kotlin", name: "Kotlin", tiobeRank: 24, monacoId: "kotlin", color: "#a97bff", extensions: ["kt","kts"], commentPrefix: "//", tags: ["android","jvm","web"] },
  { id: "sas", name: "SAS", tiobeRank: 25, monacoId: "sas", color: "#1e90ff", extensions: ["sas","sas7bdat"], commentPrefix: "*", tags: ["data","statistics","enterprise"] },
  { id: "classic_vb", name: "Classic Visual Basic", tiobeRank: 26, monacoId: "vb", color: "#7b5ea7", extensions: ["bas","cls","frm","vbp"], commentPrefix: "'", tags: ["windows","legacy","enterprise"] },
  { id: "objc", name: "Objective-C", tiobeRank: 27, monacoId: "objective-c", color: "#438eff", extensions: ["m","mm"], commentPrefix: "//", tags: ["mobile","apple","legacy"] },
  { id: "dart", name: "Dart", tiobeRank: 28, monacoId: "dart", color: "#00b4ab", extensions: ["dart"], commentPrefix: "//", tags: ["mobile","web","flutter"] },
  { id: "ruby", name: "Ruby", tiobeRank: 29, monacoId: "ruby", color: "#701516", extensions: ["rb","erb","rake","gemspec","ru"], commentPrefix: "#", tags: ["web","scripting"] },
  { id: "lua", name: "Lua", tiobeRank: 30, monacoId: "lua", color: "#000080", extensions: ["lua"], commentPrefix: "--", tags: ["scripting","embedded","game"] },
  // Rank 31-40
  { id: "lisp", name: "Lisp", tiobeRank: 31, monacoId: "scheme", color: "#3fb68b", extensions: ["lisp","lsp","cl","el","fasl"], commentPrefix: ";", tags: ["functional","ai","legacy"] },
  { id: "julia", name: "Julia", tiobeRank: 32, monacoId: "julia", color: "#a270ba", extensions: ["jl"], commentPrefix: "#", tags: ["scientific","data","hpc"] },
  { id: "ml", name: "ML", tiobeRank: 33, monacoId: "fsharp", color: "#dc566d", extensions: ["ml","sml","fun","sig"], commentPrefix: "(*", tags: ["functional","academic"] },
  { id: "typescript", name: "TypeScript", tiobeRank: 34, monacoId: "typescript", color: "#3178c6", extensions: ["ts","tsx","d.ts"], commentPrefix: "//", tags: ["web","scripting"] },
  { id: "haskell", name: "Haskell", tiobeRank: 35, monacoId: "haskell", color: "#5e5086", extensions: ["hs","lhs"], commentPrefix: "--", tags: ["functional","academic"] },
  { id: "vbscript", name: "VBScript", tiobeRank: 36, monacoId: "vb", color: "#9b6dff", extensions: ["vbs","wsf"], commentPrefix: "'", tags: ["windows","scripting","legacy"] },
  { id: "abap", name: "ABAP", tiobeRank: 37, monacoId: "abap", color: "#e8274b", extensions: ["abap","prog","fugr","clas"], commentPrefix: "*", tags: ["enterprise","sap","erp"] },
  { id: "ocaml", name: "OCaml", tiobeRank: 38, monacoId: "fsharp", color: "#ef7a08", extensions: ["ml","mli","mly","mll"], commentPrefix: "(*", tags: ["functional","systems"] },
  { id: "zig", name: "Zig", tiobeRank: 39, monacoId: "zig", color: "#ec915c", extensions: ["zig","zon"], commentPrefix: "//", tags: ["systems","embedded"] },
  { id: "caml", name: "Caml", tiobeRank: 40, monacoId: "fsharp", color: "#dc566d", extensions: ["ml","cmo","cmi"], commentPrefix: "(*", tags: ["functional","academic"] },
  // Rank 41-50
  { id: "erlang", name: "Erlang", tiobeRank: 41, monacoId: "erlang", color: "#b83998", extensions: ["erl","hrl","beam"], commentPrefix: "%", tags: ["functional","distributed","telecom"] },
  { id: "xpp", name: "X++", tiobeRank: 42, monacoId: "csharp", color: "#008575", extensions: ["xpp","axpp"], commentPrefix: "//", tags: ["enterprise","microsoft","erp"] },
  { id: "scala", name: "Scala", tiobeRank: 43, monacoId: "scala", color: "#dc322f", extensions: ["scala","sc"], commentPrefix: "//", tags: ["jvm","functional","data"] },
  { id: "tsql", name: "Transact-SQL", tiobeRank: 44, monacoId: "sql", color: "#e38c00", extensions: ["tsql","sql"], commentPrefix: "--", tags: ["data","database","microsoft"] },
  { id: "powershell", name: "PowerShell", tiobeRank: 45, monacoId: "powershell", color: "#012456", extensions: ["ps1","psm1","psd1","ps1xml"], commentPrefix: "#", tags: ["scripting","windows","devops"] },
  { id: "gml", name: "GML", tiobeRank: 46, monacoId: "javascript", color: "#71b417", extensions: ["gml","yy","yyp"], commentPrefix: "//", tags: ["game","scripting"] },
  { id: "labview", name: "LabVIEW", tiobeRank: 47, monacoId: "plaintext", color: "#fccc03", extensions: ["vi","lvproj","ctl"], commentPrefix: "//", isVisual: true, tags: ["engineering","data-acquisition","visual"] },
  { id: "ladder_logic", name: "Ladder Logic", tiobeRank: 48, monacoId: "plaintext", color: "#0078d4", extensions: ["ld","lad","plc"], commentPrefix: "//", isVisual: true, tags: ["plc","industrial","embedded","visual"] },
  { id: "solidity", name: "Solidity", tiobeRank: 49, monacoId: "sol", color: "#363636", extensions: ["sol"], commentPrefix: "//", tags: ["blockchain","web3","ethereum"] },
  { id: "foxpro", name: "(Visual) FoxPro", tiobeRank: 50, monacoId: "plaintext", color: "#1984c5", extensions: ["prg","dbc","dbf","vcx","scx"], commentPrefix: "&&", tags: ["legacy","database","windows"] },
];

/** Map from file extension to language entry (first match wins — TIOBE rank order) */
export const EXT_TO_LANGUAGE: Record<string, LanguageEntry> = (() => {
  const map: Record<string, LanguageEntry> = {};
  for (const lang of TIOBE_TOP50) {
    for (const ext of lang.extensions) {
      if (!map[ext]) map[ext] = lang; // first match wins (handles conflicts like .m → MATLAB beats Objective-C)
    }
  }
  return map;
})();

/** Get language entry from file path */
export function getLanguageFromPath(filePath: string): LanguageEntry | undefined {
  const parts = filePath.split(".");
  if (parts.length < 2) return undefined;
  const ext = parts[parts.length - 1].toLowerCase();
  return EXT_TO_LANGUAGE[ext];
}

/** Hook — returns the registry and helpers */
export function useLanguageRegistry() {
  return {
    languages: TIOBE_TOP50,
    getByExtension: (ext: string) => EXT_TO_LANGUAGE[ext.toLowerCase().replace(/^\./, "")],
    getByPath: getLanguageFromPath,
    getById: (id: string) => TIOBE_TOP50.find(l => l.id === id),
    textLanguages: TIOBE_TOP50.filter(l => !l.isVisual),
  };
}
