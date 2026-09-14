import { useState, useEffect, useRef } from 'react';
import { FaTrash, FaSync, FaDownload } from 'react-icons/fa';
import { useToast } from '../context/ToastContext';

interface LogEntry {
    timestamp: string;
    level: string;
    message: string;
}

export default function Logs() {
    const [logs, setLogs] = useState<LogEntry[]>([]);
    const [loading, setLoading] = useState(true);
    const [autoRefresh, setAutoRefresh] = useState(true);
    const { addToast } = useToast();
    const scrollRef = useRef<HTMLDivElement>(null);

    const fetchLogs = async () => {
        try {
            const res = await fetch('/api/logs');
            if (res.ok) {
                const data = await res.json();
                setLogs(data);
            }
        } catch (err) {
            console.error("Failed to fetch logs:", err);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchLogs();
    }, []);

    useEffect(() => {
        if (autoRefresh) {
            const interval = setInterval(fetchLogs, 2000);
            return () => clearInterval(interval);
        }
    }, [autoRefresh]);

    useEffect(() => {
        if (scrollRef.current) {
            scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
    }, [logs]);

    const clearLogs = async () => {
        try {
            const res = await fetch('/api/logs/clear', { method: 'POST' });
            if (res.ok) {
                setLogs([]);
                addToast("Logs cleared", "success");
            }
        } catch {
            addToast("Failed to clear logs", "error");
        }
    };

    const downloadLogs = () => {
        const text = logs.map(l => `[${l.timestamp}] ${l.level}: ${l.message}`).join('\n');
        const blob = new Blob([text], { type: 'text/plain' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `wadm_logs_${new Date().toISOString().split('T')[0]}.log`;
        a.click();
    };

    const getLevelColor = (level: string) => {
        switch (level.toUpperCase()) {
            case 'ERROR': return '#f87171';
            case 'WARN': return '#fbbf24';
            case 'INFO': return '#60a5fa';
            case 'DEBUG': return '#94a3b8';
            default: return 'var(--text-secondary)';
        }
    };

    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem', height: 'calc(100vh - 180px)' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                    <div className="glass-panel" style={{ padding: '0.4rem 1rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <input 
                            type="checkbox" 
                            id="auto-refresh" 
                            checked={autoRefresh} 
                            onChange={(e) => setAutoRefresh(e.target.checked)}
                            style={{ cursor: 'pointer' }}
                        />
                        <label htmlFor="auto-refresh" style={{ fontSize: '0.9rem', cursor: 'pointer' }}>Auto-refresh</label>
                    </div>
                    <span style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>{logs.length} entries stored</span>
                </div>
                <div style={{ display: 'flex', gap: '0.75rem' }}>
                    <button className="btn-sm" onClick={fetchLogs} title="Refresh Now">
                        <FaSync className={loading ? 'spin' : ''} />
                    </button>
                    <button className="btn-sm" onClick={downloadLogs} title="Download Logs">
                        <FaDownload />
                    </button>
                    <button className="btn-sm danger" onClick={clearLogs} title="Clear Logs">
                        <FaTrash />
                    </button>
                </div>
            </div>

            <div 
                ref={scrollRef}
                className="glass-panel" 
                style={{ 
                    flex: 1, 
                    padding: '1.5rem', 
                    fontFamily: '"Fira Code", monospace', 
                    fontSize: '0.85rem', 
                    overflowY: 'auto',
                    background: 'rgba(0,0,0,0.3)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: '0.25rem'
                }}
            >
                {logs.length === 0 ? (
                    <div style={{ textAlign: 'center', color: 'var(--text-secondary)', marginTop: '2rem' }}>
                        No logs available.
                    </div>
                ) : (
                    logs.map((log, i) => (
                        <div key={i} style={{ display: 'flex', gap: '1rem', borderBottom: '1px solid rgba(255,255,255,0.03)', paddingBottom: '2px' }}>
                            <span style={{ color: 'var(--text-secondary)', minWidth: '150px' }}>{log.timestamp}</span>
                            <span style={{ 
                                color: getLevelColor(log.level), 
                                fontWeight: 700, 
                                minWidth: '60px',
                                textAlign: 'right'
                            }}>[{log.level}]</span>
                            <span style={{ color: 'var(--text-primary)', wordBreak: 'break-all' }}>{log.message}</span>
                        </div>
                    ))
                )}
            </div>

            <style>{`
                .spin { animation: spin 1s linear infinite; }
                @keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
            `}</style>
        </div>
    );
}
