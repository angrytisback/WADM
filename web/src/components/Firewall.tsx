import { useState, useEffect, useCallback } from 'react';
import { useToast } from '../context/ToastContext';

interface FirewallStatus {
    active: boolean;
    rules: string[];
    installed?: boolean;
}

function Firewall() {
    const [status, setStatus] = useState<FirewallStatus | null>(null);
    const [loading, setLoading] = useState(false);
    const [newRule, setNewRule] = useState('');
    const [showReboot, setShowReboot] = useState(false);
    const { addToast } = useToast();

    const fetchStatus = useCallback(() => {
        setLoading(true);
        fetch('/api/firewall')
            .then(res => res.json())
            .then(data => {
                setStatus(data);
                setLoading(false);
            })
            .catch(err => {
                console.error(err);
                addToast('Failed to fetch firewall status', 'error');
                setLoading(false);
            });
    }, [addToast]);

    useEffect(() => {
        fetchStatus();
    }, [fetchStatus]);

    const installFirewall = async () => {
        setLoading(true);
        try {
            const res = await fetch('/api/firewall/install', { method: 'POST' });
            if (res.ok) {
                addToast('Firewall installed successfully', 'success');
                fetchStatus();
            } else {
                const err = await res.json();
                addToast(`Installation failed: ${err}`, 'error');
            }
        } catch {
            addToast('Installation request failed', 'error');
        } finally {
            setLoading(false);
        }
    };

    const toggleFirewall = async () => {
        if (!status) return;
        const action = status.active ? 'disable' : 'enable';

        if (action === 'enable' && !confirm("Are you sure you want to enable the firewall? Ensure you have SSH allowed if remote.")) {
            return;
        }

        try {
            const res = await fetch('/api/firewall/action', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ action })
            });
            if (!res.ok) throw new Error('Failed to toggle firewall');

            addToast(`Firewall ${action}d`, 'success');
            if (action === 'enable') {
                setShowReboot(true);
            }
            fetchStatus();
        } catch {
            addToast("Failed to fetch firewall status", "error");
        }
    };

    const rebootSystem = async () => {
        try {
            await fetch('/api/system/reboot', { method: 'POST' });
            addToast('Rebooting system...', 'success');
            setShowReboot(false);
        } catch {
            addToast('Failed to reboot system', 'error');
        }
    };

    const addRule = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!newRule.trim()) return;

        try {
            const res = await fetch('/api/firewall/rules', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ rule: newRule })
            });
            if (!res.ok) throw new Error('Failed to add rule');

            addToast("Rule added successfully", "success");
            await fetchStatus();
            setNewRule('');
        } catch {
            addToast("Failed to add rule", "error");
        }
    };

    const deleteRule = async (rule: string) => {
        if (!confirm(`Delete rule: ${rule}?`)) return;

        
        

        try {
            const res = await fetch('/api/firewall/rules', {
                method: 'DELETE',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ rule: rule }) 
            });
            if (!res.ok) throw new Error('Failed to delete rule');

            addToast("Rule deleted successfully", "success");
            await fetchStatus();
        } catch {
            addToast("Failed to delete rule", "error");
        }
    };

    if (status && status.installed === false) {
        return (
            <div className="firewall-container fade-in">
                <div className="glass-panel" style={{ padding: '2rem', textAlign: 'center', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1rem' }}>
                    <h2 style={{ color: 'var(--warning)' }}>Firewall Not Installed</h2>
                    <p style={{ color: 'var(--text-secondary)' }}>
                        UFW (Uncomplicated Firewall) was not found on this system.
                        Would you like to install it now?
                    </p>
                    <button
                        className="btn-primary"
                        onClick={installFirewall}
                        disabled={loading}
                    >
                        {loading ? 'Installing...' : 'Install Firewall (UFW)'}
                    </button>
                </div>
            </div>
        );
    }

    return (
        <div className="firewall-container fade-in">
            <div className="glass-panel" style={{
                marginBottom: '2rem',
                padding: '1.5rem',
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                flexWrap: 'wrap',
                gap: '1rem'
            }}>
                <div>
                    <h2 style={{ margin: 0 }}>Firewall Status (UFW)</h2>
                    <div style={{
                        marginTop: '0.5rem',
                        color: status?.active ? '#4caf50' : '#ff5252',
                        fontWeight: 'bold'
                    }}>
                        {status ? (status.active ? 'ACTIVE' : 'INACTIVE') : 'Loading...'}
                    </div>
                </div>
                <button
                    className={`btn ${status?.active ? 'btn-danger' : 'btn-primary'}`}
                    onClick={toggleFirewall}
                    disabled={loading || !status}
                >
                    {status?.active ? 'Disable Firewall' : 'Enable Firewall'}
                </button>
            </div>

            <div className="glass-panel">
                <h3>Rules</h3>

                <form onSubmit={addRule} style={{ display: 'flex', gap: '1rem', marginBottom: '2rem' }}>
                    <input
                        type="text"
                        className="input-field"
                        placeholder="e.g. allow 80/tcp"
                        value={newRule}
                        onChange={e => setNewRule(e.target.value)}
                        style={{ flex: 1 }}
                    />
                    <button type="submit" className="btn btn-primary" disabled={loading}>Add Rule</button>
                </form>

                <div className="rules-list">
                    {status?.rules && status.rules.length > 0 ? (
                        <table style={{ width: '100%', borderCollapse: 'collapse' }}>
                            <thead>
                                <tr style={{ textAlign: 'left', borderBottom: '1px solid var(--glass-border)' }}>
                                    <th style={{ padding: '1rem' }}>Rule</th>
                                    <th style={{ padding: '1rem', textAlign: 'right' }}>Action</th>
                                </tr>
                            </thead>
                            <tbody>
                                {status.rules.map((rule, idx) => (
                                    <tr key={idx} style={{ borderBottom: '1px solid rgba(255,255,255,0.05)' }}>
                                        <td style={{ padding: '1rem', fontFamily: 'monospace' }}>{rule}</td>
                                        <td style={{ padding: '1rem', textAlign: 'right' }}>
                                            <button
                                                onClick={() => deleteRule(rule)}
                                                className="btn-text item-action danger"
                                                title="Delete Rule"
                                            >
                                                🗑️
                                            </button>
                                        </td>
                                    </tr>
                                ))}
                            </tbody>
                        </table>
                    ) : (
                        <div style={{ padding: '2rem', textAlign: 'center', color: 'var(--text-secondary)' }}>
                            No rules found.
                        </div>
                    )}
                </div>
            </div>


            {
                showReboot && (
                    <div style={{
                        position: 'fixed', inset: 0,
                        background: 'rgba(0,0,0,0.7)', backdropFilter: 'blur(5px)',
                        display: 'flex', alignItems: 'center', justifyContent: 'center',
                        zIndex: 1000
                    }}>
                        <div className="glass-panel" style={{ padding: '2rem', maxWidth: '400px', width: '90%' }}>
                            <h2 style={{ marginBottom: '1rem', color: 'var(--warning)' }}>System Reboot Required</h2>
                            <p style={{ color: 'var(--text-secondary)', marginBottom: '1.5rem', lineHeight: '1.6' }}>
                                Enabling the firewall requires a system reboot to ensure all network rules are correctly applied and secure.
                            </p>
                            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '1rem' }}>
                                <button className="btn-text" onClick={() => setShowReboot(false)}>
                                    Later
                                </button>
                                <button className="btn-primary" style={{ borderColor: 'var(--warning)', color: 'var(--warning)' }} onClick={rebootSystem}>
                                    Reboot Now
                                </button>
                            </div>
                        </div>
                    </div>
                )
            }
        </div >
    );
}

export default Firewall;
