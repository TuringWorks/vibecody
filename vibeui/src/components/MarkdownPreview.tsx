/* eslint-disable @typescript-eslint/no-explicit-any */
import ReactMarkdown from 'react-markdown';
import React from 'react';
import { openUrl } from '@tauri-apps/plugin-opener';
import './MarkdownPreview.css';

interface MarkdownPreviewProps {
    content: string;
}

// Reusable component override for safely opening external links
const sharedComponents: any = {
    p: React.Fragment,
    a({ href, children, ...props }: any) {
        return (
            <a
                {...props}
                data-href={href}
                style={{ cursor: 'pointer', ...props.style }}
                onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    if (!href) return;

                    let targetUrl = href;
                    // Auto-fix schema-less domains (e.g., github.com/...)
                    if (!href.startsWith('http') && !href.startsWith('mailto:') && !href.startsWith('#') && !href.startsWith('/')) {
                       const firstSlash = href.indexOf('/');
                       const firstDot = href.indexOf('.');
                       if (firstDot !== -1 && (firstSlash === -1 || firstDot < firstSlash)) {
                           const endIdx = firstSlash !== -1 ? firstSlash : href.length;
                           const potentialTld = href.substring(firstDot + 1, endIdx).toLowerCase();
                           
                           // Only auto-upgrade if the part after the dot is a common top-level web domain, 
                           // preventing local files like CHANGELOG.md from being parsed as domains.
                           const commonTlds = ['com', 'org', 'net', 'io', 'co', 'dev', 'ai', 'app', 'xyz', 'tech', 'gov', 'edu'];
                           if (commonTlds.includes(potentialTld)) {
                               targetUrl = `https://${href}`;
                           }
                       }
                    }

                    if (targetUrl.startsWith('http') || targetUrl.startsWith('mailto:')) {
                        openUrl(targetUrl).catch(console.error);
                    } else {
                        console.warn('Local markdown link clicked internally, navigation deferred:', targetUrl);
                    }
                }}
            >
                {children}
            </a>
        );
    }
};

// Minimal pre-processor to strip frontmatter and parse tables since remark-gfm requires internet
function preprocessMarkdown(markdown: string): string {
    let content = markdown;
    
    // Strip YAML frontmatter at the very beginning of the document
    if (content.startsWith('---\n') || content.startsWith('---\r\n')) {
        // Look for the next closing '---' line
        const endFrontmatterIndex = content.indexOf('\n---', 3);
        if (endFrontmatterIndex !== -1) {
            // Strip it out, including the closing --- and following newline
            content = content.substring(endFrontmatterIndex + 4).trimStart();
        }
    }

    const lines = content.split('\n');
    const out: string[] = [];
    let inTable = false;
    let tableLines: string[] = [];

    const isTableRow = (line: string) => line.trim().includes('|');
    const isDividerRow = (line: string) => /^[\s|:-]+$/.test(line) && line.includes('|') && line.includes('-');

    for (let i = 0; i < lines.length; i++) {
        const line = lines[i];
        
        if (!inTable) {
            // Check if current line might be a header and next line is a divider
            if (isTableRow(line) && i + 1 < lines.length && isDividerRow(lines[i + 1])) {
                inTable = true;
                tableLines.push(line);
            } else {
                out.push(line);
            }
        } else {
            if (isTableRow(line)) {
                tableLines.push(line);
            } else {
                // End of table
                out.push('```__markdown_table__');
                out.push(tableLines.join('\n'));
                out.push('```');
                inTable = false;
                tableLines = [];
                out.push(line);
            }
        }
    }
    
    if (inTable) {
        out.push('```__markdown_table__');
        out.push(tableLines.join('\n'));
        out.push('```');
    }
    
    return out.join('\n');
}

function parseRow(line: string) {
    let trimmed = line.trim();
    if (trimmed.startsWith('|')) trimmed = trimmed.substring(1);
    if (trimmed.endsWith('|')) trimmed = trimmed.substring(0, trimmed.length - 1);
    return trimmed.split('|').map(s => s.trim());
}

function renderTable(tableText: string) {
    const lines = tableText.trim().split('\n');
    if (lines.length < 2) return <pre>{tableText}</pre>;
    
    const parsedHeaders = parseRow(lines[0]);
    const parsedRows = lines.slice(2).map(parseRow);

    return (
        <div className="markdown-table-wrapper">
            <table>
                <thead>
                    <tr>
                        {parsedHeaders.map((h, i) => (
                            <th key={i}>
                                <ReactMarkdown components={sharedComponents}>
                                    {h}
                                </ReactMarkdown>
                            </th>
                        ))}
                    </tr>
                </thead>
                <tbody>
                    {parsedRows.map((row, i) => (
                        <tr key={i}>
                            {row.map((cell, j) => (
                                <td key={j}>
                                    <ReactMarkdown components={sharedComponents}>
                                        {cell}
                                    </ReactMarkdown>
                                </td>
                            ))}
                        </tr>
                    ))}
                </tbody>
            </table>
        </div>
    );
}

export function MarkdownPreview({ content }: MarkdownPreviewProps) {
    const processedContent = preprocessMarkdown(content);

    return (
        <div
            style={{
                padding: '20px',
                height: '100%',
                overflowY: 'auto',
                background: 'var(--bg-primary)',
                color: 'var(--text-primary)',
            }}
            className="markdown-preview"
        >
            <ReactMarkdown
                components={{
                    ...sharedComponents,
                    p: 'p', // Restore standard paragraph wrapping for the main document body!
                    code({ node: _node, inline, className, children, ...props }: any) {
                        const match = /language-(\w+)/.exec(className || '');
                        if (!inline && match && match[1] === '__markdown_table__') {
                            return renderTable(String(children).replace(/\n$/, ''));
                        }
                        return <code className={className} {...props}>{children}</code>;
                    }
                }}
            >
                {processedContent}
            </ReactMarkdown>
        </div>
    );
}
