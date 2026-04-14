/**
 * Detect programming language from file extension
 */
export function detectLanguage(filename: string): string {
    const ext = filename.split('.').pop()?.toLowerCase();

    const languageMap: Record<string, string> = {
        // JavaScript/TypeScript
        'js': 'javascript', 'jsx': 'javascript', 'ts': 'typescript', 'tsx': 'typescript',
        'mjs': 'javascript', 'cjs': 'javascript',

        // Web
        'html': 'html', 'htm': 'html', 'css': 'css', 'scss': 'scss', 'sass': 'scss', 'less': 'less',
        'vue': 'html', 'svelte': 'html',

        // Systems
        'rs': 'rust', 'c': 'c', 'cpp': 'cpp', 'cc': 'cpp', 'cxx': 'cpp', 'h': 'c', 'hpp': 'cpp',
        'go': 'go', 'zig': 'cpp',  // Zig → C++ syntax (closest match)
        'd': 'cpp',                 // D → C++ syntax
        'v': 'cpp',                 // V (vlang) → C++ syntax
        'vala': 'csharp',           // Vala → C# syntax (similar)

        // JVM
        'java': 'java', 'kt': 'kotlin', 'kts': 'kotlin', 'scala': 'scala',
        'groovy': 'java',           // Groovy → Java syntax
        'clj': 'clojure', 'cljs': 'clojure', 'cljc': 'clojure', 'edn': 'clojure',

        // .NET
        'cs': 'csharp', 'fs': 'fsharp', 'fsx': 'fsharp', 'vb': 'vb',

        // Python
        'py': 'python', 'pyw': 'python', 'pyi': 'python',

        // Ruby
        'rb': 'ruby', 'erb': 'ruby', 'rake': 'ruby', 'gemspec': 'ruby',

        // PHP
        'php': 'php',

        // Swift / Objective-C (.mm is unambiguously ObjC; bare .m goes to MATLAB below)
        'swift': 'swift', 'mm': 'objective-c',

        // Dart
        'dart': 'dart',

        // Elixir / Erlang (distinct languages sharing BEAM VM — use separate Monaco IDs)
        'ex': 'elixir', 'exs': 'elixir', 'erl': 'erlang', 'hrl': 'erlang',

        // Haskell
        'hs': 'haskell',            // Monaco doesn't have Haskell; map to plaintext with custom later
        'lhs': 'haskell',

        // Functional
        'ml': 'fsharp',             // OCaml → F# syntax (ML family)
        'mli': 'fsharp',
        'rkt': 'scheme',            // Racket → Scheme
        'scm': 'scheme',
        'lisp': 'scheme', 'cl': 'scheme', 'el': 'scheme',

        // Crystal / Nim
        'cr': 'ruby',               // Crystal → Ruby syntax (very similar)
        'nim': 'python',            // Nim → Python syntax (similar indentation)
        'nims': 'python',

        // Perl
        'pl': 'perl', 'pm': 'perl', 'pod': 'perl',

        // Lua
        'lua': 'lua',

        // R / Julia
        'r': 'r', 'R': 'r',
        'jl': 'julia',

        // Shell
        'sh': 'shell', 'bash': 'shell', 'zsh': 'shell', 'fish': 'shell',
        'ps1': 'powershell', 'psm1': 'powershell',

        // Fortran
        'f': 'fortran', 'f90': 'fortran', 'f95': 'fortran', 'f03': 'fortran', 'f08': 'fortran',
        'ftn': 'fortran', 'for': 'fortran',

        // Pascal
        'pas': 'pascal', 'pp': 'pascal',

        // Ada
        'adb': 'ada', 'ads': 'ada', 'ada': 'ada',

        // MATLAB (.m wins over Objective-C for bare .m files)
        'm': 'matlab', 'mat': 'matlab', 'mlx': 'matlab', 'mlapp': 'matlab',

        // Assembly
        'asm': 'asm', 's': 'asm', 'nasm': 'asm', 'S': 'asm',

        // COBOL
        'cob': 'cobol', 'cbl': 'cobol', 'cpy': 'cobol',

        // SAS
        'sas': 'sas',

        // VBScript
        'vbs': 'vb', 'wsf': 'vb',

        // ABAP
        'abap': 'abap',

        // Solidity
        'sol': 'sol',

        // Transact-SQL
        'tsql': 'sql',

        // PL/SQL
        'pls': 'sql', 'plsql': 'sql', 'pkb': 'sql', 'pks': 'sql',

        // FoxPro
        'prg': 'foxpro',

        // GML (GameMaker Language — JS-like syntax)
        'gml': 'javascript',

        // X++
        'xpp': 'csharp', 'axpp': 'csharp',

        // Prolog
        'pro': 'prolog',

        // Config / Data
        'json': 'json', 'jsonc': 'json', 'json5': 'json',
        'yaml': 'yaml', 'yml': 'yaml',
        'xml': 'xml', 'svg': 'xml', 'xsl': 'xml', 'xsd': 'xml',
        'toml': 'ini',              // TOML → INI (closest in Monaco)
        'ini': 'ini', 'cfg': 'ini', 'conf': 'ini',
        'env': 'ini',

        // Markup
        'md': 'markdown', 'markdown': 'markdown', 'mdx': 'markdown',
        'rst': 'plaintext', 'tex': 'plaintext', 'latex': 'plaintext',
        'txt': 'plaintext',
        'ps': 'postscript', 'eps': 'postscript',  // PostScript files (text-based)

        // SQL
        'sql': 'sql', 'mysql': 'mysql', 'pgsql': 'pgsql',

        // Docker / Infra
        'dockerfile': 'dockerfile',
        'tf': 'hcl', 'tfvars': 'hcl',
        'bicep': 'bicep',

        // GraphQL / Protobuf
        'graphql': 'graphql', 'gql': 'graphql',
        'proto': 'protobuf',

        // Other
        'vim': 'plaintext',
        'makefile': 'shell', 'cmake': 'plaintext',
        'gradle': 'java',
        'lock': 'json',
    };

    return languageMap[ext || ''] || 'plaintext';
}

/**
 * Format file size for display
 */
export function formatFileSize(bytes: number): string {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
    if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
    return (bytes / (1024 * 1024 * 1024)).toFixed(1) + ' GB';
}

import React from "react";
import { Icon, type IconName } from "../components/Icon";

const ICON_SIZE = 16;

const FILE_ICONS: Record<string, IconName> = {
    js: "file-code", ts: "file-code", jsx: "atom", tsx: "atom",
    rs: "cog", py: "file-code", go: "file-code", java: "coffee",
    rb: "gem", php: "file-code",
    html: "globe", css: "palette", scss: "palette",
    json: "braces", yaml: "settings", yml: "settings", toml: "settings", xml: "file-code",
    md: "file-text", txt: "file", pdf: "book-open", epub: "book-open", ps: "file-text", eps: "file-text",
    png: "image-file", jpg: "image-file", jpeg: "image-file", gif: "image-file", svg: "paintbrush",
    zip: "archive", tar: "archive", gz: "archive",
};

/**
 * Get file icon based on type
 */
export function getFileIcon(filename: string, isDirectory: boolean): React.ReactElement {
    if (isDirectory) return <Icon name="folder" size={ICON_SIZE} />;
    const ext = filename.split('.').pop()?.toLowerCase() || '';
    const iconName: IconName = FILE_ICONS[ext] ?? "file";
    return <Icon name={iconName} size={ICON_SIZE} />;
}
