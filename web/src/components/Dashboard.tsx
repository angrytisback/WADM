import { useState, useEffect } from 'react';
import { CircularProgress } from './CircularProgress';
import { SplitCircularProgress } from './SplitCircularProgress';
import type { SystemStats } from '../types';

interface DashboardProps {
    stats: SystemStats | null;
    onNavigate: (tab: string) => void;
}



const formatUptime = (seconds: number) => {
    const days = Math.floor(seconds / (3600 * 24));
    const hours = Math.floor((seconds % (3600 * 24)) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${days}d ${hours}h ${minutes}m`;
};


const Icons = {
    Chart: () => <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><line x1="18" y1="20" x2="18" y2="10"></line><line x1="12" y1="20" x2="12" y2="4"></line><line x1="6" y1="20" x2="6" y2="14"></line></svg>,
    Activity: () => <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline></svg>,
    Box: () => <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path><polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline><line x1="12" y1="22.08" x2="12" y2="12"></line></svg>,
    Package: () => <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><line x1="16.5" y1="9.4" x2="7.5" y2="4.21"></line><path d="M21 16V8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16z"></path><polyline points="3.27 6.96 12 12.01 20.73 6.96"></polyline><line x1="12" y1="22.08" x2="12" y2="12"></line></svg>,
    Server: () => <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect x="2" y="2" width="20" height="8" rx="2" ry="2"></rect><rect x="2" y="14" width="20" height="8" rx="2" ry="2"></rect><line x1="6" y1="6" x2="6.01" y2="6"></line><line x1="6" y1="18" x2="6.01" y2="18"></line></svg>,
    Alert: () => <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="10"></circle><line x1="12" y1="8" x2="12" y2="12"></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg>,
    Check: () => <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path><polyline points="22 4 12 14.01 9 11.01"></polyline></svg>
};

export default function Dashboard({ stats, onNavigate }: DashboardProps) {
    if (!stats) return <div style={{ padding: '2rem', textAlign: 'center', color: 'var(--text-secondary)' }}>Loading System Stats...</div>;

    const memPercent = (stats.ram_used / stats.ram_total) * 100;
    const swapPercent = stats.swap_total > 0 ? (stats.swap_used / stats.swap_total) * 100 : 0;

    
    
    
    const rxRate = stats.network_rx / 2;
    const txRate = stats.network_tx / 2;

    const maxSpeed = stats.network_max_speed || 125000000; 
    const rxPercent = (rxRate / maxSpeed) * 100;
    const txPercent = (txRate / maxSpeed) * 100;



    const formatRate = (bytes: number) => {
        if (bytes === 0) return '0 B/s';
        const k = 1024;
        const sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
    };

    const networkContent = (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '2px', alignItems: 'center' }}>
            <span style={{ color: '#a78bfa', fontSize: '0.85rem' }}>↓ {formatRate(rxRate)}</span>
            <span style={{ color: '#c4b5fd', fontSize: '0.85rem' }}>↑ {formatRate(txRate)}</span>
        </div>
    );

    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '2rem', height: '100%' }}>
            {/* Main Grid */}
            <div className="dashboard-grid" style={{ gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: '1.5rem' }}>

                {/* 1. System Load */}
                <div
                    className="glass-panel"
                    onClick={() => onNavigate('usage')}
                    style={{ padding: '2rem', cursor: 'pointer', display: 'flex', flexDirection: 'column', gap: '1.5rem', gridColumn: '1 / -1' }}
                >
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                            <div style={{ padding: '0.5rem', background: 'rgba(59, 130, 246, 0.1)', borderRadius: '8px', color: 'var(--accent-color)' }}>
                                <Icons.Activity />
                            </div>
                            <h3 style={{ fontSize: '1.25rem', margin: 0, fontWeight: 600 }}>System Load</h3>
                        </div>
                        <span style={{ fontSize: '0.9rem', color: 'var(--accent-color)', fontWeight: 500 }}>View Details →</span>
                    </div>
                    <div style={{ display: 'flex', gap: '3rem', justifyContent: 'center', alignItems: 'center', flexWrap: 'wrap' }}>
                        <CircularProgress value={stats.cpu_usage} color="var(--accent-color)" size={140} strokeWidth={12} label="CPU" />
                        <CircularProgress value={memPercent} color="#34d399" size={140} strokeWidth={12} label="RAM" />
                        {stats.swap_total > 0 && (
                            <CircularProgress value={swapPercent} color="#f59e0b" size={140} strokeWidth={12} label="Swap" />
                        )}
                        <SplitCircularProgress
                            leftValue={rxPercent}
                            rightValue={txPercent}
                            size={140}
                            strokeWidth={12}
                            label="Network"
                            sublabel={stats.network_interface}
                            customValueText={networkContent}
                        />
                    </div>
                </div>

                {/* 2. Docker Containers */}
                <div
                    className="glass-panel"
                    onClick={() => onNavigate('docker')}
                    style={{ padding: '2rem', cursor: 'pointer', display: 'flex', flexDirection: 'column', gap: '1rem', alignItems: 'center', justifyContent: 'center' }}
                >
                    <div style={{ padding: '1rem', background: 'rgba(59, 130, 246, 0.1)', borderRadius: '50%', color: '#3b82f6', marginBottom: '0.5rem' }}>
                        <Icons.Box />
                    </div>
                    <div style={{ fontSize: '3.5rem', fontWeight: 700, color: 'var(--text-primary)', lineHeight: 1 }}>
                        {stats.active_containers || 0}
                    </div>
                    <div style={{ textAlign: 'center' }}>
                        <div style={{ fontSize: '1.1rem', fontWeight: 600, color: 'var(--text-primary)' }}>Docker Containers</div>
                        <div style={{ fontSize: '0.9rem', color: 'var(--text-secondary)', marginTop: '0.25rem' }}>Running Active</div>
                    </div>
                </div>

                {/* 3. Package Updates */}
                <div
                    className="glass-panel"
                    style={{ padding: '2rem', display: 'flex', flexDirection: 'column', gap: '1rem', alignItems: 'center', justifyContent: 'center', position: 'relative' }}
                >
                    <div
                        onClick={() => onNavigate('packages')}
                        style={{ cursor: 'pointer', display: 'flex', flexDirection: 'column', alignItems: 'center', width: '100%' }}
                    >
                        <div style={{ padding: '1rem', background: 'rgba(245, 158, 11, 0.1)', borderRadius: '50%', color: '#f59e0b', marginBottom: '0.5rem' }}>
                            <Icons.Package />
                        </div>
                        <div style={{ fontSize: '3.5rem', fontWeight: 700, color: 'var(--text-primary)', lineHeight: 1 }}>
                            {stats.upgradable_packages || 0}
                        </div>
                        <div style={{ textAlign: 'center' }}>
                            <div style={{ fontSize: '1.1rem', fontWeight: 600, color: 'var(--text-primary)' }}>Package Updates</div>
                            <div style={{ fontSize: '0.9rem', color: 'var(--text-secondary)', marginTop: '0.25rem' }}>Available to Install</div>
                        </div>
                    </div>
                    {stats.upgradable_packages > 0 && (
                        <button
                            className="btn-primary"
                            style={{ marginTop: '0.5rem', width: '80%', display: 'flex', justifyContent: 'center', alignItems: 'center', gap: '0.5rem' }}
                            onClick={(e) => {
                                e.stopPropagation();
                                fetch('/api/packages/update-all', { method: 'POST' })
                                    .then(res => {
                                        if (res.ok) alert('System update started. This may take a while.');
                                        else alert('Failed to start update.');
                                    })
                                    .catch(() => alert('Network error'));
                            }}
                        >
                            Update All
                        </button>
                    )}
                </div>

                {/* 4. Active Services */}
                <div
                    className="glass-panel"
                    onClick={() => onNavigate('services')}
                    style={{ padding: '2rem', cursor: 'pointer', display: 'flex', flexDirection: 'column', gap: '1rem', alignItems: 'center', justifyContent: 'center' }}
                >
                    <div style={{ padding: '1rem', background: 'rgba(16, 185, 129, 0.1)', borderRadius: '50%', color: '#10b981', marginBottom: '0.5rem' }}>
                        <Icons.Server />
                    </div>
                    <div style={{ fontSize: '3.5rem', fontWeight: 700, color: 'var(--text-primary)', lineHeight: 1 }}>
                        {stats.active_services || 0}
                    </div>
                    <div style={{ textAlign: 'center' }}>
                        <div style={{ fontSize: '1.1rem', fontWeight: 600, color: 'var(--text-primary)' }}>Active Services</div>
                        <div style={{ fontSize: '0.9rem', color: 'var(--success)', marginTop: '0.25rem', fontWeight: 500 }}>System Operational</div>
                    </div>
                </div>

                {/* 5. System Health */}
                <div
                    className="glass-panel"
                    onClick={() => onNavigate('services')}
                    style={{ padding: '2rem', cursor: 'pointer', display: 'flex', flexDirection: 'column', gap: '1rem', alignItems: 'center', justifyContent: 'center' }}
                >
                    <div style={{ padding: '1rem', background: stats.failed_services > 0 ? 'rgba(239, 68, 68, 0.1)' : 'rgba(16, 185, 129, 0.1)', borderRadius: '50%', color: stats.failed_services > 0 ? '#ef4444' : '#10b981', marginBottom: '0.5rem' }}>
                        {stats.failed_services > 0 ? <Icons.Alert /> : <Icons.Check />}
                    </div>
                    <div style={{ fontSize: '3.5rem', fontWeight: 700, color: stats.failed_services > 0 ? 'var(--danger)' : 'var(--success)', lineHeight: 1 }}>
                        {stats.failed_services || 0}
                    </div>
                    <div style={{ textAlign: 'center' }}>
                        <div style={{ fontSize: '1.1rem', fontWeight: 600, color: 'var(--text-primary)' }}>Critical Errors</div>
                        <div style={{ fontSize: '0.9rem', color: 'var(--text-secondary)', marginTop: '0.25rem' }}>
                            {stats.failed_services > 0 ? 'Attention Needed' : 'No Issues Found'}
                        </div>
                    </div>
                </div>

            </div>

            {/* Footer / System Info Area */}
            <div style={{ marginTop: 'auto', paddingTop: '2rem' }}>
                <DashboardFooter />
            </div>
        </div>
    );
}

function DashboardFooter() {
    const [info, setInfo] = useState<any>(null);

    useEffect(() => {
        fetch('/api/system')
            .then(res => res.json())
            .then(setInfo)
            .catch(() => { });
    }, []);

    if (!info) return null;

    return (
        <div className="glass-panel" style={{ padding: '1.5rem', display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '1.5rem' }}>
            <div>
                <div style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginBottom: '0.2rem' }}>HOSTNAME</div>
                <div style={{ fontWeight: 600, fontSize: '1.1rem' }}>{info.host_name}</div>
            </div>
            <div>
                <div style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginBottom: '0.2rem' }}>OS</div>
                <div style={{ fontWeight: 500 }}>{info.os_name} {info.os_version}</div>
            </div>
            <div>
                <div style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginBottom: '0.2rem' }}>KERNEL</div>
                <div style={{ fontFamily: 'monospace' }}>{info.kernel_version}</div>
            </div>
            <div>
                <div style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginBottom: '0.2rem' }}>UPTIME</div>
                <div style={{ fontWeight: 500 }}>{formatUptime(info.uptime)}</div>
            </div>
        </div>
    );
}
