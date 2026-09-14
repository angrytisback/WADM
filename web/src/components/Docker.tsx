import { useState, useEffect, useCallback } from 'react';
import { useToast } from '../context/ToastContext';
import { useSystem } from '../context/SystemContext';
import { useModal } from '../context/ModalContext';


interface Container {
    id: string;
    name: string;
    image: string;
    status: string;
    state: string;
}

interface DockerStatus {
    installed: boolean;
    running: boolean;
    version: string;
}

export default function Docker() {
    const { confirm } = useModal();
    const [containers, setContainers] = useState<Container[]>([]);
    const [statsMap, setStatsMap] = useState<Record<string, { cpu: number, memory: number }>>({});
    const [status, setStatus] = useState<DockerStatus | null>(null);
    const [loading, setLoading] = useState(true);
    // Modified to track { id: 'container_id', action: 'restart' } or 'service'
    const [actionState, setActionState] = useState<{ id: string, action: string } | string | null>(null);
    const { addToast } = useToast();
    const { systemInfo } = useSystem();
    const canManage = systemInfo?.is_root || systemInfo?.has_sudo;
    const privilegeHint = !canManage ? "Root or Sudo privileges required for this action" : "";

    const fetchContainers = useCallback(async () => {
        try {
            const res = await fetch('/api/docker');
            if (res.ok) {
                const data = await res.json();
                setContainers(data);
            }
        } catch (err) {
            console.error(err);
            addToast("Failed to fetch containers", "error");
        } finally {
            setLoading(false);
        }
    }, [addToast]);

    const checkStatus = useCallback(async () => {
        setLoading(true);
        try {
            const res = await fetch('/api/docker/status');
            const data: DockerStatus = await res.json();
            setStatus(data);

            if (data.running) {
                fetchContainers();
            } else {
                setLoading(false);
            }
        } finally {
            setLoading(false);
        }
    }, [fetchContainers]);

    useEffect(() => {
        checkStatus();
    }, [checkStatus]);


    useEffect(() => {
        let interval: number;
        if (containers.length > 0) {
            const fetchStats = async () => {
                const runningContainers = containers.filter(c => c.state === 'running');
                if (runningContainers.length === 0) return;


                const newStats: Record<string, { cpu: number, memory: number }> = {};

                await Promise.all(runningContainers.map(async (c) => {
                    try {
                        const res = await fetch(`/api/docker/${c.id}/stats`);
                        if (res.ok) {
                            const data = await res.json();
                            newStats[c.id] = {
                                cpu: data.cpu_usage,
                                memory: data.memory_usage
                            };
                        }
                    } catch {

                    }
                }));

                setStatsMap(prev => ({ ...prev, ...newStats }));
            };

            fetchStats();
            interval = setInterval(fetchStats, 5000);
        }
        return () => clearInterval(interval);
    }, [containers]);

    const startService = async () => {
        setActionState('service');
        try {
            const res = await fetch('/api/docker/start', { method: 'POST' });
            if (res.ok) {
                addToast("Docker service started", "success");
                setTimeout(checkStatus, 2000);
            } else {
                throw new Error("Failed to start service");
            }
        } catch {
            addToast("Failed to fetch Docker info", "error");
        } finally {
            setActionState(null);
        }
    };

    const installDocker = async () => {
        setActionState('installing');
        try {
            const res = await fetch('/api/dependencies/install', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name: 'Docker' }),
            });
            if (res.ok) {
                addToast("Docker installation started", "success");
                setTimeout(checkStatus, 10000);
            } else {
                const err = await res.json();
                addToast(`Installation failed: ${err}`, "error");
            }
        } catch {
            addToast("Failed to start Docker installation", "error");
        } finally {
            setActionState(null);
        }
    };

    const handleAction = async (id: string, action: string) => {
        setActionState({ id, action });
        try {
            const res = await fetch(`/api/docker/${id}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ action }),
            });
            if (res.ok) {
                const actionText = action === 'remove' ? 'removed' : `${action}ed`;
                addToast(`Container ${actionText}`, "success");
                fetchContainers();
            } else {
                const err = await res.json();
                addToast(`Failed: ${err}`, "error");
            }
        } catch {
            addToast("Operation failed", "error");
        } finally {
            setActionState(null);
        }
    };

    const formatBytes = (bytes: number) => {
        if (!bytes) return '-';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    };

    if (loading) return <div className="glass-panel" style={{ padding: '2rem' }}>Loading Docker status...</div>;

    if (!status?.installed) {
        return (
            <div className="glass-panel fade-in" style={{ padding: '3rem', textAlign: 'center' }}>
                <div style={{ fontSize: '3rem', marginBottom: '1rem' }}>🐳</div>
                <h3 style={{ marginBottom: '1rem' }}>Docker Not Found</h3>
                <p style={{ color: 'var(--text-secondary)', marginBottom: '2rem', maxWidth: '400px', marginInline: 'auto' }}>
                    Docker is not installed on this system. You can install it automatically using the button below.
                </p>
                <button
                    className="btn-primary"
                    onClick={installDocker}
                    disabled={actionState === 'installing' || !canManage}
                    title={privilegeHint}
                    style={{ display: 'flex', alignItems: 'center', gap: '0.8rem', marginInline: 'auto', opacity: !canManage ? 0.5 : 1 }}
                >
                    {actionState === 'installing' && <span className="spinner"></span>}
                    {actionState === 'installing' ? 'Installing Docker...' : 'Install Docker Automatically'}
                </button>
            </div>
        );
    }

    if (!status?.running) {
        return (
            <div className="glass-panel" style={{ padding: '3rem', textAlign: 'center' }}>
                <div style={{ fontSize: '3rem', marginBottom: '1rem', filter: 'grayscale(1)' }}>🐳</div>
                <h3>Docker Service Stopped</h3>
                <p style={{ color: 'var(--text-secondary)', marginBottom: '2rem' }}>
                    The Docker service is installed ({status.version}) but not currently running.
                </p>
                <button
                    className="btn-primary"
                    onClick={startService}
                    disabled={actionState === 'service' || !canManage}
                    title={privilegeHint}
                    style={{ display: 'flex', alignItems: 'center', gap: '0.8rem', marginInline: 'auto', opacity: !canManage ? 0.5 : 1 }}
                >
                    {actionState === 'service' && <span className="spinner"></span>}
                    {actionState === 'service' ? 'Starting...' : 'Start Docker Service'}
                </button>
            </div>
        );
    }

    return (
        <div className="glass-panel" style={{ padding: '2rem' }}>
            <header style={{ marginBottom: '2rem', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div>
                    <h3 style={{ margin: 0 }}>Docker Containers</h3>
                    <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>version {status.version}</div>
                </div>
                <button className="btn-primary" onClick={fetchContainers}>Refresh</button>
            </header>

            {containers.length === 0 ? (
                <p style={{ color: 'var(--text-secondary)' }}>No containers found.</p>
            ) : (
                <div style={{ overflowX: 'auto' }}>
                    <table style={{ width: '100%', borderCollapse: 'collapse' }}>
                        <thead>
                            <tr style={{ textAlign: 'left', borderBottom: '1px solid var(--glass-border)' }}>
                                <th style={{ padding: '1rem' }}>Name</th>
                                <th style={{ padding: '1rem' }}>Image</th>
                                <th style={{ padding: '1rem' }}>Status</th>
                                <th style={{ padding: '1rem' }}>CPU</th>
                                <th style={{ padding: '1rem' }}>Memory</th>
                                <th style={{ padding: '1rem' }}>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            {containers.map((c) => (
                                <tr key={c.id} style={{ borderBottom: '1px solid var(--glass-border)' }}>
                                    <td style={{ padding: '1rem', fontWeight: 500 }}>{c.name}</td>
                                    <td style={{ padding: '1rem', color: 'var(--text-secondary)' }}>{c.image}</td>
                                    <td style={{ padding: '1rem' }}>
                                        <span className={`badge ${c.state === 'running' ? 'success' : 'neutral'}`}>
                                            {c.status}
                                        </span>
                                    </td>
                                    <td style={{ padding: '1rem', fontFamily: 'monospace' }}>
                                        {c.state === 'running' ? (
                                            statsMap[c.id] ? `${statsMap[c.id].cpu.toFixed(2)}%` : '...'
                                        ) : '-'}
                                    </td>
                                    <td style={{ padding: '1rem', fontFamily: 'monospace' }}>
                                        {c.state === 'running' ? (
                                            statsMap[c.id] ? formatBytes(statsMap[c.id].memory) : '...'
                                        ) : '-'}
                                    </td>
                                    <td style={{ padding: '1rem', display: 'flex', gap: '0.5rem' }}>
                                        {c.state === 'running' ? (
                                            <>
                                                <button
                                                    className="btn-sm"
                                                    onClick={() => handleAction(c.id, 'restart')}
                                                    disabled={!!actionState || !canManage}
                                                    title={privilegeHint}
                                                    style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', opacity: !canManage ? 0.5 : 1 }}
                                                >
                                                    {typeof actionState === 'object' && actionState?.id === c.id && actionState.action === 'restart' && <span className="spinner" style={{ width: '0.8rem', height: '0.8rem' }}></span>}
                                                    Restart
                                                </button>
                                                <button
                                                    className="btn-sm danger"
                                                    onClick={() => handleAction(c.id, 'stop')}
                                                    disabled={!!actionState || !canManage}
                                                    title={privilegeHint}
                                                    style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', opacity: !canManage ? 0.5 : 1 }}
                                                >
                                                    {typeof actionState === 'object' && actionState?.id === c.id && actionState.action === 'stop' && <span className="spinner" style={{ width: '0.8rem', height: '0.8rem' }}></span>}
                                                    Stop
                                                </button>
                                            </>
                                        ) : (
                                            <>
                                                <button
                                                    className="btn-sm success"
                                                    onClick={() => handleAction(c.id, 'start')}
                                                    disabled={!!actionState || !canManage}
                                                    title={privilegeHint}
                                                    style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', opacity: !canManage ? 0.5 : 1 }}
                                                >
                                                    {typeof actionState === 'object' && actionState?.id === c.id && actionState.action === 'start' && <span className="spinner" style={{ width: '0.8rem', height: '0.8rem' }}></span>}
                                                    Start
                                                </button>
                                                <button
                                                    className="btn-sm danger"
                                                    onClick={async () => {
                                                        const ok = await confirm({
                                                            title: 'Delete Container',
                                                            message: `Are you sure you want to delete container ${c.name}?`,
                                                            type: 'danger',
                                                            confirmText: 'Delete'
                                                        });
                                                        if (ok) {
                                                            handleAction(c.id, 'remove');
                                                        }
                                                    }}
                                                    disabled={!!actionState || !canManage}
                                                    title={privilegeHint}
                                                    style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', opacity: !canManage ? 0.5 : 1 }}
                                                >
                                                    {typeof actionState === 'object' && actionState?.id === c.id && actionState.action === 'remove' && <span className="spinner" style={{ width: '0.8rem', height: '0.8rem' }}></span>}
                                                    Delete
                                                </button>
                                            </>
                                        )}
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            )}
        </div>
    );
}
