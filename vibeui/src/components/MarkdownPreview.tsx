import ReactMarkdown from 'react-markdown';

interface MarkdownPreviewProps {
    content: string;
}

export function MarkdownPreview({ content }: MarkdownPreviewProps) {
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
            <ReactMarkdown>{content}</ReactMarkdown>
        </div>
    );
}
