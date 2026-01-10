import { useState, useEffect } from 'react';
import { useToast } from '../context/ToastContext';

interface Config {
    developer_mode: boolean;
}

export default function Settings() {
    const [config, setConfig] = useState<Config | null>(null);
    const [loading, setLoading] = useState(false);
    const { addToast } = useToast();

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
        } catch (error) {
            addToast('Failed to update settings', 'error');
        } finally {
            setLoading(false);
        }
    };

    if (!config) return <div style={{ padding: '2rem' }}>Loading settings...</div>;

    return (
        <div style={{ padding: '2rem', maxWidth: '800px', margin: '0 auto' }}>
            <h1 style={{ fontSize: '2rem', fontWeight: 700, marginBottom: '2rem' }}>system settings</h1>

            <div className="glass-panel" style={{ padding: '2rem' }}>
                <h2 style={{ fontSize: '1.25rem', fontWeight: 600, marginBottom: '1.5rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    <span role="img" aria-label="tools">🛠️</span> Developer Options
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
        </div>
    );
}
