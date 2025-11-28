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
                background: '#1e1e1e',
                foreground: '#d4d4d4',
            },
            fontFamily: 'Menlo, Monaco, "Courier New", monospace',
            fontSize: 14,
        });

        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);

        term.open(terminalRef.current);
        fitAddon.fit();

        xtermRef.current = term;
        fitAddonRef.current = fitAddon;

        // Spawn terminal backend
        const initTerminal = async () => {
            try {
                const id = await invoke<number>('spawn_terminal');
                terminalIdRef.current = id;

                // Listen for output
                const unlisten = await listen<[number, string]>('terminal-output', (event) => {
                    const [eventId, data] = event.payload;
                    if (eventId === id) {
                        term.write(data);
                    }
                });

                // Handle input
                term.onData((data) => {
                    invoke('write_terminal', { id, data });
                });

                // Handle resize
                const handleResize = () => {
                    fitAddon.fit();
                    if (terminalIdRef.current !== null) {
                        invoke('resize_terminal', {
                            id: terminalIdRef.current,
                            rows: term.rows,
                            cols: term.cols,
                        });
                    }
                };

                window.addEventListener('resize', handleResize);

                // Initial resize
                handleResize();

                return () => {
                    unlisten();
                    window.removeEventListener('resize', handleResize);
                    term.dispose();
                };
            } catch (error) {
                console.error('Failed to spawn terminal:', error);
                term.write('\r\nFailed to spawn terminal backend.\r\n');
            }
        };

        const cleanupPromise = initTerminal();

        return () => {
            cleanupPromise.then(cleanup => cleanup && cleanup());
        };
    }, []);

    return (
        <div className="terminal-container" style={{ height: '100%', width: '100%', padding: '4px', background: '#1e1e1e', overflow: 'hidden', position: 'relative' }}>
            <button
                onClick={onClose}
                style={{
                    position: 'absolute',
                    top: '5px',
                    right: '15px',
                    zIndex: 10,
                    background: 'transparent',
                    border: 'none',
                    color: '#666',
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
