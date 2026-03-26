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

        // Initialize xterm
        const term = new XTerm({
            cursorBlink: true,
            theme: {
                background: 'var(--bg-primary)',
                foreground: '#d4d4d4',
            },
            fontFamily: 'var(--font-mono)',
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
            fitAddon.fit();
            if (terminalIdRef.current !== null) {
                invoke('resize_terminal', {
                    id: terminalIdRef.current,
                    rows: term.rows,
                    cols: term.cols,
                });
            }
        };

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

                window.addEventListener('resize', handleResize);
                handleResize();
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
            if (unlisten) unlisten();
            window.removeEventListener('resize', handleResize);
            term.dispose();
        };
    }, []);

    return (
        <div className="terminal-container" style={{ height: '100%', width: '100%', padding: '4px', background: 'var(--bg-primary)', overflow: 'hidden', position: 'relative' }}>
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
