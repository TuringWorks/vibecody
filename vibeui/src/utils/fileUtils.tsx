/**
 * Detect programming language from file extension
 */
export function detectLanguage(filename: string): string {
    const ext = filename.split('.').pop()?.toLowerCase();

    const languageMap: Record<string, string> = {
        // JavaScript/TypeScript
        'js': 'javascript',
        'jsx': 'javascript',
        'ts': 'typescript',
        'tsx': 'typescript',
        'mjs': 'javascript',
        'cjs': 'javascript',

        // Web
        'html': 'html',
        'htm': 'html',
        'css': 'css',
        'scss': 'scss',
        'sass': 'sass',
        'less': 'less',

        // Rust
        'rs': 'rust',
        'toml': 'toml',

        // Python
        'py': 'python',
        'pyw': 'python',

        // Go
        'go': 'go',

        // C/C++
        'c': 'c',
        'cpp': 'cpp',
        'cc': 'cpp',
        'cxx': 'cpp',
        'h': 'c',
        'hpp': 'cpp',

        // Java/Kotlin
        'java': 'java',
        'kt': 'kotlin',
        'kts': 'kotlin',

        // C#
        'cs': 'csharp',

        // PHP
        'php': 'php',

        // Ruby
        'rb': 'ruby',

        // Shell
        'sh': 'shell',
        'bash': 'shell',
        'zsh': 'shell',

        // Config/Data
        'json': 'json',
        'yaml': 'yaml',
        'yml': 'yaml',
        'xml': 'xml',
        'md': 'markdown',
        'markdown': 'markdown',
        'txt': 'plaintext',

        // SQL
        'sql': 'sql',

        // Docker
        'dockerfile': 'dockerfile',

        // Others
        'swift': 'swift',
        'r': 'r',
        'lua': 'lua',
        'vim': 'vim',
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
