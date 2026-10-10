import { useState, useEffect } from 'react';
import { useToast } from '../context/ToastContext';
import { useStats } from '../context/StatsContext';
import { useAuth } from '../context/AuthContext';
import { FaTools, FaClock, FaShieldAlt, FaInfoCircle, FaCog, FaLock } from 'react-icons/fa';
import SSLManagement from './SSLManagement';

interface Config {
    developer_mode: boolean;
}

export default function Settings() {
    const [config, setConfig] = useState<Config | null>(null);
    const [loading, setLoading] = useState(false);
    const [settingsTab, setSettingsTab] = useState<'general' | 'ssl'>('general');
    const { addToast } = useToast();
    const { refreshInterval, setRefreshInterval } = useStats();
    const { isAdmin } = useAuth();

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
        <div style={{ padding: '2rem', maxWidth: '1000px', margin: '0 auto', display: 'flex', flexDirection: 'column', gap: '2rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '1rem' }}>
                <h1 style={{ fontSize: '2rem', fontWeight: 700, margin: 0 }}>System Settings</h1>
                <div style={{ display: 'flex', gap: '0.5rem', background: 'rgba(0,0,0,0.2)', padding: '4px', borderRadius: '10px' }}>
                    <button
                        onClick={() => setSettingsTab('general')}
                        style={{
                            display: 'flex', alignItems: 'center', gap: '0.5rem',
                            padding: '0.5rem 1rem', borderRadius: '8px', border: 'none',
                            background: settingsTab === 'general' ? 'var(--accent-color, #38bdf8)' : 'transparent',
                            color: settingsTab === 'general' ? '#0f172a' : 'var(--text-secondary)',
                            fontWeight: 600, cursor: 'pointer', transition: 'all 0.2s', fontSize: '0.9rem'
                        }}
                    >
                        <FaCog /> General
                    </button>
                    <button
                        onClick={() => setSettingsTab('ssl')}
                        style={{
                            display: 'flex', alignItems: 'center', gap: '0.5rem',
                            padding: '0.5rem 1rem', borderRadius: '8px', border: 'none',
                            background: settingsTab === 'ssl' ? 'var(--accent-color, #38bdf8)' : 'transparent',
                            color: settingsTab === 'ssl' ? '#0f172a' : 'var(--text-secondary)',
                            fontWeight: 600, cursor: 'pointer', transition: 'all 0.2s', fontSize: '0.9rem'
                        }}
                    >
                        <FaLock /> SSL / TLS
                    </button>
                </div>
            </div>

            {settingsTab === 'ssl' ? (
                <SSLManagement />
            ) : (
                <>
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
                        disabled={loading || !isAdmin()}
                        title={!isAdmin() ? "Admin role required to toggle developer mode" : ""}
                        style={{
                            background: config.developer_mode ? '#10b981' : '#374151',
                            color: 'white',
                            border: 'none',
                            padding: '0.75rem 1.5rem',
                            borderRadius: '8px',
                            cursor: (loading || !isAdmin()) ? 'not-allowed' : 'pointer',
                            fontWeight: 600,
                            transition: 'all 0.2s',
                            opacity: loading ? 0.7 : !isAdmin() ? 0.5 : 1
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
            </>
            )}
        </div>
    );
}
