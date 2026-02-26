import { useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useToast } from '../hooks/useToast';
import { Toaster } from './Toaster';

const QUICK_LAUNCH = [
    { label: 'localhost:3000', url: 'http://localhost:3000' },
    { label: 'localhost:5173', url: 'http://localhost:5173' },
    { label: 'localhost:8080', url: 'http://localhost:8080' },
    { label: 'localhost:4000', url: 'http://localhost:4000' },
];

export function BrowserPanel() {
    const { toasts, toast, dismiss } = useToast();
    const [urlInput, setUrlInput] = useState('http://localhost:3000');
    const [iframeSrc, setIframeSrc] = useState('');
    const [history, setHistory] = useState<string[]>([]);
    const [histIdx, setHistIdx] = useState(-1);
    const iframeRef = useRef<HTMLIFrameElement>(null);

    const navigate = (url: string) => {
        const target = url.startsWith('http') ? url : `http://${url}`;
        setUrlInput(target);
        setIframeSrc(target);
        setHistory((prev) => {
            const trimmed = prev.slice(0, histIdx + 1);
            const next = [...trimmed, target];
            setHistIdx(next.length - 1);
            return next;
        });
    };

    const goBack = () => {
        if (histIdx > 0) {
            const newIdx = histIdx - 1;
            setHistIdx(newIdx);
            setUrlInput(history[newIdx]);
            setIframeSrc(history[newIdx]);
        }
    };

    const goForward = () => {
        if (histIdx < history.length - 1) {
            const newIdx = histIdx + 1;
            setHistIdx(newIdx);
            setUrlInput(history[newIdx]);
            setIframeSrc(history[newIdx]);
        }
    };

    const refresh = () => {
        if (iframeSrc) {
            // Force reload by temporarily clearing then restoring
            setIframeSrc('');
            setTimeout(() => setIframeSrc(iframeSrc), 50);
        }
    };

    const openExternal = async () => {
        if (!urlInput) return;
        try {
            await invoke('open_external_url', { url: urlInput });
        } catch (e) {
            toast.error(`Failed to open external URL: ${e}`);
        }
    };

    return (
        <div style={{ display: 'flex', flexDirection: 'column', height: '100%', background: 'var(--bg-primary)' }}>
            {/* Toolbar */}
            <div style={{
                display: 'flex', alignItems: 'center', gap: '4px',
                padding: '4px 8px', borderBottom: '1px solid var(--border-color)',
                background: 'var(--bg-secondary)', flexShrink: 0,
            }}>
                <button
                    onClick={goBack}
                    disabled={histIdx <= 0}
                    title="Back"
                    style={navBtnStyle}
                >←</button>
                <button
                    onClick={goForward}
                    disabled={histIdx >= history.length - 1}
                    title="Forward"
                    style={navBtnStyle}
                >→</button>
                <button onClick={refresh} disabled={!iframeSrc} title="Refresh" style={navBtnStyle}>↻</button>

                <div style={{ width: '1px', height: '16px', background: 'var(--border-color)', margin: '0 4px' }} />

                <input
                    type="text"
                    value={urlInput}
                    onChange={(e) => setUrlInput(e.target.value)}
                    onKeyDown={(e) => { if (e.key === 'Enter') navigate(urlInput); }}
                    placeholder="Enter URL…"
                    style={{
                        flex: 1, padding: '3px 8px', fontSize: '12px',
                        background: 'var(--bg-tertiary)', border: '1px solid var(--border-color)',
                        color: 'var(--text-primary)', borderRadius: '4px', outline: 'none',
                    }}
                />
                <button
                    onClick={() => navigate(urlInput)}
                    style={{ ...navBtnStyle, padding: '3px 10px', fontSize: '12px' }}
                >Go</button>
                <button
                    onClick={openExternal}
                    title="Open in system browser"
                    style={{ ...navBtnStyle, fontSize: '11px', padding: '3px 8px' }}
                >↗</button>
            </div>

            {/* Quick-launch chips */}
            <div style={{
                display: 'flex', gap: '6px', padding: '4px 8px',
                borderBottom: '1px solid var(--border-color)',
                background: 'var(--bg-secondary)', flexShrink: 0,
            }}>
                {QUICK_LAUNCH.map(({ label, url }) => (
                    <button
                        key={url}
                        onClick={() => navigate(url)}
                        style={{
                            padding: '2px 8px', fontSize: '11px',
                            background: iframeSrc === url ? 'var(--accent-blue)' : 'var(--bg-tertiary)',
                            color: iframeSrc === url ? '#fff' : 'var(--text-secondary)',
                            border: '1px solid var(--border-color)', borderRadius: '10px',
                            cursor: 'pointer',
                        }}
                    >{label}</button>
                ))}
            </div>

            {/* Webview / iframe */}
            <div style={{ flex: 1, position: 'relative', overflow: 'hidden' }}>
                {iframeSrc ? (
                    <iframe
                        ref={iframeRef}
                        src={iframeSrc}
                        sandbox="allow-scripts allow-same-origin allow-forms allow-modals"
                        style={{ width: '100%', height: '100%', border: 'none' }}
                        title="Browser preview"
                    />
                ) : (
                    <div style={{
                        display: 'flex', alignItems: 'center', justifyContent: 'center',
                        height: '100%', flexDirection: 'column', gap: '12px',
                        color: 'var(--text-secondary)', fontSize: '13px',
                    }}>
                        <div style={{ fontSize: '32px' }}>🌐</div>
                        <div>Enter a URL or click a quick-launch chip to preview</div>
                    </div>
                )}
            </div>
            <Toaster toasts={toasts} onDismiss={dismiss} />
        </div>
    );
}

const navBtnStyle: React.CSSProperties = {
    background: 'none',
    border: '1px solid var(--border-color)',
    color: 'var(--text-primary)',
    borderRadius: '4px',
    cursor: 'pointer',
    padding: '3px 7px',
    fontSize: '14px',
    lineHeight: 1,
};
