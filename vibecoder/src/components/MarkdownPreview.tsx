/* eslint-disable @typescript-eslint/no-explicit-any */
import ReactMarkdown from 'react-markdown';
import React from 'react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { htmlToMarkdown } from '../lib/markdownHtml';
import { classifyMarkdownLink, slugifyHeading } from '../lib/markdownLinks';
import { MarkdownWithDetails } from './MarkdownDetails';
import './MarkdownPreview.css';
import { DiagramView } from './DiagramView';
import { diagramKindForFence, type DiagramKind } from '../lib/diagrams';

interface MarkdownPreviewProps {
    content: string;
    /**
     * Whether the preview owns its own scrolling.
     *
     * It does by default, which is right when it fills a pane on its own. A
     * caller that lays the text out itself — the document viewer's two-column
     * spread — needs the opposite: a box with `overflow` set is a single
     * unbreakable fragment, so the columns cannot split it and the whole
     * document ends up in the first one.
     */
    scrolls?: boolean;
    /**
     * Absolute path of the file this markdown came from. Relative links resolve
     * against its directory, so `docs/README.md` means the same thing here as it
     * does on GitHub. Without it, a relative link has no destination to compute.
     */
    basePath?: string | null;
    /**
     * Open another workspace file. A preview with no opener leaves local links
     * inert — it says so in the console rather than pretending the click worked.
     */
    onOpenFile?: (path: string, fragment: string | null) => void;
    /** A heading to scroll to once this content renders: how a link's `#fragment` survives the file change. */
    focusFragment?: string | null;
}

interface LinkHandling {
    basePath?: string | null;
    onOpenFile?: (path: string, fragment: string | null) => void;
}

/**
 * The component maps below are module-level constants on purpose: react-markdown
 * re-parses when the `components` identity changes and this preview re-renders on
 * every keystroke. Link handling reaches them through context rather than through
 * a map rebuilt per render.
 */
const LinkHandlingContext = React.createContext<LinkHandling>({});

function decodeSafely(value: string): string {
    try {
        return decodeURIComponent(value);
    } catch {
        return value;
    }
}

/** The rendered text of a node — a heading's slug is computed from what it reads as. */
function textOf(node: React.ReactNode): string {
    if (node === null || node === undefined || typeof node === 'boolean') return '';
    if (typeof node === 'string' || typeof node === 'number') return String(node);
    if (Array.isArray(node)) return node.map(textOf).join('');
    if (React.isValidElement(node)) return textOf((node.props as any).children);
    return '';
}

/** The element a `#fragment` names: an exact id, else the heading whose slug matches. */
function findAnchorTarget(container: HTMLElement, fragment: string): HTMLElement | null {
    const raw = decodeSafely(fragment);
    const wanted = slugifyHeading(raw);
    const candidates = Array.from(container.querySelectorAll<HTMLElement>('[id]'));
    return candidates.find((el) => el.id === raw || el.id === wanted) ?? null;
}

function MarkdownLink({ node: _node, href, children, ...props }: any) {
    const { basePath, onOpenFile } = React.useContext(LinkHandlingContext);
    const target = typeof href === 'string' ? classifyMarkdownLink(href, basePath) : null;

    const follow = (e: React.MouseEvent<HTMLAnchorElement>) => {
        e.preventDefault();
        e.stopPropagation();
        if (!target) return;

        switch (target.kind) {
            case 'external':
                openUrl(target.url).catch(console.error);
                return;
            case 'anchor': {
                const container = e.currentTarget.closest('.markdown-preview') as HTMLElement | null;
                const found = container ? findAnchorTarget(container, target.fragment) : null;
                if (found) found.scrollIntoView?.({ behavior: 'smooth', block: 'start' });
                else console.warn('No heading in this document matches anchor:', target.fragment);
                return;
            }
            case 'file':
                if (!onOpenFile) {
                    console.warn('This preview has no file opener; link not followed:', target.path);
                    return;
                }
                onOpenFile(target.path, target.fragment);
                return;
            default: {
                const exhaustive: never = target;
                return exhaustive;
            }
        }
    };

    return (
        <a
            {...props}
            data-href={href}
            data-link-kind={target?.kind}
            title={target?.kind === 'file' ? target.path : props.title}
            style={{ cursor: 'pointer', ...props.style }}
            onClick={follow}
        >
            {children}
        </a>
    );
}

// Reusable component overrides: link handling that works for every surface.
const sharedComponents: any = {
    p: React.Fragment,
    a: MarkdownLink,
};

// Headings carry the id their own `#fragment` names, so an in-document link
// has something to find.
const headingComponents: any = Object.fromEntries(
    ([1, 2, 3, 4, 5, 6] as const).map((level) => {
        const tag = `h${level}`;
        const Heading = ({ node: _node, children, ...props }: any) =>
            React.createElement(
                tag,
                { id: slugifyHeading(textOf(children)) || undefined, ...props },
                children
            );
        Heading.displayName = `Markdown${tag.toUpperCase()}`;
        return [tag, Heading];
    })
);

// Frontmatter belongs to the document, not to a block within it: run this once
// on the whole source, before it is split, or a disclosure whose body opens
// with a `---` rule would lose everything up to the next one.
function stripFrontmatter(markdown: string): string {
    if (!markdown.startsWith('---\n') && !markdown.startsWith('---\r\n')) return markdown;
    const endFrontmatterIndex = markdown.indexOf('\n---', 3);
    if (endFrontmatterIndex === -1) return markdown;
    return markdown.substring(endFrontmatterIndex + 4).trimStart();
}

// Minimal pre-processor to parse tables since remark-gfm requires internet, and
// to rewrite the raw HTML that would otherwise reach the reader as literal tags.
function preprocessMarkdown(markdown: string): string {
    const content = htmlToMarkdown(markdown);

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

/**
 * The diagram inside a fenced block, if the fence opened one.
 *
 * ```` ```mermaid ```` and ```` ```plantuml ```` are diagrams people write in
 * documents, and a document that shows them as source is showing the thing
 * rather than the picture of it. Read off the `<pre>` rather than the `<code>`
 * so the diagram replaces the whole block: a `<div>` inside a `<pre>` is not
 * markup any browser should be asked to make sense of.
 */
function diagramInPre(children: any): { kind: DiagramKind; source: string } | null {
    const only = React.Children.toArray(children)[0] as any;
    const className: string = only?.props?.className ?? '';
    const language = /language-([\w-]+)/.exec(className)?.[1];
    const kind = diagramKindForFence(language);
    if (!kind) return null;
    return { kind, source: nodeText(only?.props?.children).replace(/\n$/, '') };
}

/** The text of a code block, whatever depth react-markdown nested it at. */
function nodeText(node: any): string {
    if (node == null || node === false) return '';
    if (typeof node === 'string' || typeof node === 'number') return String(node);
    if (Array.isArray(node)) return node.map(nodeText).join('');
    return nodeText(node?.props?.children);
}

const bodyComponents: any = {
    ...sharedComponents,
    ...headingComponents,
    p: 'p', // Restore standard paragraph wrapping for the main document body!
    pre({ children, ...props }: any) {
        const diagram = diagramInPre(children);
        if (!diagram) return <pre {...props}>{children}</pre>;
        return <DiagramView kind={diagram.kind} source={diagram.source} />;
    },
    code({ node: _node, inline, className, children, ...props }: any) {
        const match = /language-(\w+)/.exec(className || '');
        if (!inline && match && match[1] === '__markdown_table__') {
            return renderTable(String(children).replace(/\n$/, ''));
        }
        return <code className={className} {...props}>{children}</code>;
    }
};

const renderBlock = (markdown: string) => (
    <ReactMarkdown components={bodyComponents}>{preprocessMarkdown(markdown)}</ReactMarkdown>
);

// A <summary> is one line: paragraphs are dropped to fragments so the label
// sits next to the disclosure triangle instead of below it.
const renderSummary = (markdown: string) => (
    <ReactMarkdown components={sharedComponents}>{htmlToMarkdown(markdown)}</ReactMarkdown>
);

export function MarkdownPreview({
    content,
    basePath,
    onOpenFile,
    focusFragment,
    scrolls = true,
}: MarkdownPreviewProps) {
    const containerRef = React.useRef<HTMLDivElement>(null);
    const handling = React.useMemo(() => ({ basePath, onOpenFile }), [basePath, onOpenFile]);

    // A fragment that arrived with a file change: the heading it names only
    // exists once this content has rendered, so the scroll waits for it here.
    React.useEffect(() => {
        if (!focusFragment || !containerRef.current) return;
        const target = findAnchorTarget(containerRef.current, focusFragment);
        target?.scrollIntoView?.({ behavior: 'auto', block: 'start' });
    }, [focusFragment, content]);

    return (
        <LinkHandlingContext.Provider value={handling}>
            <div
                ref={containerRef}
                style={{
                    padding: '20px',
                    height: scrolls ? '100%' : 'auto',
                    overflowY: scrolls ? 'auto' : 'visible',
                    background: 'var(--bg-primary)',
                    color: 'var(--text-primary)',
                }}
                className="markdown-preview"
            >
                <MarkdownWithDetails
                    source={stripFrontmatter(content)}
                    renderBlock={renderBlock}
                    renderInline={renderSummary}
                />
            </div>
        </LinkHandlingContext.Provider>
    );
}
