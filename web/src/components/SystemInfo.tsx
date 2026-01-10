import { useState, useEffect } from 'react';

interface DetailedSystemInfo {
    os_name: string;
    os_version: string;
    kernel_version: string;
    host_name: string;
    uptime: number;
    cpu_arch: string;
    cpu_count: number;
    total_memory: number;
    used_memory: number;
    total_swap: number;
    used_swap: number;
}

function SystemInfo() {
    const [info, setInfo] = useState<DetailedSystemInfo | null>(null);
    const [error, setError] = useState('');

    useEffect(() => {
        fetch('/api/system')
            .then(res => {
                if (!res.ok) throw new Error('Failed to fetch system info');
                return res.json();
            })
            .then(setInfo)
            .catch(err => setError(err.message));
    }, []);

    const formatUptime = (seconds: number) => {
        const days = Math.floor(seconds / (3600 * 24));
        const hours = Math.floor((seconds % (3600 * 24)) / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        return `${days}d ${hours}h ${minutes}m`;
    };

    const formatBytes = (bytes: number) => {
        const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
        if (bytes === 0) return '0 Byte';
        const i = parseInt(Math.floor(Math.log(bytes) / Math.log(1024)).toString());
        return Math.round(bytes / Math.pow(1024, i)) + ' ' + sizes[i];
    };

    if (error) return <div className="error-message">Error: {error}</div>;
    if (!info) return <div className="loading">Loading System Info...</div>;

    return (
        <div className="system-info-container fade-in">
            <div className="glass-panel" style={{ maxWidth: '800px', margin: '0 auto', padding: '2rem' }}>
                <div style={{ display: 'flex', alignItems: 'center', marginBottom: '2rem' }}>
                    <div style={{ fontSize: '4rem', marginRight: '2rem' }}>🖥️</div>
                    <div>
                        <h2 style={{ margin: 0, color: 'var(--accent-color)' }}>{info.host_name}</h2>
                        <div style={{ color: 'var(--text-secondary)' }}>{info.os_name} {info.os_version}</div>
                    </div>
                </div>

                <div className="info-grid" style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))', gap: '2rem' }}>
                    <div className="info-item glass-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                        <span className="label" style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>Kernel</span>
                        <span className="value" style={{ fontSize: '1.2rem', fontWeight: '500' }}>{info.kernel_version}</span>
                    </div>
                    <div className="info-item glass-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                        <span className="label" style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>Architecture</span>
                        <span className="value" style={{ fontSize: '1.2rem', fontWeight: '500' }}>{info.cpu_arch}</span>
                    </div>
                    <div className="info-item glass-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                        <span className="label" style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>CPU Cores</span>
                        <span className="value" style={{ fontSize: '1.2rem', fontWeight: '500' }}>{info.cpu_count}</span>
                    </div>
                    <div className="info-item glass-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                        <span className="label" style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>Uptime</span>
                        <span className="value" style={{ fontSize: '1.2rem', fontWeight: '500' }}>{formatUptime(info.uptime)}</span>
                    </div>
                    <div className="info-item glass-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                        <span className="label" style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>Memory</span>
                        <span className="value" style={{ fontSize: '1.1rem', fontWeight: '500' }}>
                            {formatBytes(info.used_memory)} / {formatBytes(info.total_memory)}
                        </span>
                        <div className="progress-bar-bg" style={{ marginTop: '1rem', height: '6px', background: 'rgba(255,255,255,0.05)', borderRadius: '3px', overflow: 'hidden' }}>
                            <div style={{
                                width: `${(info.used_memory / info.total_memory) * 100}%`,
                                height: '100%',
                                background: 'var(--accent-color)',
                                borderRadius: '3px'
                            }}></div>
                        </div>
                    </div>
                    <div className="info-item glass-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                        <span className="label" style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>Swap</span>
                        <span className="value" style={{ fontSize: '1.1rem', fontWeight: '500' }}>
                            {formatBytes(info.used_swap)} / {formatBytes(info.total_swap)}
                        </span>
                    </div>
                </div>
            </div>
        </div>
    );
}

export default SystemInfo;
