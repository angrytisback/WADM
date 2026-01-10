import { useEffect, useRef, useState } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';

export default function Terminal() {
    const terminalRef = useRef<HTMLDivElement>(null);
    const wsRef = useRef<WebSocket | null>(null);
    const xtermRef = useRef({ term: null as XTerm | null, fit: null as FitAddon | null });
    const [status, setStatus] = useState<'connecting' | 'connected' | 'disconnected' | 'forbidden'>('connecting');

    useEffect(() => {

        const term = new XTerm({
            cursorBlink: true,
            theme: {
                background: '#0f172a',
                foreground: '#f8fafc',
            },
            fontFamily: '"JetBrains Mono", monospace',
            fontSize: 14,
        });
        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);

        if (terminalRef.current) {
            term.open(terminalRef.current);
            fitAddon.fit();
        }

        xtermRef.current = { term, fit: fitAddon };


        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const token = localStorage.getItem('wadm_token') || '';
        const wsUrl = `${protocol}//${window.location.host}/api/terminal/ws?token=${token}`;
        const ws = new WebSocket(wsUrl);
        wsRef.current = ws;

        ws.onopen = () => {
            setStatus('connected');
            term.writeln('\x1b[32mConnected to WADM Terminal\x1b[0m');
            term.focus();


            setTimeout(() => {
                if (xtermRef.current.fit) {
                    xtermRef.current.fit.fit();
                    const dims = xtermRef.current.term?.rows && xtermRef.current.term?.cols
                        ? { cols: xtermRef.current.term.cols, rows: xtermRef.current.term.rows }
                        : null;
                    if (dims && ws.readyState === WebSocket.OPEN) {
                        ws.send(`RESIZE:${dims.cols}x${dims.rows}`);
                    }
                }
            }, 100);
        };

        ws.onmessage = (event) => {
            if (typeof event.data === 'string') {
                term.write(event.data);
            } else {
                const reader = new FileReader();
                reader.onload = () => {
                    term.write(new Uint8Array(reader.result as ArrayBuffer));
                };
                reader.readAsArrayBuffer(event.data);
            }
        };

        ws.onclose = (event) => {
            if (event.code === 1008 || event.reason.includes("Forbidden")) {
                setStatus('forbidden');
                term.writeln('\r\n\x1b[31mAccess Denied: Developer Mode is disabled.\x1b[0m');
            } else {
                setStatus('disconnected');
                term.writeln('\r\n\x1b[33mConnection closed.\x1b[0m');
            }
        };

        ws.onerror = () => {
            setStatus('disconnected');
        };


        term.onData(data => {
            if (ws.readyState === WebSocket.OPEN) {
                ws.send(data);
            }
        });


        const handleResize = () => {
            if (xtermRef.current.fit) {
                xtermRef.current.fit.fit();
                const cols = xtermRef.current.term?.cols;
                const rows = xtermRef.current.term?.rows;
                if (cols && rows && ws.readyState === WebSocket.OPEN) {
                    ws.send(`RESIZE:${cols}x${rows}`);
                }
            }
        };

        window.addEventListener('resize', handleResize);

        return () => {
            window.removeEventListener('resize', handleResize);
            if (ws.readyState === WebSocket.OPEN) ws.close();
            term.dispose();
        };
    }, []);

    if (status === 'forbidden') {
        return (
            <div style={{ padding: '2rem', textAlign: 'center', color: '#f8fafc' }}>
                <h2 style={{ color: '#ef4444' }}>Access Denied</h2>
                <p style={{ marginTop: '1rem', color: 'var(--text-secondary)' }}>
                    Terminal access is disabled. You must enable <b>Developer Mode</b> in Settings to use this feature.
                </p>
                {/* We assume navigation is handled by parent, so no link here, or just text advice */}
                <p style={{ marginTop: '0.5rem', fontSize: '0.9rem', color: 'var(--accent-color)' }}>
                    Navigate to Settings to enable it.
                </p>
            </div>
        );
    }

    return (
        <div style={{ height: 'calc(100vh - 100px)', padding: '1rem', display: 'flex', flexDirection: 'column' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
                <h2 style={{ fontSize: '1.5rem', fontWeight: 600, margin: 0 }}>Terminal</h2>
                <div style={{ fontSize: '0.85rem', color: status === 'connected' ? '#10b981' : '#ef4444' }}>
                    {status === 'connected' ? '● Connected' : '● Disconnected'}
                </div>
            </div>
            <div
                ref={terminalRef}
                style={{
                    flex: 1,
                    background: '#0f172a',
                    borderRadius: '8px',
                    padding: '1rem',
                    overflow: 'hidden',
                    boxShadow: '0 4px 6px -1px rgba(0, 0, 0, 0.1)'
                }}
            />
        </div>
    );
}
