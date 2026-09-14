import { useEffect, useRef } from 'react';
import { useTerminal } from '../context/TerminalContext';
import '@xterm/xterm/css/xterm.css';

export default function Terminal({ visible }: { visible: boolean }) {
    const terminalRef = useRef<HTMLDivElement>(null);
    const { xterm, fitAddon, status, connect } = useTerminal();

    // Handle visibility changes and DOM attachment
    useEffect(() => {
        if (!xterm || !terminalRef.current) return;

        if (visible) {
            // Re-open/Re-attach to the current DOM element whenever it becomes visible
            // This is the most reliable way to fix the "black screen" in dynamic layouts
            xterm.open(terminalRef.current);
            
            // Reconnect if completely stopped and not forbidden
            if (status === 'disconnected') {
                connect();
            }

            // Small delay to ensure the container is visible and has dimensions
            const timer = setTimeout(() => {
                if (fitAddon) {
                    fitAddon.fit();
                }
                xterm.refresh(0, xterm.rows - 1);
                xterm.scrollToBottom();
                xterm.focus();
            }, 150);

            return () => clearTimeout(timer);
        }
    }, [visible, xterm, fitAddon, status, connect]);

    // Handle global resize events
    useEffect(() => {
        const handleResize = () => {
            if (visible && fitAddon) {
                fitAddon.fit();
            }
        };

        window.addEventListener('resize', handleResize);
        return () => window.removeEventListener('resize', handleResize);
    }, [visible, fitAddon]);

    if (status === 'forbidden') {
        return (
            <div style={{ padding: '2rem', textAlign: 'center', color: '#f8fafc' }}>
                <h2 style={{ color: '#ef4444' }}>Access Denied</h2>
                <p style={{ marginTop: '1rem', color: 'var(--text-secondary)' }}>
                    Terminal access is disabled. You must enable <b>Developer Mode</b> in Settings to use this feature.
                </p>
                <p style={{ marginTop: '0.5rem', fontSize: '0.9rem', color: 'var(--accent-color)' }}>
                    Navigate to Settings to enable it.
                </p>
            </div>
        );
    }

    return (
        <div style={{ height: 'calc(100vh - 120px)', padding: '1rem', display: 'flex', flexDirection: 'column' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                    <h2 style={{ fontSize: '1.5rem', fontWeight: 600, margin: 0 }}>Terminal</h2>
                    {status === 'connecting' && <span style={{ fontSize: '0.8rem', color: 'var(--accent-color)', opacity: 0.8 }}>Initializing session...</span>}
                </div>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.85rem' }}>
                    <div style={{ 
                        width: '8px', height: '8px', borderRadius: '50%', 
                        background: status === 'connected' ? '#10b981' : status === 'connecting' ? '#f59e0b' : '#ef4444',
                        boxShadow: status === 'connected' ? '0 0 10px #10b981' : 'none'
                    }}></div>
                    <span style={{ color: status === 'connected' ? '#10b981' : 'var(--text-secondary)' }}>
                        {status.charAt(0).toUpperCase() + status.slice(1)}
                    </span>
                </div>
            </div>
            <div
                ref={terminalRef}
                style={{
                    flex: 1,
                    background: '#0f172a',
                    borderRadius: '12px',
                    padding: '1rem',
                    overflow: 'hidden',
                    border: '1px solid var(--glass-border)',
                    boxShadow: '0 10px 25px -5px rgba(0, 0, 0, 0.3)'
                }}
            />
        </div>
    );
}
