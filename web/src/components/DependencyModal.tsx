import { useState, useEffect } from 'react';
import { useDependencies } from '../context/DependencyContext';
import { useToast } from '../context/ToastContext';
import { useSystem } from '../context/SystemContext';
import { FaDownload } from 'react-icons/fa';

export function DependencyModal() {
    const { report, loading, refreshDependencies } = useDependencies();
    const [isOpen, setIsOpen] = useState(true);
    const [installing, setInstalling] = useState<string | null>(null);
    const { addToast } = useToast();
    const { systemInfo } = useSystem();
    const canManage = systemInfo?.is_root || systemInfo?.has_sudo;
    const privilegeHint = !canManage ? "Root or Sudo privileges required for this action" : "";

    // Check strict dismissal preference
    useEffect(() => {
        const dismissed = localStorage.getItem('dependency_modal_dismissed');
        if (dismissed === 'true') {
            setIsOpen(false);
        }
    }, []);

    // Also close if all resolved
    useEffect(() => {
        if (report && !loading) {
            const hasMissing = report.dependencies.some(d => !d.installed);
            if (!hasMissing) {
                setIsOpen(false);
            }
        }
    }, [report, loading]);

    const handleDismiss = () => {
        localStorage.setItem('dependency_modal_dismissed', 'true');
        setIsOpen(false);
    };

    const handleInstall = async (depName: string) => {
        setInstalling(depName);
        try {
            const res = await fetch('/api/system/dependencies/install', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name: depName })
            });

            if (res.ok) {
                addToast(`${depName} installed successfully`, "success");
                await refreshDependencies();
            } else {
                addToast(`Failed to install ${depName}`, "error");
            }
        } catch (e) {
            addToast(`Error installing ${depName}`, "error");
        } finally {
            setInstalling(null);
        }
    };

    if (loading || !report || !isOpen) return null;

    const missing = report.dependencies.filter(d => !d.installed);
    if (missing.length === 0) return null;

    return (
        <div style={{
            position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
            background: 'rgba(0,0,0,0.7)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            zIndex: 2000,
            backdropFilter: 'blur(5px)'
        }}>
            <div className="glass-panel" style={{ width: '500px', maxWidth: '90%', padding: '2rem', maxHeight: '80vh', overflowY: 'auto' }}>
                <h2 style={{ marginTop: 0, color: 'var(--text-primary)', textAlign: 'center' }}>Available System Updates</h2>
                <p style={{ color: 'var(--text-secondary)', textAlign: 'center', marginBottom: '2rem' }}>
                    The following system components are missing and required for full functionality.
                </p>

                <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                    {missing.map((dep: any, idx: number) => (
                        <div key={idx} style={{
                            padding: '1.2rem',
                            background: 'var(--glass-bg)',
                            borderRadius: '12px',
                            border: '1px solid var(--glass-border)',
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'space-between'
                        }}>
                            <div>
                                <div style={{ display: 'flex', alignItems: 'center', gap: '0.8rem' }}>
                                    <strong style={{ fontSize: '1.1rem' }}>{dep.name}</strong>
                                    <span className="badge" style={{
                                        background: dep.optional ? 'rgba(245, 158, 11, 0.2)' : 'rgba(239, 68, 68, 0.2)',
                                        color: dep.optional ? '#fbbf24' : '#f87171',
                                        fontSize: '0.75rem'
                                    }}>
                                        {dep.optional ? 'Optional' : 'Critical'}
                                    </span>
                                </div>
                                <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginTop: '0.3rem' }}>
                                    Required system package
                                </div>
                            </div>

                            <button
                                className="btn-primary"
                                onClick={() => handleInstall(dep.name)}
                                disabled={installing !== null || !canManage}
                                title={privilegeHint}
                                style={{
                                    padding: '0.5rem 1rem',
                                    display: 'flex',
                                    alignItems: 'center',
                                    gap: '0.5rem',
                                    minWidth: '100px',
                                    justifyContent: 'center',
                                    opacity: !canManage ? 0.5 : 1
                                }}
                            >
                                {installing === dep.name ? (
                                    <span className="spin">⟳</span>
                                ) : (
                                    <FaDownload />
                                )}
                                {installing === dep.name ? 'Installing...' : 'Install'}
                            </button>
                        </div>
                    ))}
                </div>

                <div style={{ marginTop: '2rem', display: 'flex', justifyContent: 'center' }}>
                    <button className="btn-text" onClick={handleDismiss} style={{ color: 'var(--text-secondary)' }}>
                        Dismiss (Don't show again)
                    </button>
                </div>
            </div>
        </div>
    );
}
