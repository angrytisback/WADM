import { useState, useEffect } from 'react';
import { FaMicrochip, FaThermometerHalf } from 'react-icons/fa';
import type { GpuStats } from '../types';

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
    username: string;
    has_sudo: boolean;
    is_root: boolean;
    cpu_temp?: number;
    gpu_temp?: number;
    gpus: GpuStats[];
    smart?: {
        name: string;
        model: string;
        serial: string;
        health: string;
        temperature: number | null;
        power_on_hours: number | null;
    }[];
}

function SystemInfo() {
    const [info, setInfo] = useState<DetailedSystemInfo | null>(null);
    const [error, setError] = useState('');
    const [testingSpeed, setTestingSpeed] = useState(false);
    const [speedResult, setSpeedResult] = useState<{download_mbps: number, upload_mbps: number, ping_ms: number} | null>(null);
    const [speedError, setSpeedError] = useState('');

    useEffect(() => {
        fetch('/api/system')
            .then(res => {
                if (!res.ok) throw new Error('Failed to fetch system info');
                return res.json();
            })
            .then(setInfo)
            .catch(err => setError(err.message));
    }, []);

    const runSpeedtest = async () => {
        setTestingSpeed(true);
        setSpeedError('');
        try {
            const res = await fetch('/api/system/speedtest', { method: 'POST' });
            if (!res.ok) throw new Error(await res.text());
            const data = await res.json();
            setSpeedResult(data);
        } catch (err: any) {
            setSpeedError(err.message || 'Speedtest failed');
        } finally {
            setTestingSpeed(false);
        }
    };

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
                    <div style={{ fontSize: '4rem', marginRight: '2rem', color: 'var(--text-secondary)' }}>
                        <FaMicrochip />
                    </div>
                    <div style={{ flex: 1 }}>
                        <h2 style={{ margin: 0, color: 'var(--accent-color)' }}>{info.host_name}</h2>
                        <div style={{ color: 'var(--text-secondary)' }}>{info.os_name} {info.os_version}</div>
                    </div>
                    {(info.cpu_temp || info.gpu_temp) && (
                        <div style={{ display: 'flex', gap: '1.5rem', alignItems: 'center' }}>
                            {info.cpu_temp && (
                                <div style={{ textAlign: 'right' }}>
                                    <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', fontWeight: 600 }}>CPU TEMP</div>
                                    <div style={{ fontSize: '1.5rem', fontWeight: 800, color: '#34d399', display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                                        <FaThermometerHalf size={16} /> {info.cpu_temp.toFixed(1)}°C
                                    </div>
                                </div>
                            )}
                            {info.gpu_temp && (
                                <div style={{ textAlign: 'right' }}>
                                    <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', fontWeight: 600 }}>GPU TEMP</div>
                                    <div style={{ fontSize: '1.5rem', fontWeight: 800, color: '#818cf8', display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                                        <FaThermometerHalf size={16} /> {info.gpu_temp.toFixed(0)}°C
                                    </div>
                                </div>
                            )}
                        </div>
                    )}
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

                {info.gpus && info.gpus.length > 0 && (
                    <div style={{ marginTop: '2rem' }}>
                        <h3 style={{ marginBottom: '1rem', borderBottom: '1px solid var(--glass-border)', paddingBottom: '0.5rem' }}>Graphics Information (GPU)</h3>
                        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: '1rem' }}>
                            {info.gpus.map((gpu, idx) => (
                                <div key={idx} className="glass-panel" style={{ padding: '1rem' }}>
                                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem' }}>
                                        <strong style={{ fontSize: '1.1rem' }}>{gpu.name}</strong>
                                        <span className={`badge ${gpu.error ? 'error' : 'success'}`}>
                                            {gpu.vendor}
                                        </span>
                                    </div>
                                    <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginBottom: '0.75rem', fontFamily: 'monospace' }}>
                                        PCI ID: {gpu.pci_id || 'N/A'}
                                    </div>
                                    
                                    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.75rem' }}>
                                        <div>
                                            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.85rem', marginBottom: '0.3rem' }}>
                                                <span style={{ color: 'var(--text-secondary)' }}>Load</span>
                                                <span style={{ fontWeight: 600 }}>{gpu.load.toFixed(1)}%</span>
                                            </div>
                                            <div className="progress-bar-bg" style={{ height: '4px', background: 'rgba(255,255,255,0.05)', borderRadius: '2px' }}>
                                                <div style={{ width: `${gpu.load}%`, height: '100%', background: '#818cf8', borderRadius: '2px' }}></div>
                                            </div>
                                        </div>
                                        
                                        <div>
                                            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.85rem', marginBottom: '0.3rem' }}>
                                                <span style={{ color: 'var(--text-secondary)' }}>VRAM</span>
                                                <span style={{ fontWeight: 600 }}>{formatBytes(gpu.vram_used)} / {formatBytes(gpu.vram_total)}</span>
                                            </div>
                                            <div className="progress-bar-bg" style={{ height: '4px', background: 'rgba(255,255,255,0.05)', borderRadius: '2px' }}>
                                                <div style={{ 
                                                    width: `${gpu.vram_total > 0 ? (gpu.vram_used / gpu.vram_total) * 100 : 0}%`, 
                                                    height: '100%', 
                                                    background: '#34d399', 
                                                    borderRadius: '2px' 
                                                }}></div>
                                            </div>
                                        </div>

                                        <div style={{ display: 'flex', gap: '1rem', fontSize: '0.85rem', marginTop: '0.25rem' }}>
                                            <div>
                                                <span style={{ color: 'var(--text-secondary)' }}>Temp: </span>
                                                <span style={{ color: '#f87171', fontWeight: 600 }}>{gpu.temp > 0 ? `${gpu.temp.toFixed(1)}°C` : 'N/A'}</span>
                                            </div>
                                            {gpu.error && (
                                                <div style={{ color: 'var(--danger)', fontSize: '0.75rem' }}>
                                                    ⚠️ {gpu.error}
                                                </div>
                                            )}
                                        </div>
                                    </div>
                                </div>
                            ))}
                        </div>
                    </div>
                )}

            {info.smart && info.smart.length > 0 && (
                <div style={{ marginTop: '2rem' }}>
                    <h3 style={{ marginBottom: '1rem', borderBottom: '1px solid var(--glass-border)', paddingBottom: '0.5rem' }}>Storage Health (S.M.A.R.T.)</h3>
                    <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: '1rem' }}>
                        {info.smart.map((disk, idx) => (
                            <div key={idx} className="glass-panel" style={{ padding: '1rem' }}>
                                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.5rem' }}>
                                    <strong style={{ fontSize: '1.1rem' }}>{disk.name}</strong>
                                    <span className={`badge ${disk.health === 'Passed' ? 'success' : 'error'}`}>
                                        {disk.health}
                                    </span>
                                </div>
                                <div style={{ fontSize: '0.9rem', color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>
                                    {disk.model}
                                </div>
                                <div style={{ display: 'flex', gap: '1rem', fontSize: '0.9rem' }}>
                                    <div>
                                        <span style={{ color: 'var(--text-secondary)' }}>Temp: </span>
                                        {disk.temperature ? `${disk.temperature}°C` : 'N/A'}
                                    </div>
                                    <div>
                                        <span style={{ color: 'var(--text-secondary)' }}>Hours: </span>
                                        {disk.power_on_hours ? Math.floor(disk.power_on_hours).toLocaleString() : 'N/A'}
                                    </div>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            )}

            <div style={{ marginTop: '2rem' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem', borderBottom: '1px solid var(--glass-border)', paddingBottom: '0.5rem' }}>
                    <h3 style={{ margin: 0 }}>Network Speedtest</h3>
                    <button 
                        onClick={runSpeedtest} 
                        disabled={testingSpeed}
                        className="btn primary"
                        style={{ padding: '0.4rem 1rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}
                    >
                        {testingSpeed ? 'Testing...' : 'Run Speedtest'}
                    </button>
                </div>
                {speedError && <div style={{ color: 'var(--danger)', marginBottom: '1rem' }}>{speedError}</div>}
                {speedResult && (
                    <div className="glass-panel" style={{ padding: '1.5rem', display: 'flex', justifyContent: 'space-around', textAlign: 'center' }}>
                        <div>
                            <div style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', marginBottom: '0.5rem' }}>Download</div>
                            <div style={{ fontSize: '2rem', fontWeight: 'bold', color: '#34d399' }}>{speedResult.download_mbps.toFixed(2)}</div>
                            <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>Mbps</div>
                        </div>
                        <div>
                            <div style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', marginBottom: '0.5rem' }}>Upload</div>
                            <div style={{ fontSize: '2rem', fontWeight: 'bold', color: '#818cf8' }}>{speedResult.upload_mbps.toFixed(2)}</div>
                            <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>Mbps</div>
                        </div>
                        <div>
                            <div style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', marginBottom: '0.5rem' }}>Ping</div>
                            <div style={{ fontSize: '2rem', fontWeight: 'bold', color: 'var(--text-primary)' }}>{speedResult.ping_ms.toFixed(1)}</div>
                            <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>ms</div>
                        </div>
                    </div>
                )}
            </div>
        </div>
    </div>
);
}

export default SystemInfo;
