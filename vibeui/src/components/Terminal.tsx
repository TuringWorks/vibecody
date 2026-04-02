import { useEffect, useRef } from 'react';
import { Terminal as XTerm } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import 'xterm/css/xterm.css';

interface TerminalProps {
    onClose?: () => void;
}

export function Terminal({ onClose }: TerminalProps) {
    const terminalRef = useRef<HTMLDivElement>(null);
    const xtermRef = useRef<XTerm | null>(null);
    const fitAddonRef = useRef<FitAddon | null>(null);
    const terminalIdRef = useRef<number | null>(null);

    useEffect(() => {
        if (!terminalRef.current) return;

        // Resolve CSS variable to a hex color for xterm.js canvas
        const computedBg = getComputedStyle(document.documentElement)
            .getPropertyValue('--bg-primary').trim() || '#1a1a2e';

        const term = new XTerm({
            cursorBlink: true,
            theme: {
                background: computedBg,
                foreground: '#d4d4d4',
                cursor: '#4cc9f0',
                cursorAccent: computedBg,
                selectionBackground: 'rgba(76, 201, 240, 0.3)',
                // ANSI colors — vibrant palette for ls, git, etc.
                black:         '#1a1a2e',
                red:           '#f72585',
                green:         '#7ae582',
                yellow:        '#ffd166',
                blue:          '#4cc9f0',
                magenta:       '#b388ff',
                cyan:          '#64dfdf',
                white:         '#d4d4d4',
                brightBlack:   '#555577',
                brightRed:     '#ff5ca1',
                brightGreen:   '#a8f0b0',
                brightYellow:  '#ffe599',
                brightBlue:    '#7dd8f7',
                brightMagenta: '#d4b4ff',
                brightCyan:    '#96efef',
                brightWhite:   '#ffffff',
            },
            fontFamily: '"SF Mono", "Fira Code", "Cascadia Code", monospace',
            fontSize: 14,
        });

        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);

        term.open(terminalRef.current);
        fitAddon.fit();

        xtermRef.current = term;
        fitAddonRef.current = fitAddon;

        let disposed = false;
        let unlisten: (() => void) | null = null;

        const handleResize = () => {
            if (disposed) return;
            try {
                fitAddon.fit();
            } catch { /* container might be hidden */ }
            if (terminalIdRef.current !== null) {
                invoke('resize_terminal', {
                    id: terminalIdRef.current,
                    rows: term.rows,
                    cols: term.cols,
                });
            }
        };

        // Use ResizeObserver to detect container size changes (panel drag,
        // window resize, maximize/restore) — not just window.resize.
        const resizeObserver = new ResizeObserver(() => handleResize());
        resizeObserver.observe(terminalRef.current);

        // Spawn terminal backend
        const initTerminal = async () => {
            try {
                const id = await invoke<number>('spawn_terminal');
                if (disposed) return;
                terminalIdRef.current = id;

                const u = await listen<[number, string]>('terminal-output', (event) => {
                    const [eventId, data] = event.payload;
                    if (eventId === id) {
                        term.write(data);
                    }
                });
                if (disposed) { u(); return; }
                unlisten = u;

                term.onData((data) => {
                    invoke('write_terminal', { id, data });
                });

                // Initial fit after backend is ready + auto-focus
                handleResize();
                term.focus();
            } catch (error) {
                console.error('Failed to spawn terminal:', error);
                if (!disposed) {
                    term.write('\r\nFailed to spawn terminal backend.\r\n');
                }
            }
        };

        initTerminal();

        return () => {
            disposed = true;
            resizeObserver.disconnect();
            if (unlisten) unlisten();
            term.dispose();
        };
    }, []);

    return (
        <div className="terminal-container" onClick={() => xtermRef.current?.focus()} style={{ height: '100%', width: '100%', padding: '4px', background: 'var(--bg-primary)', overflow: 'hidden', position: 'relative' }}>
            <button
                onClick={onClose}
                style={{
                    position: 'absolute',
                    top: '5px',
                    right: '15px',
                    zIndex: 10,
                    background: 'transparent',
                    border: 'none',
                    color: 'var(--text-secondary)',
                    cursor: 'pointer',
                    fontSize: '16px'
                }}
                title="Close Terminal"
            >
                ×
            </button>
            <div ref={terminalRef} style={{ height: '100%', width: '100%' }} />
        </div>
    );
}
