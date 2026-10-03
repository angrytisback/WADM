import { useState, useEffect } from 'react';
import { useToast } from '../context/ToastContext';
import { useStats } from '../context/StatsContext';
import { FaTools, FaClock, FaShieldAlt, FaInfoCircle } from 'react-icons/fa';

interface Config {
    developer_mode: boolean;
}

export default function Settings() {
    const [config, setConfig] = useState<Config | null>(null);
    const [loading, setLoading] = useState(false);
    const { addToast } = useToast();
    const { refreshInterval, setRefreshInterval } = useStats();

    useEffect(() => {
        fetch('/api/config')
            .then(res => res.json())
            .then(setConfig)
            .catch(() => addToast('Failed to load settings', 'error'));
    }, [addToast]);

    const toggleDevMode = async () => {
        if (!config) return;
        setLoading(true);
        try {
            const res = await fetch('/api/config', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ developer_mode: !config.developer_mode }),
            });
            if (res.ok) {
                const newConfig = await res.json();
                setConfig(newConfig);
                addToast(
                    `Developer Mode ${newConfig.developer_mode ? 'Enabled' : 'Disabled'}`,
                    'success'
                );
            } else {
                throw new Error('Failed to update');
            }
        } catch {
            addToast('Failed to update settings', 'error');
        } finally {
            setLoading(false);
        }
    };

    if (!config) return <div style={{ padding: '2rem' }}>Loading settings...</div>;

    return (
        <div style={{ padding: '2rem', maxWidth: '800px', margin: '0 auto', display: 'flex', flexDirection: 'column', gap: '2rem' }}>
            <h1 style={{ fontSize: '2rem', fontWeight: 700, margin: 0 }}>System Settings</h1>

            {/* Performance & Polling Interval */}
            <div className="glass-panel" style={{ padding: '2rem' }}>
                <h2 style={{ fontSize: '1.25rem', fontWeight: 600, marginBottom: '1.5rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <FaClock /> Metric Refresh Frequency
                </h2>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                    <p style={{ color: 'var(--text-secondary)', margin: 0, fontSize: '0.9rem', lineHeight: 1.5 }}>
                        Select how frequently the dashboard collects and streams CPU, RAM, and network statistics. Lower intervals provide real-time resolution, while higher intervals conserve CPU on low-power devices.
                    </p>
                    <div style={{ display: 'flex', gap: '0.75rem', marginTop: '0.5rem' }}>
                        {[
                            { label: '1s (High Precision)', value: 1000 },
                            { label: '2s (Default)', value: 2000 },
                            { label: '5s (Resource Saver)', value: 5000 },
                        ].map((option) => (
                            <button
                                key={option.value}
                                onClick={() => {
                                    setRefreshInterval(option.value);
                                    addToast(`Refresh rate set to ${option.label}`, 'info');
                                }}
                                style={{
                                    padding: '0.6rem 1.2rem',
                                    borderRadius: '8px',
                                    border: '1px solid var(--border-color)',
                                    background: refreshInterval === option.value ? 'var(--accent-color, #3b82f6)' : 'rgba(255,255,255,0.05)',
                                    color: 'white',
                                    fontWeight: refreshInterval === option.value ? 600 : 400,
                                    cursor: 'pointer',
                                    transition: 'all 0.2s'
                                }}
                            >
                                {option.label}
                            </button>
                        ))}
                    </div>
                </div>
            </div>

            {/* Developer Mode */}
            <div className="glass-panel" style={{ padding: '2rem' }}>
                <h2 style={{ fontSize: '1.25rem', fontWeight: 600, marginBottom: '1.5rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <FaTools /> Developer Options
                </h2>

                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '1.5rem', background: 'rgba(255,255,255,0.03)', borderRadius: '12px' }}>
                    <div>
                        <div style={{ fontWeight: 600, fontSize: '1.1rem', marginBottom: '0.5rem' }}>Developer Mode</div>
                        <div style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', maxWidth: '500px' }}>
                            Enables advanced features including the Web Terminal (SSH-like access).
                            <br />
                            <span style={{ color: '#ef4444', fontWeight: 500 }}>Warning: Enabling this exposes full system shell access via the web interface.</span>
                        </div>
                    </div>

                    <button
                        onClick={toggleDevMode}
                        disabled={loading}
                        style={{
                            background: config.developer_mode ? '#10b981' : '#374151',
                            color: 'white',
                            border: 'none',
                            padding: '0.75rem 1.5rem',
                            borderRadius: '8px',
                            cursor: loading ? 'not-allowed' : 'pointer',
                            fontWeight: 600,
                            transition: 'all 0.2s',
                            opacity: loading ? 0.7 : 1
                        }}
                    >
                        {config.developer_mode ? 'Enabled' : 'Disabled'}
                    </button>
                </div>
            </div>

            {/* Security & System Info */}
            <div className="glass-panel" style={{ padding: '2rem' }}>
                <h2 style={{ fontSize: '1.25rem', fontWeight: 600, marginBottom: '1.5rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <FaShieldAlt /> Security & System Hardening
                </h2>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem', color: 'var(--text-secondary)', fontSize: '0.9rem', lineHeight: 1.6 }}>
                    <div style={{ display: 'flex', alignItems: 'flex-start', gap: '0.75rem' }}>
                        <FaInfoCircle style={{ marginTop: '0.2rem', color: 'var(--accent-color, #3b82f6)', flexShrink: 0 }} />
                        <div>
                            WADM should run under a dedicated, non-root system user. System actions (reboot, service controls) should be delegated via <code>/etc/sudoers.d/wadm</code> with exact command whitelisting.
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
