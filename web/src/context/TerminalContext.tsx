import React, { createContext, useContext, useState, useEffect, useRef } from 'react';
import { Terminal as XTerm } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';

interface TerminalContextType {
    status: 'connecting' | 'connected' | 'disconnected' | 'forbidden';
    xterm: XTerm | null;
    fitAddon: FitAddon | null;
    connect: () => void;
}

const TerminalContext = createContext<TerminalContextType | undefined>(undefined);

export function TerminalProvider({ children }: { children: React.ReactNode }) {
    const [status, setStatus] = useState<TerminalContextType['status']>('connecting');
    const xtermRef = useRef<XTerm | null>(null);
    const fitRef = useRef<FitAddon | null>(null);
    const wsRef = useRef<WebSocket | null>(null);
    const reconnectTimeoutRef = useRef<number | null>(null);
    
    const connect = () => {
        // Clear any pending reconnection
        if (reconnectTimeoutRef.current) {
            window.clearTimeout(reconnectTimeoutRef.current);
            reconnectTimeoutRef.current = null;
        }

        if (wsRef.current && (wsRef.current.readyState === WebSocket.OPEN || wsRef.current.readyState === WebSocket.CONNECTING)) {
            return;
        }

        setStatus('connecting');
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const token = localStorage.getItem('wadm_token') || '';
        
        if (!token) {
            setStatus('disconnected');
            if (xtermRef.current) xtermRef.current.writeln('\r\n\x1b[31mError: Authentication token missing. Please login again.\x1b[0m');
            return;
        }

        const wsUrl = `${protocol}//${window.location.host}/api/terminal/ws?token=${token}`;
        const ws = new WebSocket(wsUrl);
        wsRef.current = ws;

        ws.onopen = () => {
            setStatus('connected');
            if (xtermRef.current) {
                xtermRef.current.writeln('\x1b[32m--- Connected to WADM Terminal ---\x1b[0m');
                // Sync initial size
                if (fitRef.current) {
                    fitRef.current.fit();
                    const { cols, rows } = xtermRef.current;
                    ws.send(`RESIZE:${cols}x${rows}`);
                }
            }
        };

        ws.onmessage = (event) => {
            if (!xtermRef.current) return;
            if (typeof event.data === 'string') {
                xtermRef.current.write(event.data);
            } else {
                const reader = new FileReader();
                reader.onload = () => {
                    if (xtermRef.current) xtermRef.current.write(new Uint8Array(reader.result as ArrayBuffer));
                };
                reader.readAsArrayBuffer(event.data);
            }
        };

        ws.onclose = (event) => {
            if (event.code === 1008 || event.reason.includes("Forbidden")) {
                setStatus('forbidden');
                if (xtermRef.current) xtermRef.current.writeln('\r\n\x1b[31mAccess Denied: Developer Mode is disabled in Settings.\x1b[0m');
            } else {
                setStatus('disconnected');
                if (xtermRef.current) xtermRef.current.writeln('\r\n\x1b[33mConnection lost. Retrying in 3 seconds...\x1b[0m');
                
                // Throttled auto-reconnect
                reconnectTimeoutRef.current = window.setTimeout(() => {
                    connect();
                }, 3000);
            }
        };

        ws.onerror = () => {
            setStatus('disconnected');
        };
    };

    useEffect(() => {
        // Initialize xterm once
        const term = new XTerm({
            cursorBlink: true,
            theme: {
                background: '#0f172a',
                foreground: '#f8fafc',
                cursor: '#38bdf8',
            },
            fontFamily: '"JetBrains Mono", monospace',
            fontSize: 14,
            allowProposedApi: true,
        });
        const fitAddon = new FitAddon();
        term.loadAddon(fitAddon);
        
        xtermRef.current = term;
        fitRef.current = fitAddon;

        // Handle data input
        term.onData(data => {
            if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
                wsRef.current.send(data);
            }
        });

        // Handle resizing - MUST inform backend
        term.onResize(({ cols, rows }) => {
            if (wsRef.current && wsRef.current.readyState === WebSocket.OPEN) {
                wsRef.current.send(`RESIZE:${cols}x${rows}`);
            }
        });

        connect();

        return () => {
            if (reconnectTimeoutRef.current) window.clearTimeout(reconnectTimeoutRef.current);
            if (wsRef.current) wsRef.current.close();
            term.dispose();
        };
    }, []);

    return (
        <TerminalContext.Provider value={{ status, xterm: xtermRef.current, fitAddon: fitRef.current, connect }}>
            {children}
        </TerminalContext.Provider>
    );
}

export function useTerminal() {
    const context = useContext(TerminalContext);
    if (context === undefined) {
        throw new Error('useTerminal must be used within a TerminalProvider');
    }
    return context;
}
