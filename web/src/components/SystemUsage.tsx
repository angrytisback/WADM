import { useState, useEffect } from 'react';
import { XAxis, YAxis, Tooltip, ResponsiveContainer, AreaChart, Area } from 'recharts';
import { useToast } from '../context/ToastContext';
import { useStats, type PerformanceData } from '../context/StatsContext';
import { useModal } from '../context/ModalContext';
import { FaMemory, FaBroom } from 'react-icons/fa';


interface ProcessInfo {
    pid: number;
    name: string;
    cpu_usage: number;
    memory: number;
}

interface TooltipPayload {
    value: number;
    name: string;
    color: string;
    payload: PerformanceData;
}

interface TooltipProps {
    active?: boolean;
    payload?: TooltipPayload[];
    label?: string;
}

const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

const CustomPercentTooltip = ({ active, payload }: TooltipProps) => {
    if (active && payload && payload.length) {
        const data = payload[0].payload;
        const entry = payload[0];
        const name = entry.name;
        const value = entry.value;
        const color = entry.color;
        
        let detail = '';
        let label = name;
        
        if (name === 'cpu') {
            label = 'CPU Usage';
        } else if (name === 'memory') {
            label = 'RAM Usage';
            detail = `(${formatBytes(data.ram_used)} / ${formatBytes(data.ram_total)})`;
        } else if (name === 'swap') {
            label = 'Swap Usage';
            detail = `(${formatBytes(data.swap_used)} / ${formatBytes(data.swap_total)})`;
        } else if (name.startsWith('gpu_')) {
            const parts = name.split('_');
            const index = parseInt(parts[1]);
            const isVram = parts[2] === 'vram';
            const gpu = data.gpus[index];
            if (gpu) {
                label = `${gpu.name} ${isVram ? 'VRAM' : 'Load'}`;
                if (isVram) {
                    detail = `(${formatBytes(gpu.vram_used)} / ${formatBytes(gpu.vram_total)})`;
                }
            }
        }

        return (
            <div className="custom-tooltip" style={{
                backgroundColor: 'rgba(20, 20, 30, 0.95)',
                border: '1px solid var(--glass-border)',
                padding: '0.6rem 0.8rem',
                borderRadius: '8px',
                backdropFilter: 'blur(12px)',
                boxShadow: '0 8px 24px rgba(0,0,0,0.4)',
                fontSize: '0.85rem'
            }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '0.4rem' }}>
                    <div style={{ width: '8px', height: '8px', borderRadius: '50%', backgroundColor: color }}></div>
                    <span style={{ fontWeight: 600, color: 'var(--text-secondary)' }}>{label}</span>
                </div>
                <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'baseline' }}>
                    <span style={{ fontSize: '1.2rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                        {value.toFixed(1)}%
                    </span>
                    {detail && <span style={{ color: 'var(--text-secondary)', fontSize: '0.75rem', opacity: 0.8 }}>{detail}</span>}
                </div>
            </div>
        );
    }
    return null;
};

const CustomTempTooltip = ({ active, payload }: TooltipProps) => {
    if (active && payload && payload.length) {
        return (
            <div className="custom-tooltip" style={{
                backgroundColor: 'rgba(20, 20, 30, 0.95)',
                border: '1px solid var(--glass-border)',
                padding: '0.6rem 0.8rem',
                borderRadius: '8px',
                backdropFilter: 'blur(12px)',
                boxShadow: '0 8px 24px rgba(0,0,0,0.4)',
                display: 'flex',
                flexDirection: 'column',
                gap: '0.4rem'
            }}>
                <div style={{ fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-secondary)', marginBottom: '0.2rem' }}>Temperatures</div>
                {payload.map((entry, index) => (
                    <div key={index} style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.9rem' }}>
                        <div style={{ width: '8px', height: '8px', borderRadius: '50%', backgroundColor: entry.color }}></div>
                        <span style={{ color: 'var(--text-primary)' }}>{entry.name}:</span>
                        <span style={{ fontWeight: 700, color: entry.value > 80 ? 'var(--danger)' : 'var(--text-primary)' }}>
                            {entry.value.toFixed(1)}°C
                        </span>
                    </div>
                ))}
            </div>
        );
    }
    return null;
};

const CustomNetworkTooltip = ({ active, payload }: TooltipProps) => {
    if (active && payload && payload.length) {
        return (
            <div className="custom-tooltip" style={{
                backgroundColor: 'rgba(20, 20, 30, 0.95)',
                border: '1px solid var(--glass-border)',
                padding: '0.6rem 0.8rem',
                borderRadius: '8px',
                backdropFilter: 'blur(12px)',
                boxShadow: '0 8px 24px rgba(0,0,0,0.4)',
                display: 'flex',
                flexDirection: 'column',
                gap: '0.4rem'
            }}>
                <div style={{ fontSize: '0.8rem', fontWeight: 600, color: 'var(--text-secondary)', marginBottom: '0.2rem' }}>Network I/O</div>
                {payload.map((entry, index) => (
                    <div key={index} style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.9rem' }}>
                        <div style={{ width: '8px', height: '8px', borderRadius: '50%', backgroundColor: entry.color }}></div>
                        <span style={{ color: 'var(--text-primary)' }}>{entry.name}:</span>
                        <span style={{ fontWeight: 600, fontFamily: 'monospace' }}>{entry.value.toFixed(1)} KB/s</span>
                    </div>
                ))}
            </div>
        );
    }
    return null;
};

export default function SystemUsage() {
    const { confirm } = useModal();
    const { history } = useStats();
    const [processes, setProcesses] = useState<ProcessInfo[]>([]);
    const [activeTab, setActiveTab] = useState<'performance' | 'processes'>('performance');
    const { addToast } = useToast();
    const [killing, setKilling] = useState<number | null>(null);
    const [flushing, setFlushing] = useState(false);

    const handleFlushMemory = async (action: 'memory_flush' | 'cache_clean') => {
        setFlushing(true);
        try {
            const res = await fetch('/api/system/maintenance', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ action })
            });
            const data = await res.json();
            if (res.ok && data.success !== false) {
                addToast(data.message || 'Memory flushed successfully', 'success');
            } else {
                addToast(data.message || 'Operation failed', 'error');
            }
        } catch {
            addToast('Network error during maintenance action', 'error');
        } finally {
            setFlushing(false);
        }
    };

    useEffect(() => {
        if (activeTab !== 'processes') return;

        const fetchProcesses = async () => {
            try {
                const procRes = await fetch('/api/processes');
                if (procRes.ok) {
                    const procData = await procRes.json();
                    setProcesses(procData);
                }
            } catch (e) {
                // ignore
            }
        };

        fetchProcesses();
        const interval = setInterval(fetchProcesses, 2000);
        return () => clearInterval(interval);
    }, [activeTab]);

    const killProcess = async (pid: number, signal: 'SIGTERM' | 'SIGKILL') => {
        const ok = await confirm({
            title: 'Kill Process',
            message: `Are you sure you want to ${signal === 'SIGKILL' ? 'force ' : ''}kill process ${pid}?`,
            type: 'danger',
            confirmText: 'Kill'
        });
        if (!ok) return;

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
                            <AreaChart data={history}>
                                <defs>
                                    <linearGradient id="colorCpu" x1="0" y1="0" x2="0" y2="1">
                                        <stop offset="5%" stopColor="#8884d8" stopOpacity={0.5} />
                                        <stop offset="95%" stopColor="#8884d8" stopOpacity={0} />
                                    </linearGradient>
                                </defs>
                                <XAxis dataKey="time" hide />
                                <YAxis domain={[0, 100]} hide />
                                <Tooltip content={<CustomPercentTooltip />} cursor={{ stroke: 'rgba(255,255,255,0.1)' }} />
                                <Area type="monotone" dataKey="cpu" name="cpu" stroke="#8884d8" fillOpacity={1} fill="url(#colorCpu)" isAnimationActive={false} />
                            </AreaChart>
                        </ResponsiveContainer>
                    </div>

                    {/* Memory Chart */}
                    <div className="glass-panel" style={{ padding: '1.5rem', height: '350px' }}>
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
                            <h3 style={{ margin: 0, color: 'var(--text-secondary)', fontSize: '1rem' }}>Memory Usage</h3>
                            <div style={{ display: 'flex', gap: '0.5rem' }}>
                                <button
                                    onClick={() => handleFlushMemory('memory_flush')}
                                    disabled={flushing}
                                    style={{
                                        display: 'flex',
                                        alignItems: 'center',
                                        gap: '0.35rem',
                                        background: 'rgba(52, 211, 153, 0.15)',
                                        color: '#34d399',
                                        border: '1px solid rgba(52, 211, 153, 0.3)',
                                        borderRadius: '6px',
                                        padding: '0.3rem 0.6rem',
                                        fontSize: '0.75rem',
                                        cursor: flushing ? 'not-allowed' : 'pointer',
                                        fontWeight: 600
                                    }}
                                    title="Flush RAM / Drop PageCache & Inodes"
                                >
                                    <FaMemory size={12} />
                                    {flushing ? 'Flushing...' : 'Flush RAM'}
                                </button>
                                <button
                                    onClick={() => handleFlushMemory('cache_clean')}
                                    disabled={flushing}
                                    style={{
                                        display: 'flex',
                                        alignItems: 'center',
                                        gap: '0.35rem',
                                        background: 'rgba(56, 189, 248, 0.15)',
                                        color: 'var(--accent-color)',
                                        border: '1px solid rgba(56, 189, 248, 0.3)',
                                        borderRadius: '6px',
                                        padding: '0.3rem 0.6rem',
                                        fontSize: '0.75rem',
                                        cursor: flushing ? 'not-allowed' : 'pointer',
                                        fontWeight: 600
                                    }}
                                    title="Clean Package Archives & System Journal Caches"
                                >
                                    <FaBroom size={12} />
                                    Clean Cache
                                </button>
                            </div>
                        </div>
                        <ResponsiveContainer width="100%" height="85%">
                            <AreaChart data={history}>
                                <defs>
                                    <linearGradient id="colorRam" x1="0" y1="0" x2="0" y2="1">
                                        <stop offset="5%" stopColor="#82ca9d" stopOpacity={0.5} />
                                        <stop offset="95%" stopColor="#82ca9d" stopOpacity={0} />
                                    </linearGradient>
                                </defs>
                                <XAxis dataKey="time" hide />
                                <YAxis domain={[0, 100]} hide />
                                <Tooltip content={<CustomPercentTooltip />} cursor={{ stroke: 'rgba(255,255,255,0.1)' }} />
                                <Area type="monotone" dataKey="memory" name="memory" stroke="#82ca9d" fillOpacity={1} fill="url(#colorRam)" isAnimationActive={false} />
                            </AreaChart>
                        </ResponsiveContainer>
                    </div>

                    {/* Temperature Chart */}
                    <div className="glass-panel" style={{ padding: '1.5rem', height: '350px' }}>
                        <h3 style={{ marginBottom: '1rem', color: 'var(--text-secondary)', fontSize: '1rem' }}>Temperatures (°C)</h3>
                        <ResponsiveContainer width="100%" height="90%">
                            <AreaChart data={history}>
                                <defs>
                                    <linearGradient id="colorTemp" x1="0" y1="0" x2="0" y2="1">
                                        <stop offset="5%" stopColor="#ff7675" stopOpacity={0.5} />
                                        <stop offset="95%" stopColor="#ff7675" stopOpacity={0} />
                                    </linearGradient>
                                </defs>
                                <XAxis dataKey="time" hide />
                                <YAxis domain={[0, 100]} hide />
                                <Tooltip content={<CustomTempTooltip />} cursor={{ stroke: 'rgba(255,255,255,0.1)' }} />
                                <Area type="monotone" dataKey="cpu_temp" name="CPU" stroke="#ff7675" fillOpacity={1} fill="url(#colorTemp)" isAnimationActive={false} />
                                {history[history.length - 1]?.gpus.map((gpu, idx) => (
                                    <Area 
                                        key={idx}
                                        type="monotone" 
                                        dataKey={(d: PerformanceData) => d.gpus[idx]?.temp || 0} 
                                        name={`${gpu.name}`}
                                        stroke="#fab1a0" 
                                        fill="transparent"
                                        isAnimationActive={false} 
                                    />
                                ))}
                            </AreaChart>
                        </ResponsiveContainer>
                    </div>

                    {/* Network Chart */}
                    <div className="glass-panel" style={{ padding: '1.5rem', height: '350px' }}>
                        <h3 style={{ marginBottom: '1rem', color: 'var(--text-secondary)', fontSize: '1rem' }}>Network I/O (KB/s)</h3>
                        <ResponsiveContainer width="100%" height="90%">
                            <AreaChart data={history}>
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

                    {/* Dynamic GPU Load/VRAM Charts */}
                    {history[history.length - 1]?.gpus.map((gpu, idx) => (
                        <div key={idx} style={{ display: 'contents' }}>
                            <div className="glass-panel" style={{ padding: '1.5rem', height: '350px' }}>
                                <h3 style={{ marginBottom: '1rem', color: 'var(--text-secondary)', fontSize: '1rem' }}>{gpu.name} Load</h3>
                                <ResponsiveContainer width="100%" height="90%">
                                    <AreaChart data={history}>
                                        <defs>
                                            <linearGradient id={`colorGpuLoad${idx}`} x1="0" y1="0" x2="0" y2="1">
                                                <stop offset="5%" stopColor="#fdcb6e" stopOpacity={0.5} />
                                                <stop offset="95%" stopColor="#fdcb6e" stopOpacity={0} />
                                            </linearGradient>
                                        </defs>
                                        <XAxis dataKey="time" hide />
                                        <YAxis domain={[0, 100]} hide />
                                        <Tooltip content={<CustomPercentTooltip />} cursor={{ stroke: 'rgba(255,255,255,0.1)' }} />
                                        <Area 
                                            type="monotone" 
                                            dataKey={(d: PerformanceData) => d.gpus[idx]?.load || 0} 
                                            name={`gpu_${idx}_load`}
                                            stroke="#fdcb6e" 
                                            fillOpacity={1} 
                                            fill={`url(#colorGpuLoad${idx})`} 
                                            isAnimationActive={false} 
                                        />
                                    </AreaChart>
                                </ResponsiveContainer>
                            </div>

                            <div className="glass-panel" style={{ padding: '1.5rem', height: '350px' }}>
                                <h3 style={{ marginBottom: '1rem', color: 'var(--text-secondary)', fontSize: '1rem' }}>{gpu.name} VRAM</h3>
                                <ResponsiveContainer width="100%" height="90%">
                                    <AreaChart data={history}>
                                        <defs>
                                            <linearGradient id={`colorGpuVram${idx}`} x1="0" y1="0" x2="0" y2="1">
                                                <stop offset="5%" stopColor="#00b894" stopOpacity={0.5} />
                                                <stop offset="95%" stopColor="#00b894" stopOpacity={0} />
                                            </linearGradient>
                                        </defs>
                                        <XAxis dataKey="time" hide />
                                        <YAxis domain={[0, 100]} hide />
                                        <Tooltip content={<CustomPercentTooltip />} cursor={{ stroke: 'rgba(255,255,255,0.1)' }} />
                                        <Area 
                                            type="monotone" 
                                            dataKey={(d: PerformanceData) => d.gpus[idx]?.vram_percent || 0} 
                                            name={`gpu_${idx}_vram`}
                                            stroke="#00b894" 
                                            fillOpacity={1} 
                                            fill={`url(#colorGpuVram${idx})`} 
                                            isAnimationActive={false} 
                                        />
                                    </AreaChart>
                                </ResponsiveContainer>
                            </div>
                        </div>
                    ))}
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
