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

        // Swift / Objective-C
        'swift': 'swift', 'm': 'objective-c', 'mm': 'objective-c',

        // Dart
        'dart': 'dart',

        // Elixir / Erlang
        'ex': 'elixir', 'exs': 'elixir', 'erl': 'elixir', 'hrl': 'elixir',

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

        // Pascal
        'pas': 'pascal', 'pp': 'pascal',

        // Ada
        'adb': 'pascal',            // Ada → Pascal syntax (similar)
        'ads': 'pascal',

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
import type { LucideIcon } from "lucide-react";
import {
    Folder, FileCode, Atom, Cog, Coffee, Gem, Globe,
    Palette, Braces, Settings, FileText, File, BookOpen,
    Image, Paintbrush, Archive,
} from "lucide-react";

const ICON_SIZE = 16;
const ICON_STROKE = 1.5;

const FILE_ICONS: Record<string, LucideIcon> = {
    js: FileCode, ts: FileCode, jsx: Atom, tsx: Atom,
    rs: Cog, py: FileCode, go: FileCode, java: Coffee,
    rb: Gem, php: FileCode,
    html: Globe, css: Palette, scss: Palette,
    json: Braces, yaml: Settings, yml: Settings, toml: Settings, xml: FileCode,
    md: FileText, txt: File, pdf: BookOpen,
    png: Image, jpg: Image, jpeg: Image, gif: Image, svg: Paintbrush,
    zip: Archive, tar: Archive, gz: Archive,
};

/**
 * Get file icon based on type
 */
export function getFileIcon(filename: string, isDirectory: boolean): React.ReactElement {
    if (isDirectory) return <Folder size={ICON_SIZE} strokeWidth={ICON_STROKE} />;
    const ext = filename.split('.').pop()?.toLowerCase() || '';
    const Icon = FILE_ICONS[ext] || File;
    return <Icon size={ICON_SIZE} strokeWidth={ICON_STROKE} />;
}
