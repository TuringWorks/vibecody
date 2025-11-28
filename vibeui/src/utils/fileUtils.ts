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

/**
 * Get file icon based on type
 */
export function getFileIcon(filename: string, isDirectory: boolean): string {
    if (isDirectory) return '📁';

    const ext = filename.split('.').pop()?.toLowerCase();

    const iconMap: Record<string, string> = {
        // Code
        'js': '📜',
        'ts': '📘',
        'jsx': '⚛️',
        'tsx': '⚛️',
        'rs': '🦀',
        'py': '🐍',
        'go': '🐹',
        'java': '☕',
        'rb': '💎',
        'php': '🐘',

        // Web
        'html': '🌐',
        'css': '🎨',
        'scss': '🎨',

        // Config
        'json': '📋',
        'yaml': '⚙️',
        'yml': '⚙️',
        'toml': '⚙️',
        'xml': '📋',

        // Docs
        'md': '📝',
        'txt': '📄',
        'pdf': '📕',

        // Images
        'png': '🖼️',
        'jpg': '🖼️',
        'jpeg': '🖼️',
        'gif': '🖼️',
        'svg': '🎨',

        // Others
        'zip': '📦',
        'tar': '📦',
        'gz': '📦',
    };

    return iconMap[ext || ''] || '📄';
}
