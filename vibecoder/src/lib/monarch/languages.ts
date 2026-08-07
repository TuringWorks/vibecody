/**
 * Grammars for the languages Monaco does not ship one for.
 *
 * Every entry here corresponds to a language VibeCody routes to a real language
 * server (see `lib/lsp.ts`), so these files previously had IntelliSense but
 * rendered as flat grey text. Keyword lists are the reserved words plus the
 * built-in types people actually read for — not exhaustive standard libraries,
 * which colour half the file and defeat the purpose.
 *
 * Monaco's own grammars stay authoritative: nothing here duplicates a language
 * from `monaco-editor/esm/vs/basic-languages`, and a test asserts that.
 */

import type { LanguageSpec } from "./spec";

// ── Modern systems languages ────────────────────────────────────────────────

const zig: LanguageSpec = {
  id: "zig",
  extensions: [".zig", ".zon"],
  aliases: ["Zig"],
  comments: { line: ["//"] },
  // Zig has no block comments at all — a `/* */` rule would be actively wrong.
  strings: { double: true, chars: true },
  keywords: [
    "align", "allowzero", "and", "anyframe", "anytype", "asm", "async", "await",
    "break", "callconv", "catch", "comptime", "const", "continue", "defer",
    "else", "enum", "errdefer", "error", "export", "extern", "fn", "for", "if",
    "inline", "linksection", "noalias", "noinline", "nosuspend", "opaque", "or",
    "orelse", "packed", "pub", "resume", "return", "struct", "suspend", "switch",
    "test", "threadlocal", "try", "union", "unreachable", "usingnamespace", "var",
    "volatile", "while",
  ],
  types: [
    "i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64", "i128", "u128",
    "isize", "usize", "c_char", "c_short", "c_ushort", "c_int", "c_uint",
    "c_long", "c_ulong", "c_longlong", "c_ulonglong", "c_longdouble",
    "f16", "f32", "f64", "f80", "f128", "bool", "void", "noreturn", "type",
    "anyerror", "comptime_int", "comptime_float",
  ],
  constants: ["true", "false", "null", "undefined"],
  extra: [
    // `\\` opens a multi-line string literal that runs to end of line.
    [/\\\\.*$/, "string"],
    // Builtins are `@import`, `@sizeOf`, … — always `@` + identifier.
    [/@[a-zA-Z_]\w*/, "predefined"],
  ],
};

const nim: LanguageSpec = {
  id: "nim",
  extensions: [".nim", ".nims", ".nimble"],
  aliases: ["Nim"],
  // `#[ … ]#` nests, and `##` is a doc comment (covered by the `#` rule).
  comments: { line: ["#"], block: ["#[", "]#"], nested: true },
  strings: { double: true, tripleDouble: true, chars: true },
  keywords: [
    "addr", "and", "as", "asm", "bind", "block", "break", "case", "cast",
    "concept", "const", "continue", "converter", "defer", "discard", "distinct",
    "div", "do", "elif", "else", "end", "enum", "except", "export", "finally",
    "for", "from", "func", "if", "import", "in", "include", "interface", "is",
    "isnot", "iterator", "let", "macro", "method", "mixin", "mod", "not",
    "notin", "object", "of", "or", "out", "proc", "ptr", "raise", "ref",
    "return", "shl", "shr", "static", "template", "try", "tuple", "type",
    "using", "var", "when", "while", "xor", "yield",
  ],
  types: [
    "int", "int8", "int16", "int32", "int64", "uint", "uint8", "uint16",
    "uint32", "uint64", "float", "float32", "float64", "bool", "char", "string",
    "cstring", "pointer", "seq", "array", "openArray", "set", "range", "auto",
    "any", "untyped", "typed", "void", "Natural", "Positive",
  ],
  constants: ["true", "false", "nil"],
};

const crystal: LanguageSpec = {
  id: "crystal",
  extensions: [".cr"],
  aliases: ["Crystal"],
  comments: { line: ["#"] },
  strings: { double: true, chars: true },
  keywords: [
    "abstract", "alias", "annotation", "as", "asm", "begin", "break", "case",
    "class", "def", "do", "else", "elsif", "end", "ensure", "enum", "extend",
    "for", "fun", "if", "in", "include", "instance_sizeof", "is_a?", "lib",
    "macro", "module", "next", "nil?", "of", "out", "pointerof", "private",
    "protected", "require", "rescue", "responds_to?", "return", "select",
    "sizeof", "struct", "super", "then", "type", "typeof", "union", "unless",
    "until", "verbatim", "when", "while", "with", "yield",
  ],
  types: [
    "Int8", "Int16", "Int32", "Int64", "Int128", "UInt8", "UInt16", "UInt32",
    "UInt64", "UInt128", "Float32", "Float64", "Bool", "Char", "String",
    "Symbol", "Array", "Hash", "Tuple", "NamedTuple", "Range", "Set", "Nil",
    "Proc", "Slice", "StaticArray", "Pointer", "Void",
  ],
  constants: ["true", "false", "nil", "self"],
  extra: [
    // `\x40` is a literal `@`. Written as `@` Monarch may read it as an
    // attribute reference, and `@@` it treats as an *escaped* `@` — so the
    // obvious `@@?` collapses to `@?`, an optional at-sign, and matches every
    // bare identifier in the file as a variable.
    [/\x40\x40[a-zA-Z_]\w*/, "variable"], // @@class_var
    [/\x40[a-zA-Z_]\w*/, "variable"], // @instance_var
    [/:[a-zA-Z_]\w*[?!]?/, "string.symbol"],
  ],
};

const vlang: LanguageSpec = {
  id: "v",
  extensions: [".v", ".vsh"],
  aliases: ["V"],
  comments: { line: ["//"], block: ["/*", "*/"] },
  strings: { double: true, single: true, backtick: true },
  keywords: [
    "as", "asm", "assert", "atomic", "break", "const", "continue", "defer",
    "else", "enum", "fn", "for", "go", "goto", "if", "import", "in", "interface",
    "is", "isreftype", "lock", "match", "module", "mut", "none", "or", "pub",
    "return", "rlock", "select", "shared", "sizeof", "spawn", "static",
    "struct", "type", "typeof", "union", "unsafe", "volatile", "__global",
  ],
  types: [
    "bool", "string", "i8", "i16", "int", "i64", "i128", "u8", "u16", "u32",
    "u64", "u128", "rune", "f32", "f64", "isize", "usize", "voidptr", "any",
    "byte", "charptr", "byteptr", "map", "array", "thread", "chan",
  ],
  constants: ["true", "false", "none", "nil"],
};

const dlang: LanguageSpec = {
  id: "d",
  extensions: [".d", ".di"],
  aliases: ["D"],
  // D has three comment forms; `/+ +/` is the nesting one. Modelled as the
  // nesting `block` because that is the one where getting it wrong is fatal.
  comments: { line: ["//"], block: ["/+", "+/"], block2: ["/*", "*/"], nested: true },
  strings: { double: true, backtick: true, chars: true },
  keywords: [
    "abstract", "alias", "align", "asm", "assert", "auto", "body", "break",
    "case", "cast", "catch", "class", "const", "continue", "debug", "default",
    "delegate", "delete", "deprecated", "do", "else", "enum", "export",
    "extern", "final", "finally", "for", "foreach", "foreach_reverse",
    "function", "goto", "if", "immutable", "import", "in", "inout", "interface",
    "invariant", "is", "lazy", "macro", "mixin", "module", "new", "nothrow",
    "out", "override", "package", "pragma", "private", "protected", "public",
    "pure", "ref", "return", "scope", "shared", "static", "struct", "super",
    "switch", "synchronized", "template", "throw", "try", "typeid", "typeof",
    "union", "unittest", "version", "while", "with", "__gshared", "__traits",
  ],
  types: [
    "bool", "byte", "ubyte", "short", "ushort", "int", "uint", "long", "ulong",
    "cent", "ucent", "float", "double", "real", "ifloat", "idouble", "ireal",
    "cfloat", "cdouble", "creal", "char", "wchar", "dchar", "void", "string",
    "wstring", "dstring", "size_t", "ptrdiff_t",
  ],
  constants: ["true", "false", "null", "this"],
};

const vala: LanguageSpec = {
  id: "vala",
  extensions: [".vala", ".vapi"],
  aliases: ["Vala"],
  comments: { line: ["//"], block: ["/*", "*/"] },
  strings: { double: true, tripleDouble: true, chars: true },
  keywords: [
    "abstract", "as", "async", "base", "break", "case", "catch", "class",
    "const", "construct", "continue", "default", "delegate", "delete", "do",
    "dynamic", "else", "enum", "ensures", "errordomain", "extern", "finally",
    "for", "foreach", "get", "if", "in", "inline", "interface", "internal",
    "is", "lock", "namespace", "new", "out", "override", "owned", "params",
    "private", "protected", "public", "ref", "requires", "return", "set",
    "signal", "sizeof", "static", "struct", "switch", "throw", "throws",
    "try", "typeof", "unowned", "using", "value", "var", "virtual", "void",
    "weak", "while", "yield",
  ],
  types: [
    "bool", "char", "uchar", "short", "ushort", "int", "uint", "long", "ulong",
    "size_t", "ssize_t", "int8", "uint8", "int16", "uint16", "int32", "uint32",
    "int64", "uint64", "unichar", "float", "double", "string", "void",
  ],
  constants: ["true", "false", "null", "this"],
};

const odin: LanguageSpec = {
  id: "odin",
  extensions: [".odin"],
  aliases: ["Odin"],
  comments: { line: ["//"], block: ["/*", "*/"], nested: true },
  strings: { double: true, backtick: true, chars: true },
  keywords: [
    "asm", "auto_cast", "bit_field", "bit_set", "break", "case", "cast",
    "context", "continue", "defer", "distinct", "do", "dynamic", "else",
    "enum", "fallthrough", "for", "foreign", "if", "import", "in", "map",
    "matrix", "not_in", "or_else", "or_return", "package", "proc", "return",
    "struct", "switch", "transmute", "typeid", "union", "using", "when",
    "where",
  ],
  types: [
    "bool", "b8", "b16", "b32", "b64", "int", "i8", "i16", "i32", "i64",
    "i128", "uint", "u8", "u16", "u32", "u64", "u128", "uintptr", "f16",
    "f32", "f64", "complex32", "complex64", "complex128", "quaternion64",
    "quaternion128", "quaternion256", "rune", "string", "cstring", "rawptr",
    "any", "byte",
  ],
  constants: ["true", "false", "nil"],
  // `#partial`, `#no_bounds_check`, … are compiler directives.
  extra: [[/#[a-zA-Z_]\w*/, "predefined"]],
};

const gleam: LanguageSpec = {
  id: "gleam",
  extensions: [".gleam"],
  aliases: ["Gleam"],
  comments: { line: ["//"] },
  strings: { double: true },
  keywords: [
    "as", "assert", "auto", "case", "const", "delegate", "derive", "echo",
    "else", "fn", "if", "implement", "import", "let", "macro", "opaque",
    "panic", "pub", "test", "todo", "type", "use",
  ],
  types: ["Int", "Float", "String", "Bool", "List", "Result", "Nil", "BitArray"],
  constants: ["True", "False", "Ok", "Error", "Nil"],
  extra: [
    // `@external`, `@target`, `@deprecated`
    [/@[a-zA-Z_]\w*/, "annotation"],
    // Discard patterns and labels read better as parameters.
    [/_[a-zA-Z_]\w*/, "variable.parameter"],
  ],
};

// ── Functional languages ────────────────────────────────────────────────────

const haskell: LanguageSpec = {
  id: "haskell",
  extensions: [".hs", ".lhs"],
  aliases: ["Haskell"],
  comments: { line: ["--"], block: ["{-", "-}"], nested: true },
  strings: { double: true, chars: true },
  keywords: [
    "case", "class", "data", "default", "deriving", "do", "else", "family",
    "forall", "foreign", "hiding", "if", "import", "in", "infix", "infixl",
    "infixr", "instance", "let", "mdo", "module", "newtype", "of", "pattern",
    "proc", "qualified", "rec", "then", "type", "where",
  ],
  types: [
    "Int", "Integer", "Float", "Double", "Rational", "Char", "String", "Bool",
    "Maybe", "Either", "Ordering", "IO", "IOError", "Word", "Ratio", "Map",
    "Set", "Text", "ByteString", "Monad", "Functor", "Applicative", "Show",
    "Eq", "Ord", "Num",
  ],
  constants: ["True", "False", "Nothing", "Just", "Left", "Right", "LT", "EQ", "GT"],
};

const elm: LanguageSpec = {
  id: "elm",
  extensions: [".elm"],
  aliases: ["Elm"],
  comments: { line: ["--"], block: ["{-", "-}"], nested: true },
  strings: { double: true, tripleDouble: true, chars: true },
  keywords: [
    "alias", "as", "case", "effect", "else", "exposing", "if", "import", "in",
    "let", "module", "of", "port", "then", "type", "where",
  ],
  types: [
    "Int", "Float", "Bool", "Char", "String", "List", "Maybe", "Result",
    "Order", "Cmd", "Sub", "Program", "Html", "Never",
  ],
  constants: ["True", "False", "Nothing", "Just", "Ok", "Err", "LT", "EQ", "GT"],
};

const purescript: LanguageSpec = {
  id: "purescript",
  extensions: [".purs"],
  aliases: ["PureScript"],
  comments: { line: ["--"], block: ["{-", "-}"], nested: true },
  strings: { double: true, tripleDouble: true, chars: true },
  keywords: [
    "ado", "as", "case", "class", "data", "derive", "do", "else", "false",
    "forall", "foreign", "hiding", "if", "import", "in", "infix", "infixl",
    "infixr", "instance", "let", "module", "newtype", "of", "then", "true",
    "type", "where",
  ],
  types: [
    "Int", "Number", "Boolean", "Char", "String", "Array", "Maybe", "Either",
    "Ordering", "Effect", "Aff", "Unit", "Void", "Record", "Functor", "Monad",
  ],
  constants: ["true", "false", "Nothing", "Just", "Left", "Right", "unit"],
};

const rescript: LanguageSpec = {
  id: "rescript",
  extensions: [".res", ".resi"],
  aliases: ["ReScript"],
  comments: { line: ["//"], block: ["/*", "*/"] },
  strings: { double: true, backtick: true, chars: true },
  keywords: [
    "and", "as", "assert", "async", "await", "constraint", "downto", "else",
    "exception", "external", "for", "if", "in", "include", "lazy", "let",
    "module", "mutable", "of", "open", "rec", "switch", "to", "try", "type",
    "when", "while", "with",
  ],
  types: [
    "int", "float", "string", "char", "bool", "unit", "array", "list",
    "option", "result", "promise", "dict", "Js", "Belt",
  ],
  constants: ["true", "false", "None", "Some", "Ok", "Error"],
  // `@react.component`, `@@warning`, `%%raw`. `\x40` for the same reason as in
  // the Crystal spec above.
  extra: [
    [/\x40\x40?[a-zA-Z_][\w.]*/, "annotation"],
    [/%%?[a-zA-Z_]\w*/, "predefined"],
  ],
};

// ── Infrastructure / build ──────────────────────────────────────────────────

const nix: LanguageSpec = {
  id: "nix",
  extensions: [".nix"],
  aliases: ["Nix"],
  comments: { line: ["#"], block: ["/*", "*/"] },
  strings: { double: true },
  keywords: [
    "assert", "else", "if", "in", "inherit", "let", "or", "rec", "then",
    "with",
  ],
  builtins: [
    "builtins", "import", "derivation", "map", "toString", "removeAttrs",
    "abort", "throw", "fetchTarball", "fetchGit",
  ],
  constants: ["true", "false", "null"],
  extra: [
    // `''…''` indented strings. Must precede the `'` handling and the `''`
    // would otherwise read as two empty char literals.
    [/''/, { token: "string.quote", next: "@nixIndentedString" }],
    // Paths: `./foo`, `../bar`, `<nixpkgs>`.
    [/\.{0,2}\/[\w./-]+/, "string.link"],
    [/<[\w./-]+>/, "string.link"],
  ],
  trailing: [],
};

// Extra state for Nix's indented strings, merged in below.
const NIX_EXTRA_STATES = {
  nixIndentedString: [
    [/[^']+/, "string"],
    [/''\$/, "string.escape"], // escaped interpolation
    [/'''/, "string.escape"],
    [/''/, { token: "string.quote", next: "@pop" }],
    [/'/, "string"],
  ],
} as const;

const cmake: LanguageSpec = {
  id: "cmake",
  extensions: [".cmake"],
  filenames: ["CMakeLists.txt"],
  aliases: ["CMake"],
  // CMake commands are case-insensitive, and `#[[ … ]]` is its bracket comment.
  ignoreCase: true,
  comments: { line: ["#"], block: ["#[[", "]]"] },
  strings: { double: true },
  keywords: [
    "if", "elseif", "else", "endif", "foreach", "endforeach", "while",
    "endwhile", "function", "endfunction", "macro", "endmacro", "break",
    "continue", "return", "include", "include_guard", "set", "unset", "list",
    "string", "math", "option", "message", "project", "cmake_minimum_required",
    "add_executable", "add_library", "add_subdirectory", "add_custom_command",
    "add_custom_target", "add_dependencies", "add_definitions",
    "target_link_libraries", "target_include_directories",
    "target_compile_options", "target_compile_definitions",
    "target_compile_features", "target_sources", "find_package", "find_library",
    "find_path", "find_program", "install", "enable_testing", "add_test",
    "configure_file", "file", "get_filename_component", "separate_arguments",
    "source_group", "set_property", "get_property", "set_target_properties",
  ],
  constants: ["ON", "OFF", "TRUE", "FALSE", "YES", "NO", "NOTFOUND"],
  extra: [
    // `${VAR}` / `$ENV{VAR}` / `$<GENEX>` — the thing you actually scan for.
    [/\$(?:ENV|CACHE)?\{[^}]*\}/, "variable"],
    [/\$<[^>]*>/, "variable"],
  ],
};

/**
 * CMake interpolates inside double quotes, and `"${CMAKE_SOURCE_DIR}/src"` is
 * how half of every CMakeLists.txt reads — so the string state has to see
 * variables too. Overrides the generated `doubleString`.
 */
const CMAKE_EXTRA_STATES = {
  doubleString: [
    [/\$(?:ENV|CACHE)?\{[^}]*\}/, "variable"],
    [/[^\\"$]+/, "string"],
    [/\\./, "string.escape"],
    [/\$/, "string"],
    [/"/, { token: "string.quote", next: "@pop" }],
  ],
} as const;

// ── Typesetting ─────────────────────────────────────────────────────────────

const latex: LanguageSpec = {
  id: "latex",
  extensions: [".tex", ".ltx", ".sty", ".cls", ".bib"],
  aliases: ["LaTeX", "TeX"],
  comments: { line: ["%"] },
  // TeX has no string literals; quotes are ordinary characters.
  strings: { double: false },
  keywords: [],
  extra: [
    // `\begin{...}` / `\end{...}` carry the structure, so name the environment.
    [/(\\(?:begin|end))(\s*)(\{)([^}]*)(\})/, ["keyword", "white", "@brackets", "type", "@brackets"]],
    [/\\[a-zA-Z@]+\*?/, "keyword"], // \section, \textbf, \newcommand
    [/\\[^a-zA-Z]/, "string.escape"], // \\, \%, \&
    [/\$\$/, { token: "string", next: "@latexDisplayMath" }],
    [/\$/, { token: "string", next: "@latexInlineMath" }],
    [/[&~^_]/, "operator"],
    [/#\d/, "variable.parameter"], // macro arguments
  ],
};

const LATEX_EXTRA_STATES = {
  latexInlineMath: [
    [/\\[a-zA-Z@]+\*?/, "keyword"],
    [/[^$\\]+/, "string"],
    [/\$/, { token: "string", next: "@pop" }],
    [/./, "string"],
  ],
  latexDisplayMath: [
    [/\\[a-zA-Z@]+\*?/, "keyword"],
    [/[^$\\]+/, "string"],
    [/\$\$/, { token: "string", next: "@pop" }],
    [/./, "string"],
  ],
} as const;

// ── Scientific / engineering ────────────────────────────────────────────────

const matlab: LanguageSpec = {
  id: "matlab",
  extensions: [".m", ".mlx", ".mlapp"],
  aliases: ["MATLAB"],
  // `%{ … %}` must be alone on its lines in real MATLAB; treated as a plain
  // block form here, which is right for every well-formed file.
  comments: { line: ["%", "..."], block: ["%{", "%}"] },
  // Double-quoted strings are MATLAB R2017a+. Single quotes are handled in
  // `extra` because a bare `'` is also the transpose operator.
  strings: { double: true },
  keywords: [
    "break", "case", "catch", "classdef", "continue", "else", "elseif", "end",
    "for", "function", "global", "if", "otherwise", "parfor", "persistent",
    "properties", "methods", "events", "enumeration", "return", "spmd",
    "switch", "try", "while", "arguments",
  ],
  types: [
    "double", "single", "int8", "int16", "int32", "int64", "uint8", "uint16",
    "uint32", "uint64", "logical", "char", "string", "cell", "struct",
    "function_handle", "table", "categorical",
  ],
  constants: ["true", "false", "pi", "Inf", "inf", "NaN", "nan", "eps", "NaT"],
  extra: [
    // MATLAB's `'` is both the transpose operator and the char-array quote, and
    // Monarch matches at the cursor with no lookbehind available — so ordering
    // is the disambiguation. A *complete* quoted run on one line is a string;
    // a quote with no partner is transpose. That gets `s = 'hi'` and `B = A'`
    // both right. (`a' + b'` still reads as one string — genuinely ambiguous,
    // and every editor guesses here.)
    [/'(?:[^'\\\r\n]|'')*'/, "string"],
    [/'/, "operator"],
    [/~=/, "operator"],
  ],
};

const assembly: LanguageSpec = {
  id: "asm",
  extensions: [".asm", ".s", ".nasm", ".S"],
  aliases: ["Assembly"],
  // `;` is the classic marker; `#` is used by GAS on several targets.
  comments: { line: [";", "#"], block: ["/*", "*/"] },
  strings: { double: true, single: true },
  keywords: [
    // The x86/ARM instructions and directives worth colouring; not a full ISA.
    "mov", "movl", "movq", "movzx", "movsx", "lea", "push", "pop", "add",
    "sub", "mul", "imul", "div", "idiv", "inc", "dec", "neg", "and", "or",
    "xor", "not", "shl", "shr", "sar", "cmp", "test", "jmp", "je", "jne",
    "jz", "jnz", "jg", "jge", "jl", "jle", "ja", "jb", "call", "ret", "leave",
    "enter", "nop", "hlt", "int", "syscall", "loop", "in", "out",
    "ldr", "str", "adr", "bl", "bx", "blx", "cbz", "cbnz", "svc", "adrp",
  ],
  types: [
    "byte", "word", "dword", "qword", "tbyte", "short", "long", "quad",
    "ascii", "asciz", "zero", "space",
  ],
  extra: [
    // Directives (`.text`, `.globl`, `%macro`) and labels (`main:`).
    [/^\s*[.@][a-zA-Z_][\w.]*/, "keyword.directive"],
    [/^\s*[a-zA-Z_.$][\w.$]*:/, "type.identifier"],
    [/%[a-zA-Z_]\w*/, "variable.predefined"], // NASM macros, AT&T registers
    [/\$[\w-]+/, "number"], // AT&T immediates
  ],
};

// ── Enterprise / legacy ─────────────────────────────────────────────────────

const cobol: LanguageSpec = {
  id: "cobol",
  extensions: [".cob", ".cbl", ".cpy", ".cbo", ".ccp"],
  aliases: ["COBOL"],
  ignoreCase: true,
  // `*>` is the modern inline form; a `*` in column 7 is the fixed-format one,
  // handled in `extra` because it depends on position, not just the marker.
  comments: { line: ["*>"] },
  strings: { double: true, single: true },
  keywords: [
    "ACCEPT", "ADD", "ALTER", "AND", "CALL", "CANCEL", "CLOSE", "COMPUTE",
    "CONFIGURATION", "CONTINUE", "COPY", "DATA", "DECLARATIVES", "DELETE",
    "DISPLAY", "DIVIDE", "DIVISION", "ELSE", "END", "END-IF", "END-PERFORM",
    "END-EVALUATE", "ENVIRONMENT", "EVALUATE", "EXIT", "FD", "FILE",
    "FILE-CONTROL", "GO", "GOBACK", "IDENTIFICATION", "IF", "INITIALIZE",
    "INPUT-OUTPUT", "INSPECT", "INTO", "LINKAGE", "LOCAL-STORAGE", "MERGE",
    "MOVE", "MULTIPLY", "NOT", "OPEN", "OR", "PERFORM", "PICTURE", "PIC",
    "PROCEDURE", "PROGRAM-ID", "READ", "RELEASE", "RETURN", "REWRITE",
    "SEARCH", "SECTION", "SELECT", "SET", "SORT", "STOP", "STRING",
    "SUBTRACT", "THEN", "TO", "UNSTRING", "UNTIL", "USING", "VALUE",
    "VARYING", "WHEN", "WORKING-STORAGE", "WRITE",
  ],
  types: [
    "BINARY", "COMP", "COMP-1", "COMP-2", "COMP-3", "DISPLAY", "PACKED-DECIMAL",
    "POINTER", "USAGE",
  ],
  constants: ["ZERO", "ZEROS", "ZEROES", "SPACE", "SPACES", "HIGH-VALUE",
    "HIGH-VALUES", "LOW-VALUE", "LOW-VALUES", "NULL", "NULLS", "TRUE", "FALSE"],
  identifier: /[a-zA-Z][\w-]*/, // COBOL names contain hyphens
  extra: [
    // Fixed format: columns 1-6 are sequence numbers, column 7 is the
    // indicator area — `*` or `/` there comments the whole line.
    [/^.{6}[*/].*$/, "comment"],
    [/^\s*\*.*$/, "comment"],
    // Picture strings (`PIC X(10)`, `9(5)V99`) are not identifiers.
    [/\b[9XAVSPZ]+\(\d+\)(?:[9XAVSPZ]+(?:\(\d+\))?)*/, "string"],
  ],
};

const sas: LanguageSpec = {
  id: "sas",
  extensions: [".sas"],
  aliases: ["SAS"],
  ignoreCase: true,
  comments: { block: ["/*", "*/"] },
  strings: { double: true, single: true },
  keywords: [
    "data", "set", "run", "quit", "proc", "if", "then", "else", "do", "end",
    "while", "until", "by", "output", "input", "infile", "file", "put",
    "keep", "drop", "rename", "where", "merge", "retain", "array", "length",
    "format", "informat", "label", "libname", "filename", "options", "title",
    "footnote", "var", "class", "model", "tables", "means", "sum", "select",
    "when", "otherwise", "return", "stop", "delete", "call", "link",
  ],
  builtins: [
    "sum", "mean", "min", "max", "abs", "round", "int", "substr", "scan",
    "trim", "left", "right", "upcase", "lowcase", "index", "length", "today",
    "datepart", "timepart", "put", "input", "lag", "dif",
  ],
  extra: [
    // Macro language: `%macro`, `%do`, `&var`, `&&var`.
    [/%[a-zA-Z_]\w*/, "keyword.macro"],
    [/&&?[a-zA-Z_]\w*\.?/, "variable"],
    // `* comment ;` — a statement-style comment, terminated by a semicolon.
    [/^\s*\*[^;]*;/, "comment"],
  ],
};

const ada: LanguageSpec = {
  id: "ada",
  extensions: [".adb", ".ads", ".ada"],
  aliases: ["Ada"],
  ignoreCase: true,
  comments: { line: ["--"] },
  // `'` handled entirely in `extra` — see below.
  strings: { double: true },
  keywords: [
    "abort", "abs", "abstract", "accept", "access", "aliased", "all", "and",
    "array", "at", "begin", "body", "case", "constant", "declare", "delay",
    "delta", "digits", "do", "else", "elsif", "end", "entry", "exception",
    "exit", "for", "function", "generic", "goto", "if", "in", "interface",
    "is", "limited", "loop", "mod", "new", "not", "null", "of", "or", "others",
    "out", "overriding", "package", "parallel", "pragma", "private",
    "procedure", "protected", "raise", "range", "record", "rem", "renames",
    "requeue", "return", "reverse", "select", "separate", "some", "subtype",
    "synchronized", "tagged", "task", "terminate", "then", "type", "until",
    "use", "when", "while", "with", "xor",
  ],
  types: [
    "Integer", "Natural", "Positive", "Float", "Long_Float", "Duration",
    "Character", "Wide_Character", "String", "Wide_String", "Boolean",
    "Short_Integer", "Long_Integer", "Long_Long_Integer",
  ],
  constants: ["True", "False", "null"],
  extra: [
    // A complete character literal first: `'x'`. If the attribute rule below
    // ran first it would consume `'x` and leave the closing quote dangling.
    [/'(?:[^'\\]|\\.)'/, "string"],
    // Attributes: `Obj'Length`, `Integer'Image` — a quote that is not a quote.
    [/'[A-Za-z]\w*/, "predefined"],
  ],
};

const fortran: LanguageSpec = {
  id: "fortran",
  extensions: [".f", ".f90", ".f95", ".f03", ".f08", ".for", ".ftn", ".fpp"],
  aliases: ["Fortran"],
  ignoreCase: true,
  comments: { line: ["!"] },
  strings: { double: true, single: true },
  keywords: [
    "allocatable", "allocate", "assign", "associate", "asynchronous", "backspace",
    "bind", "block", "call", "case", "class", "close", "common", "contains",
    "continue", "cycle", "data", "deallocate", "default", "deferred", "do",
    "else", "elseif", "elsewhere", "end", "enddo", "endif", "entry", "enum",
    "equivalence", "exit", "extends", "external", "final", "flush", "forall",
    "format", "function", "generic", "goto", "if", "implicit", "import",
    "include", "inquire", "intent", "interface", "intrinsic", "module",
    "namelist", "nullify", "only", "open", "operator", "optional", "parameter",
    "pass", "pointer", "print", "private", "procedure", "program", "protected",
    "public", "pure", "read", "recursive", "result", "return", "rewind",
    "save", "select", "sequence", "stop", "submodule", "subroutine", "target",
    "then", "type", "use", "value", "volatile", "where", "while", "write",
  ],
  types: [
    "integer", "real", "double", "precision", "complex", "logical",
    "character", "dimension", "kind", "len",
  ],
  constants: [".true.", ".false."],
  extra: [
    // Fixed-form: `c`, `C` or `*` in column 1 comments the line.
    [/^[cC*].*$/, "comment"],
    // Logical literals and operators are dotted: `.and.`, `.true.`
    [/\.(?:true|false|and|or|not|eqv|neqv|eq|ne|lt|le|gt|ge)\./, "keyword"],
  ],
};

const prolog: LanguageSpec = {
  id: "prolog",
  extensions: [".pro", ".prolog", ".plg"],
  aliases: ["Prolog"],
  comments: { line: ["%"], block: ["/*", "*/"] },
  strings: { double: true, single: true },
  keywords: [
    "module", "use_module", "dynamic", "discontiguous", "initialization",
    "is", "mod", "rem", "div", "abs", "not", "fail", "true", "false", "halt",
    "assert", "asserta", "assertz", "retract", "findall", "bagof", "setof",
    "forall", "between", "succ_or_zero", "catch", "throw",
  ],
  extra: [
    // Variables are Capitalised or `_`-led; atoms are lower-case. That
    // distinction *is* the language, so it must survive the identifier rule.
    [/[A-Z_]\w*/, "variable"],
    [/:-|-->/, "keyword.operator"],
    [/!/, "keyword"], // the cut
  ],
};

const vhdl: LanguageSpec = {
  id: "vhdl",
  extensions: [".vhd", ".vhdl", ".vho"],
  aliases: ["VHDL"],
  ignoreCase: true,
  comments: { line: ["--"] },
  strings: { double: true, chars: true },
  keywords: [
    "abs", "access", "after", "alias", "all", "and", "architecture", "array",
    "assert", "attribute", "begin", "block", "body", "buffer", "bus", "case",
    "component", "configuration", "constant", "disconnect", "downto", "else",
    "elsif", "end", "entity", "exit", "file", "for", "function", "generate",
    "generic", "group", "guarded", "if", "impure", "in", "inertial", "inout",
    "is", "label", "library", "linkage", "literal", "loop", "map", "mod",
    "nand", "new", "next", "nor", "not", "null", "of", "on", "open", "or",
    "others", "out", "package", "port", "postponed", "procedure", "process",
    "pure", "range", "record", "register", "reject", "rem", "report",
    "return", "rol", "ror", "select", "severity", "signal", "shared", "sla",
    "sll", "sra", "srl", "subtype", "then", "to", "transport", "type",
    "unaffected", "units", "until", "use", "variable", "wait", "when",
    "while", "with", "xnor", "xor",
  ],
  types: [
    "bit", "bit_vector", "boolean", "character", "integer", "natural",
    "positive", "real", "severity_level", "signed", "std_logic",
    "std_logic_vector", "std_ulogic", "string", "time", "unsigned",
  ],
  constants: ["true", "false"],
};

const foxpro: LanguageSpec = {
  id: "foxpro",
  extensions: [".prg", ".prw", ".spr"],
  aliases: ["FoxPro"],
  ignoreCase: true,
  comments: { line: ["&&"] },
  strings: { double: true, single: true },
  keywords: [
    "if", "else", "endif", "do", "while", "enddo", "for", "endfor", "next",
    "case", "endcase", "otherwise", "with", "endwith", "scan", "endscan",
    "try", "catch", "finally", "endtry", "function", "procedure", "return",
    "parameters", "lparameters", "local", "public", "private", "define",
    "class", "endclass", "select", "use", "append", "replace", "delete",
    "pack", "index", "seek", "locate", "browse", "close", "set", "store",
    "release", "text", "endtext", "exit", "loop",
  ],
  constants: [".t.", ".f.", ".null."],
  extra: [
    // `*` in column 1 is the classic full-line comment.
    [/^\s*\*.*$/, "comment"],
    [/\.[tTfF]\./, "constant"],
  ],
};

const erlang: LanguageSpec = {
  id: "erlang",
  extensions: [".erl", ".hrl", ".escript"],
  aliases: ["Erlang"],
  comments: { line: ["%"] },
  strings: { double: true },
  keywords: [
    "after", "and", "andalso", "band", "begin", "bnot", "bor", "bsl", "bsr",
    "bxor", "case", "catch", "cond", "div", "end", "fun", "if", "let", "maybe",
    "not", "of", "or", "orelse", "receive", "rem", "try", "when", "xor",
  ],
  builtins: [
    "is_atom", "is_binary", "is_boolean", "is_float", "is_function",
    "is_integer", "is_list", "is_map", "is_number", "is_pid", "is_record",
    "is_tuple", "spawn", "spawn_link", "self", "node", "length", "hd", "tl",
    "element", "setelement", "tuple_size", "byte_size", "throw", "exit",
    "error", "apply",
  ],
  constants: ["true", "false", "undefined", "ok"],
  extra: [
    // Module attributes carry the file's structure: -module, -export, -spec.
    [/^-\s*[a-z]\w*/, "keyword.directive"],
    // Variables are Capitalised or `_`-led; everything lower-case is an atom.
    [/[A-Z_]\w*/, "variable"],
    // `$a` character literals and `'quoted atoms'`.
    [/\$(?:\\.|.)/, "string"],
    [/'(?:[^'\\]|\\.)*'/, "string.atom"],
    [/<<|>>/, "@brackets"],
    [/->|:-|\|\||=:=|=\/=|\/=|=</, "keyword.operator"],
  ],
};

const protobuf: LanguageSpec = {
  id: "protobuf",
  extensions: [".proto"],
  aliases: ["Protocol Buffers", "protobuf"],
  comments: { line: ["//"], block: ["/*", "*/"] },
  strings: { double: true, single: true },
  keywords: [
    "syntax", "edition", "package", "import", "public", "weak", "option",
    "message", "enum", "service", "rpc", "returns", "stream", "oneof", "map",
    "repeated", "optional", "required", "reserved", "to", "max", "extend",
    "extensions", "group",
  ],
  types: [
    "double", "float", "int32", "int64", "uint32", "uint64", "sint32",
    "sint64", "fixed32", "fixed64", "sfixed32", "sfixed64", "bool", "string",
    "bytes",
  ],
  constants: ["true", "false", "proto2", "proto3"],
};

const postscript: LanguageSpec = {
  id: "postscript",
  extensions: [".ps", ".eps", ".epsf"],
  aliases: ["PostScript", "EPS"],
  comments: { line: ["%"] },
  // PostScript strings are `(…)` with *nested* parentheses — not quotes at all.
  strings: { double: false },
  keywords: [
    "def", "begin", "end", "dup", "exch", "pop", "roll", "copy", "index",
    "if", "ifelse", "for", "forall", "repeat", "loop", "exit", "stop",
    "gsave", "grestore", "save", "restore", "showpage", "newpath", "moveto",
    "rmoveto", "lineto", "rlineto", "curveto", "closepath", "stroke", "fill",
    "clip", "translate", "scale", "rotate", "setlinewidth", "setlinecap",
    "setlinejoin", "setgray", "setrgbcolor", "setcmykcolor", "findfont",
    "scalefont", "setfont", "show", "stringwidth", "array", "dict",
    "currentpoint", "arc", "arcn", "eofill", "load", "bind", "exec",
  ],
  builtins: ["add", "sub", "mul", "div", "idiv", "mod", "neg", "abs", "sqrt",
    "sin", "cos", "atan", "exp", "ln", "log", "truncate", "round", "ceiling",
    "floor", "eq", "ne", "gt", "ge", "lt", "le", "and", "or", "not", "xor"],
  extra: [
    [/\(/, { token: "string.quote", next: "@psString" }],
    [/\/[^\s()<>[\]{}/%]+/, "string.escape"], // /literal names
    [/<<|>>/, "@brackets"],
    [/<[0-9a-fA-F\s]*>/, "string"], // hex strings
    [/\d+#[0-9a-zA-Z]+/, "number"], // radix numbers, e.g. 16#FF
  ],
};

const POSTSCRIPT_EXTRA_STATES = {
  psString: [
    [/[^()\\]+/, "string"],
    [/\\./, "string.escape"],
    // Parentheses nest inside PostScript strings; a flat rule would end the
    // string at the first inner `)` and mis-colour the rest of the file.
    [/\(/, "string", "@push"],
    [/\)/, { token: "string.quote", next: "@pop" }],
  ],
} as const;

const astro: LanguageSpec = {
  id: "astro",
  extensions: [".astro"],
  aliases: ["Astro"],
  comments: { line: ["//"], block: ["/*", "*/"] },
  strings: { double: true, single: true, backtick: true },
  keywords: [
    "import", "export", "from", "as", "default", "const", "let", "var",
    "function", "return", "if", "else", "for", "of", "in", "await", "async",
    "class", "extends", "new", "typeof", "interface", "type",
  ],
  constants: ["true", "false", "null", "undefined"],
  extra: [
    // Component frontmatter fence.
    [/^---\s*$/, "keyword.control"],
    // Markup tags, so the template half is not one grey block.
    [/<\/?[a-zA-Z][\w.-]*/, "tag"],
    [/\/?>/, "tag"],
  ],
};

/** Every language we supply a grammar for. */
export const MONARCH_LANGUAGES: readonly LanguageSpec[] = [
  // Modern systems
  zig, nim, crystal, vlang, dlang, vala, odin, gleam,
  // Functional
  haskell, elm, purescript, rescript, erlang,
  // Infrastructure / build
  nix, cmake, protobuf,
  // Typesetting
  latex, postscript,
  // Scientific / engineering
  matlab, assembly,
  // Enterprise / legacy
  cobol, sas, ada, fortran, prolog, vhdl, foxpro,
  // Web
  astro,
];

/**
 * States that cannot be expressed as `extra` rules alone, keyed by language id.
 * Merged into the generated tokenizer at registration.
 */
export const EXTRA_STATES: Readonly<Record<string, Record<string, unknown>>> = {
  nix: NIX_EXTRA_STATES,
  latex: LATEX_EXTRA_STATES,
  postscript: POSTSCRIPT_EXTRA_STATES,
  cmake: CMAKE_EXTRA_STATES,
};
