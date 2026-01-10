import { useState, useEffect, useCallback } from 'react';
import { useToast } from '../context/ToastContext';

interface Package {
    name: string;
    version: string;
    status: string;
}





export default function Packages() {
    const [activeTab, setActiveTab] = useState<'updates' | 'installed'>('updates');
    const [packages, setPackages] = useState<Package[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    
    const [isActionLoading, setIsActionLoading] = useState(false);

    
    const [installPackageName, setInstallPackageName] = useState('');

    
    const [pendingRemove, setPendingRemove] = useState<string | null>(null);
    const [dryRunOutput, setDryRunOutput] = useState<string | null>(null);

    const [searchQuery, setSearchQuery] = useState('');
    const { addToast } = useToast();

    const fetchPackages = useCallback(async () => {
        setLoading(true);
        setError(null);
        try {
            const endpoint = activeTab === 'updates' ? '/api/packages' : '/api/packages/installed';
            const token = localStorage.getItem('wadm_token');
            const headers: HeadersInit = token ? { 'Authorization': `Bearer ${token}` } : {};

            const res = await fetch(endpoint, { headers });
            if (res.ok) {
                const data = await res.json();
                if (Array.isArray(data)) {
                    setPackages(data);
                } else {
                    setPackages([]); 
                    console.error('Unexpected data:', data);
                }
            } else {
                throw new Error('Failed to fetch');
            }
        } catch (err) {
            console.error(err);
            setError('Failed to load packages.');
        } finally {
            setLoading(false);
        }
    }, [activeTab]);

    useEffect(() => {
        fetchPackages();
    }, [fetchPackages]);

    const handleUpdate = async (name: string) => {
        setIsActionLoading(true);
        try {
            await fetch('/api/packages/upgrade', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name })
            });
            addToast(`Updated ${name}`, 'success');
            fetchPackages();
        } catch {
            addToast(`Failed to update ${name}`, 'error');
        } finally {
            setIsActionLoading(false);
        }
    };

    const handleInstall = async () => {
        if (!installPackageName) return;
        setIsActionLoading(true);
        try {
            await fetch('/api/packages/install', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name: installPackageName })
            });
            addToast(`Installed ${installPackageName}`, 'success');
            setInstallPackageName('');
            if (activeTab === 'installed') fetchPackages();
        } catch {
            addToast(`Failed to install`, 'error');
        } finally {
            setIsActionLoading(false);
        }
    };

    const initiateRemove = async (name: string) => {
        setPendingRemove(name);
        setDryRunOutput(null);
        
        try {
            const res = await fetch('/api/packages/remove-dry-run', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name })
            });
            const text = await res.json(); 
            setDryRunOutput(text);
        } catch {
            setDryRunOutput('Could not determine dependencies. Be careful.');
        }
    };

    const confirmRemove = async () => {
        if (!pendingRemove) return;
        setIsActionLoading(true);
        try {
            await fetch('/api/packages/remove', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name: pendingRemove })
            });
            addToast(`Removed ${pendingRemove}`, 'success');
            fetchPackages();
        } catch {
            addToast(`Failed to remove`, 'error');
        } finally {
            setPendingRemove(null);
            setDryRunOutput(null);
            setIsActionLoading(false);
        }
    };

    const filteredPackages = packages.filter(pkg =>
        pkg.name.toLowerCase().includes(searchQuery.toLowerCase())
    );

    if (loading) return <div className="glass-panel" style={{ padding: '2rem' }}>Loading packages...</div>;

    if (error) {
        return (
            <div className="glass-panel" style={{ padding: '2rem', height: '100%', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center' }}>
                <h3 style={{ color: 'var(--danger)' }}>Error Loading Packages</h3>
                <p style={{ color: 'var(--text-secondary)', marginBottom: '1rem' }}>{error}</p>
                <button onClick={fetchPackages} className="btn-primary">Retry</button>
            </div>
        );
    }

    return (
        <div className="glass-panel" style={{ padding: '0', height: '100%', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
            {/* Header */}
            <div style={{ padding: '1.5rem', borderBottom: '1px solid var(--glass-border)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <div style={{ display: 'flex', gap: '1rem', alignItems: 'center' }}>
                    <h2 style={{ margin: 0 }}>Packages</h2>
                    <div style={{ display: 'flex', background: 'rgba(0,0,0,0.2)', borderRadius: '8px', padding: '0.25rem' }}>
                        <button
                            onClick={() => setActiveTab('updates')}
                            style={{
                                background: activeTab === 'updates' ? 'var(--accent-color)' : 'transparent',
                                color: activeTab === 'updates' ? '#fff' : 'var(--text-secondary)',
                                border: 'none', padding: '0.5rem 1rem', borderRadius: '6px', cursor: 'pointer', fontWeight: 500
                            }}
                        >
                            Updates
                        </button>
                        <button
                            onClick={() => setActiveTab('installed')}
                            style={{
                                background: activeTab === 'installed' ? 'var(--accent-color)' : 'transparent',
                                color: activeTab === 'installed' ? '#fff' : 'var(--text-secondary)',
                                border: 'none', padding: '0.5rem 1rem', borderRadius: '6px', cursor: 'pointer', fontWeight: 500
                            }}
                        >
                            Installed
                        </button>
                    </div>
                </div>

                <div style={{ display: 'flex', gap: '1rem' }}>
                    <div style={{ display: 'flex', gap: '0.5rem' }}>
                        <input
                            type="text"
                            className="input-field"
                            placeholder="Package name..."
                            value={installPackageName}
                            onChange={(e) => setInstallPackageName(e.target.value)}
                            style={{ width: '150px', padding: '0.5rem' }}
                        />
                        <button onClick={handleInstall} className="btn-primary" disabled={isActionLoading || !installPackageName}>
                            Install
                        </button>
                    </div>
                    <input
                        type="text"
                        className="input-field"
                        placeholder="Search..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        style={{ padding: '0.5rem' }}
                    />
                    <button onClick={fetchPackages} className="btn-secondary">Refresh</button>
                </div>
            </div>

            {/* List */}
            <div style={{ flex: 1, overflow: 'auto', padding: '0' }}>
                <table style={{ width: '100%', borderCollapse: 'collapse' }}>
                    <thead style={{ background: 'rgba(0,0,0,0.1)', position: 'sticky', top: 0 }}>
                        <tr style={{ textAlign: 'left' }}>
                            <th style={{ padding: '1rem' }}>Name</th>
                            <th style={{ padding: '1rem' }}>Version</th>
                            <th style={{ padding: '1rem' }}>Status</th>
                            <th style={{ padding: '1rem', width: '100px' }}>Action</th>
                        </tr>
                    </thead>
                    <tbody>
                        {filteredPackages.map((pkg) => (
                            <tr key={pkg.name} style={{ borderBottom: '1px solid var(--glass-border)' }}>
                                <td style={{ padding: '1rem' }}>{pkg.name}</td>
                                <td style={{ padding: '1rem', fontFamily: 'monospace' }}>{pkg.version}</td>
                                <td style={{ padding: '1rem' }}>
                                    <span className={`badge ${activeTab === 'updates' ? 'warning' : 'success'}`}>
                                        {activeTab === 'updates' ? 'Update Available' : 'Installed'}
                                    </span>
                                </td>
                                <td style={{ padding: '1rem' }}>
                                    {activeTab === 'updates' ? (
                                        <button
                                            className="btn-sm"
                                            onClick={() => handleUpdate(pkg.name)}
                                            disabled={isActionLoading}
                                        >
                                            Update
                                        </button>
                                    ) : (
                                        <button
                                            className="btn-sm danger"
                                            onClick={() => initiateRemove(pkg.name)}
                                            disabled={isActionLoading}
                                        >
                                            Remove
                                        </button>
                                    )}
                                </td>
                            </tr>
                        ))}
                        {filteredPackages.length === 0 && (
                            <tr>
                                <td colSpan={4} style={{ padding: '2rem', textAlign: 'center', color: 'var(--text-secondary)' }}>
                                    {activeTab === 'updates' ? 'No updates available.' : 'No packages found.'}
                                </td>
                            </tr>
                        )}
                    </tbody>
                </table>
            </div>

            {/* Removal Confirmation Modal */}
            {pendingRemove && (
                <div style={{
                    position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
                    background: 'rgba(0,0,0,0.7)', backdropFilter: 'blur(5px)',
                    display: 'flex', justifyContent: 'center', alignItems: 'center', zIndex: 1000
                }}>
                    <div className="glass-panel" style={{ width: '500px', maxWidth: '90%', padding: '2rem', maxHeight: '80vh', display: 'flex', flexDirection: 'column' }}>
                        <h3 style={{ marginTop: 0, color: 'var(--danger)' }}>Confirm Removal</h3>
                        <p>Are you sure you want to remove <strong>{pendingRemove}</strong>?</p>

                        <div style={{ background: 'rgba(0,0,0,0.3)', padding: '1rem', borderRadius: '8px', flex: 1, overflow: 'auto', marginBottom: '1.5rem' }}>
                            <div style={{ fontWeight: 600, marginBottom: '0.5rem', color: 'var(--warning)' }}>Impact Analysis (Dry Run):</div>
                            <pre style={{ margin: 0, whiteSpace: 'pre-wrap', fontSize: '0.85rem', fontFamily: 'monospace' }}>
                                {dryRunOutput || 'Calculating dependencies...'}
                            </pre>
                        </div>

                        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '1rem' }}>
                            <button className="btn-secondary" onClick={() => setPendingRemove(null)}>Cancel</button>
                            <button className="btn-primary danger" onClick={confirmRemove}>Confirm Remove</button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}
