import { useState, useEffect, useRef } from 'react';
import { XAxis, YAxis, Tooltip, ResponsiveContainer, AreaChart, Area } from 'recharts';
import type { SystemStats } from '../types';
import { useToast } from '../context/ToastContext';

interface ProcessInfo {
    pid: number;
    name: string;
    cpu_usage: number;
    memory: number;
}

interface PerformanceData {
    time: string;
    cpu: number;
    memory: number;
    swap: number;
    network_rx: number;
    network_tx: number;
}


interface TooltipPayload {
    value: number;
    name: string;
    color: string;
}

interface TooltipProps {
    active?: boolean;
    payload?: TooltipPayload[];
}

const CustomPercentTooltip = ({ active, payload }: TooltipProps) => {
    if (active && payload && payload.length) {
        return (
            <div className="custom-tooltip" style={{
                backgroundColor: 'var(--glass-bg)',
                border: '1px solid var(--glass-border)',
                padding: '0.4rem 0.8rem',
                borderRadius: '6px',
                backdropFilter: 'blur(10px)',
            }}>
                <span style={{ fontWeight: 500, color: 'var(--text-primary)' }}>
                    {payload[0].value.toFixed(1)}%
                </span>
            </div>
        );
    }
    return null;
};


const CustomNetworkTooltip = ({ active, payload }: TooltipProps) => {
    if (active && payload && payload.length) {
        return (
            <div className="custom-tooltip" style={{
                backgroundColor: 'var(--glass-bg)',
                border: '1px solid var(--glass-border)',
                padding: '0.5rem',
                borderRadius: '6px',
                backdropFilter: 'blur(10px)',
                display: 'flex',
                flexDirection: 'column',
                gap: '0.2rem'
            }}>
                {payload.map((entry, index) => (
                    <div key={index} style={{ color: entry.color, fontSize: '0.9rem' }}>
                        {entry.name}: {entry.value.toFixed(1)} KB/s
                    </div>
                ))}
            </div>
        );
    }
    return null;
};

export default function SystemUsage() {
    const [processes, setProcesses] = useState<ProcessInfo[]>([]);
    const [perfHistory, setPerfHistory] = useState<PerformanceData[]>(() =>
        Array(60).fill({ time: '', cpu: 0, memory: 0, swap: 0, network_rx: 0, network_tx: 0 })
    );
    const [activeTab, setActiveTab] = useState<'performance' | 'processes'>('performance');
    const { addToast } = useToast();
    const [killing, setKilling] = useState<number | null>(null);
    const lastNetworkRef = useRef<{ rx: number, tx: number, time: number } | null>(null);

    useEffect(() => {
        const fetchData = async () => {
            try {
                if (activeTab === 'processes') {
                    const procRes = await fetch('/api/processes');
                    if (procRes.ok) {
                        const procData = await procRes.json();
                        setProcesses(procData);
                    }
                }

                
                const statsRes = await fetch('/api/stats');
                if (statsRes.ok) {
                    const stats: SystemStats = await statsRes.json();

                    const now = Date.now();
                    let rxRate = 0;
                    let txRate = 0;

                    if (lastNetworkRef.current) {
                        const timeDelta = (now - lastNetworkRef.current.time) / 1000;
                        if (timeDelta > 0) {
                            const rxDelta = stats.network_rx - lastNetworkRef.current.rx;
                            const txDelta = stats.network_tx - lastNetworkRef.current.tx;
                            if (rxDelta >= 0 && txDelta >= 0) {
                                rxRate = rxDelta / timeDelta;
                                txRate = txDelta / timeDelta;
                            }
                        }
                    }
                    lastNetworkRef.current = { rx: stats.network_rx, tx: stats.network_tx, time: now };

                    setPerfHistory(prev => {
                        const newData = [
                            ...prev,
                            {
                                time: new Date().toLocaleTimeString(),
                                cpu: stats.cpu_usage,
                                memory: (stats.ram_used / stats.ram_total) * 100,
                                swap: stats.swap_total > 0 ? (stats.swap_used / stats.swap_total) * 100 : 0,
                                network_rx: rxRate / 1024, 
                                network_tx: txRate / 1024  
                            }
                        ];
                        
                        if (newData.length > 60) newData.shift();
                        return newData;
                    });
                }
            } catch {
                
            }
        };

        fetchData();
        const interval = setInterval(fetchData, 2000);
        return () => clearInterval(interval);
    }, [activeTab]);

    const formatBytes = (bytes: number) => {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    };

    const killProcess = async (pid: number, signal: 'SIGTERM' | 'SIGKILL') => {
        if (!confirm(`Are you sure you want to ${signal === 'SIGKILL' ? 'force ' : ''}kill process ${pid}?`)) return;

        setKilling(pid);
        try {
            const res = await fetch('/api/processes/kill', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ pid, signal })
            });

            if (res.ok) {
                addToast(`Process ${pid} killed (${signal})`, 'success');
                
                const procRes = await fetch('/api/processes');
                if (procRes.ok) setProcesses(await procRes.json());
            } else {
                const err = await res.json();
                addToast(`Failed to kill process: ${err}`, 'error');
            }
        } catch {
            addToast('Failed to execute kill command', 'error');
        } finally {
            setKilling(null);
        }
    };

    return (
        <div className="fade-in" style={{ padding: '0 1rem' }}>
            {/* Improved Tab UI */}
            <div style={{ marginBottom: '2rem', display: 'flex', gap: '1rem', borderBottom: '1px solid var(--glass-border)' }}>
                <button
                    className={activeTab === 'performance' ? 'nav-link active' : 'nav-link'}
                    onClick={() => setActiveTab('performance')}
                    style={{
                        background: activeTab === 'performance' ? 'rgba(56, 189, 248, 0.1)' : 'transparent',
                        border: 'none',
                        borderBottom: activeTab === 'performance' ? '2px solid var(--accent-color)' : '2px solid transparent',
                        borderRadius: '8px 8px 0 0',
                        color: activeTab === 'performance' ? 'var(--accent-color)' : 'var(--text-secondary)',
                        cursor: 'pointer',
                        padding: '1rem 1.5rem',
                        fontSize: '1rem',
                        fontWeight: 500,
                        transition: 'all 0.2s',
                        outline: 'none'
                    }}
                >
                    Performance Metrics
                </button>
                <button
                    className={activeTab === 'processes' ? 'nav-link active' : 'nav-link'}
                    onClick={() => setActiveTab('processes')}
                    style={{
                        background: activeTab === 'processes' ? 'rgba(56, 189, 248, 0.1)' : 'transparent',
                        border: 'none',
                        borderBottom: activeTab === 'processes' ? '2px solid var(--accent-color)' : '2px solid transparent',
                        borderRadius: '8px 8px 0 0',
                        color: activeTab === 'processes' ? 'var(--accent-color)' : 'var(--text-secondary)',
                        cursor: 'pointer',
                        padding: '1rem 1.5rem',
                        fontSize: '1rem',
                        fontWeight: 500,
                        transition: 'all 0.2s',
                        outline: 'none'
                    }}
                >
                    Running Processes
                </button>
            </div>

            {activeTab === 'performance' && (
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(450px, 1fr))', gap: '2rem' }}>
                    {/* CPU Chart */}
                    <div className="glass-panel" style={{ padding: '1.5rem', height: '350px' }}>
                        <h3 style={{ marginBottom: '1rem', color: 'var(--text-secondary)', fontSize: '1rem' }}>CPU Usage</h3>
                        <ResponsiveContainer width="100%" height="90%">
                            <AreaChart data={perfHistory}>
                                <defs>
                                    <linearGradient id="colorCpu" x1="0" y1="0" x2="0" y2="1">
                                        <stop offset="5%" stopColor="#8884d8" stopOpacity={0.5} />
                                        <stop offset="95%" stopColor="#8884d8" stopOpacity={0} />
                                    </linearGradient>
                                </defs>
                                <XAxis dataKey="time" hide />
                                <YAxis domain={[0, 100]} hide />
                                <Tooltip content={<CustomPercentTooltip />} cursor={{ stroke: 'rgba(255,255,255,0.1)' }} />
                                <Area type="monotone" dataKey="cpu" stroke="#8884d8" fillOpacity={1} fill="url(#colorCpu)" isAnimationActive={false} />
                            </AreaChart>
                        </ResponsiveContainer>
                    </div>

                    {/* Memory Chart */}
                    <div className="glass-panel" style={{ padding: '1.5rem', height: '350px' }}>
                        <h3 style={{ marginBottom: '1rem', color: 'var(--text-secondary)', fontSize: '1rem' }}>Memory Usage</h3>
                        <ResponsiveContainer width="100%" height="90%">
                            <AreaChart data={perfHistory}>
                                <defs>
                                    <linearGradient id="colorRam" x1="0" y1="0" x2="0" y2="1">
                                        <stop offset="5%" stopColor="#82ca9d" stopOpacity={0.5} />
                                        <stop offset="95%" stopColor="#82ca9d" stopOpacity={0} />
                                    </linearGradient>
                                </defs>
                                <XAxis dataKey="time" hide />
                                <YAxis domain={[0, 100]} hide />
                                <Tooltip content={<CustomPercentTooltip />} cursor={{ stroke: 'rgba(255,255,255,0.1)' }} />
                                <Area type="monotone" dataKey="memory" stroke="#82ca9d" fillOpacity={1} fill="url(#colorRam)" isAnimationActive={false} />
                            </AreaChart>
                        </ResponsiveContainer>
                    </div>

                    {/* Swap Chart */}
                    <div className="glass-panel" style={{ padding: '1.5rem', height: '350px' }}>
                        <h3 style={{ marginBottom: '1rem', color: 'var(--text-secondary)', fontSize: '1rem' }}>Swap Usage</h3>
                        <ResponsiveContainer width="100%" height="90%">
                            <AreaChart data={perfHistory}>
                                <defs>
                                    <linearGradient id="colorSwap" x1="0" y1="0" x2="0" y2="1">
                                        <stop offset="5%" stopColor="#fab1a0" stopOpacity={0.5} />
                                        <stop offset="95%" stopColor="#fab1a0" stopOpacity={0} />
                                    </linearGradient>
                                </defs>
                                <XAxis dataKey="time" hide />
                                <YAxis domain={[0, 100]} hide />
                                <Tooltip content={<CustomPercentTooltip />} cursor={{ stroke: 'rgba(255,255,255,0.1)' }} />
                                <Area type="monotone" dataKey="swap" stroke="#fab1a0" fillOpacity={1} fill="url(#colorSwap)" isAnimationActive={false} />
                            </AreaChart>
                        </ResponsiveContainer>
                    </div>

                    {/* Network Chart */}
                    <div className="glass-panel" style={{ padding: '1.5rem', height: '350px' }}>
                        <h3 style={{ marginBottom: '1rem', color: 'var(--text-secondary)', fontSize: '1rem' }}>Network I/O (KB/s)</h3>
                        <ResponsiveContainer width="100%" height="90%">
                            <AreaChart data={perfHistory}>
                                <defs>
                                    <linearGradient id="colorRx" x1="0" y1="0" x2="0" y2="1">
                                        <stop offset="5%" stopColor="#74b9ff" stopOpacity={0.5} />
                                        <stop offset="95%" stopColor="#74b9ff" stopOpacity={0} />
                                    </linearGradient>
                                    <linearGradient id="colorTx" x1="0" y1="0" x2="0" y2="1">
                                        <stop offset="5%" stopColor="#a29bfe" stopOpacity={0.5} />
                                        <stop offset="95%" stopColor="#a29bfe" stopOpacity={0} />
                                    </linearGradient>
                                </defs>
                                <XAxis dataKey="time" hide />
                                <YAxis hide />
                                <Tooltip content={<CustomNetworkTooltip />} cursor={{ stroke: 'rgba(255,255,255,0.1)' }} />
                                <Area type="monotone" dataKey="network_rx" name="Download" stroke="#74b9ff" fillOpacity={1} fill="url(#colorRx)" isAnimationActive={false} stackId="1" />
                                <Area type="monotone" dataKey="network_tx" name="Upload" stroke="#a29bfe" fillOpacity={1} fill="url(#colorTx)" isAnimationActive={false} stackId="1" />
                            </AreaChart>
                        </ResponsiveContainer>
                    </div>
                </div>
            )}

            {activeTab === 'processes' && (
                <div className="glass-panel" style={{ overflowX: 'auto', borderRadius: '12px' }}>
                    <table style={{ width: '100%', borderCollapse: 'separate', borderSpacing: '0', textAlign: 'left' }}>
                        <thead>
                            <tr style={{ background: 'rgba(255,255,255,0.02)' }}>
                                <th style={{ padding: '1.2rem', color: 'var(--text-secondary)', fontWeight: 500, borderBottom: '1px solid var(--glass-border)' }}>Name</th>
                                <th style={{ padding: '1.2rem', color: 'var(--text-secondary)', fontWeight: 500, borderBottom: '1px solid var(--glass-border)' }}>PID</th>
                                <th style={{ padding: '1.2rem', color: 'var(--text-secondary)', fontWeight: 500, borderBottom: '1px solid var(--glass-border)' }}>CPU</th>
                                <th style={{ padding: '1.2rem', color: 'var(--text-secondary)', fontWeight: 500, borderBottom: '1px solid var(--glass-border)' }}>Memory</th>
                                <th style={{ padding: '1.2rem', color: 'var(--text-secondary)', fontWeight: 500, borderBottom: '1px solid var(--glass-border)' }}>Actions</th>
                            </tr>
                        </thead>
                        <tbody>
                            {processes.map((proc) => (
                                <tr key={proc.pid} className="table-row-hover">
                                    <td style={{ padding: '1rem 1.2rem', fontWeight: 500, borderBottom: '1px solid rgba(255,255,255,0.03)' }}>{proc.name}</td>
                                    <td style={{ padding: '1rem 1.2rem', fontFamily: 'monospace', color: 'var(--text-secondary)', borderBottom: '1px solid rgba(255,255,255,0.03)' }}>{proc.pid}</td>
                                    <td style={{ padding: '1rem 1.2rem', borderBottom: '1px solid rgba(255,255,255,0.03)' }}>
                                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                            <div style={{ width: '60px', height: '4px', background: 'rgba(255,255,255,0.1)', borderRadius: '2px' }}>
                                                <div style={{ width: `${Math.min(proc.cpu_usage, 100)}%`, height: '100%', background: 'var(--accent-color)', borderRadius: '2px' }}></div>
                                            </div>
                                            {proc.cpu_usage.toFixed(1)}%
                                        </div>
                                    </td>
                                    <td style={{ padding: '1rem 1.2rem', borderBottom: '1px solid rgba(255,255,255,0.03)' }}>{formatBytes(proc.memory)}</td>
                                    <td style={{ padding: '1rem 1.2rem', borderBottom: '1px solid rgba(255,255,255,0.03)', display: 'flex', gap: '0.5rem' }}>
                                        <button
                                            className="btn-sm danger"
                                            onClick={() => killProcess(proc.pid, 'SIGTERM')}
                                            disabled={killing === proc.pid}
                                            title="Terminate (SIGTERM)"
                                            style={{ padding: '0.3rem 0.6rem', fontSize: '0.8rem' }}
                                        >
                                            Kill
                                        </button>
                                        <button
                                            className="btn-sm danger-outline"
                                            onClick={() => killProcess(proc.pid, 'SIGKILL')}
                                            disabled={killing === proc.pid}
                                            title="Force Kill (SIGKILL -9)"
                                            style={{ padding: '0.3rem 0.6rem', fontSize: '0.8rem', border: '1px solid var(--danger)', background: 'transparent', color: 'var(--danger)' }}
                                        >
                                            -9
                                        </button>
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
