import { useState, useEffect, useCallback } from 'react';
import { useToast } from '../context/ToastContext';

interface Service {
    name: string;
    status: string;
    description: string;
}

export default function Services() {
    const [services, setServices] = useState<Service[]>([]);
    const [loading, setLoading] = useState(true);
    const [actionLoading, setActionLoading] = useState<string | null>(null);
    const [searchQuery, setSearchQuery] = useState('');
    const [logs, setLogs] = useState<string | null>(null);
    const [viewingLogs, setViewingLogs] = useState<string | null>(null);
    const { addToast } = useToast();

    const fetchServices = useCallback(async () => {
        try {
            const res = await fetch('/api/services');
            const data = await res.json();
            setServices(data);
        } catch {
            addToast("Failed to toggle service", "error");
        } finally {
            setLoading(false);
        }
    }, [addToast]);

    useEffect(() => {
        fetchServices();
    }, [fetchServices]);

    const handleAction = async (name: string, action: string) => {
        setActionLoading(name);
        try {
            const res = await fetch(`/api/services/${name}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ action }),
            });
            if (res.ok) {
                addToast(`Service ${name} ${action}ed`, 'success');
                
                fetchServices();
                setTimeout(fetchServices, 1000);
                setTimeout(fetchServices, 3000);
            } else {
                const errorText = await res.text();
                addToast(`Action failed: ${errorText}`, 'error');
            }
        } catch (err) {
            console.error(err);
            addToast('Network error', 'error');
        } finally {
            setActionLoading(null);
        }
    };

    const fetchLogs = async (name: string) => {
        setViewingLogs(name);
        setLogs('Loading logs...');
        try {
            const res = await fetch(`/api/services/${name}/logs`);
            if (res.ok) {
                const text = await res.json();
                setLogs(text);
            } else {
                setLogs('Failed to load logs');
                addToast('Failed to load logs', 'error');
            }
        } catch {
            setLogs('Network error');
            addToast('Network error loading logs', 'error');
        }
    };

    const filteredServices = services.filter(svc =>
        svc.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        svc.description.toLowerCase().includes(searchQuery.toLowerCase())
    );

    if (loading) return <div className="glass-panel" style={{ padding: '2rem' }}>Loading services...</div>;

    return (
        <div className="glass-panel" style={{ padding: '2rem', height: '100%', display: 'flex', flexDirection: 'column', position: 'relative' }}>
            <header style={{ marginBottom: '1.5rem', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <h3 style={{ margin: 0 }}>System Services</h3>
                <div style={{ display: 'flex', gap: '1rem' }}>
                    <input
                        type="text"
                        className="input-field"
                        placeholder="Search services..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        style={{ minWidth: '250px' }}
                    />
                    <button onClick={fetchServices} className="btn-primary">Refresh</button>
                </div>
            </header>

            <div style={{ overflowX: 'auto', flex: 1 }}>
                <table style={{ width: '100%', borderCollapse: 'collapse' }}>
                    <thead>
                        <tr style={{ textAlign: 'left', borderBottom: '1px solid var(--glass-border)' }}>
                            <th style={{ padding: '1rem' }}>Name</th>
                            <th style={{ padding: '1rem' }}>Status</th>
                            <th style={{ padding: '1rem' }}>Description</th>
                            <th style={{ padding: '1rem' }}>Actions</th>
                        </tr>
                    </thead>
                    <tbody>
                        {filteredServices.map((svc) => (
                            <tr key={svc.name} style={{ borderBottom: '1px solid var(--glass-border)' }}>
                                <td style={{ padding: '1rem', fontWeight: 500 }}>{svc.name}</td>
                                <td style={{ padding: '1rem' }}>
                                    <span className={`badge ${svc.status.toLowerCase().includes('active') ? 'success' : svc.status.includes('Failed') ? 'error' : 'neutral'}`}>
                                        {svc.status}
                                    </span>
                                </td>
                                <td style={{ padding: '1rem', color: 'var(--text-secondary)', fontSize: '0.9rem' }}>{svc.description}</td>
                                <td style={{ padding: '1rem', display: 'flex', gap: '0.5rem' }}>
                                    <button
                                        className="btn-sm"
                                        onClick={() => fetchLogs(svc.name)}
                                        style={{ background: 'rgba(255,255,255,0.1)' }}
                                    >
                                        Logs
                                    </button>
                                    <div style={{ width: '1px', background: 'var(--glass-border)', margin: '0 0.2rem' }}></div>
                                    {svc.status.toLowerCase().includes('active') ? (
                                        <>
                                            <button
                                                className="btn-sm warning"
                                                onClick={() => handleAction(svc.name, 'restart')}
                                                disabled={actionLoading === svc.name}
                                            >
                                                Restart
                                            </button>
                                            <button
                                                className="btn-sm error"
                                                onClick={() => handleAction(svc.name, 'stop')}
                                                disabled={actionLoading === svc.name}
                                            >
                                                Stop
                                            </button>
                                        </>
                                    ) : (
                                        <>
                                            <button
                                                className="btn-sm success"
                                                onClick={() => handleAction(svc.name, 'start')}
                                                disabled={actionLoading === svc.name}
                                            >
                                                Start
                                            </button>
                                            <button
                                                className="btn-sm"
                                                onClick={() => handleAction(svc.name, 'enable')}
                                                disabled={actionLoading === svc.name}
                                            >
                                                Enable
                                            </button>
                                        </>
                                    )}
                                </td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            </div>

            {/* Log Viewer Modal */}
            {viewingLogs && (
                <div style={{
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    right: 0,
                    bottom: 0,
                    background: 'rgba(0,0,0,0.8)',
                    backdropFilter: 'blur(5px)',
                    display: 'flex',
                    flexDirection: 'column',
                    padding: '2rem',
                    zIndex: 100
                }}>
                    <div className="glass-panel" style={{ flex: 1, display: 'flex', flexDirection: 'column', padding: '0', overflow: 'hidden' }}>
                        <header style={{ padding: '1rem', borderBottom: '1px solid var(--glass-border)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                            <h3 style={{ margin: 0 }}>Logs: {viewingLogs}</h3>
                            <div style={{ display: 'flex', gap: '1rem' }}>
                                <button onClick={() => fetchLogs(viewingLogs)} className="btn-sm">Refresh</button>
                                <button onClick={() => setViewingLogs(null)} className="btn-sm error">Close</button>
                            </div>
                        </header>
                        <div style={{ flex: 1, overflow: 'auto', padding: '1rem', fontFamily: 'monospace', fontSize: '0.85rem', whiteSpace: 'pre-wrap', color: '#ccc' }}>
                            {logs}
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}
