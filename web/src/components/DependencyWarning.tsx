import { useState } from 'react';
import { useDependencies } from '../context/DependencyContext';
import { FaExclamationTriangle, FaTimes } from 'react-icons/fa';

export function DependencyWarning() {
    const { report } = useDependencies();
    const [isOpen, setIsOpen] = useState(false);
    const [hasShownAuto, setHasShownAuto] = useState(false);

    const missing = report?.dependencies.filter(d => !d.installed) || [];
    const critical = missing.some(d => !d.optional);

    if (missing.length > 0 && !hasShownAuto && !isOpen) {
        setIsOpen(true);
        setHasShownAuto(true);
    }

    if (!report) return null;
    if (missing.length === 0) return null;

    return (
        <div style={{ position: 'fixed', bottom: '2rem', right: '2rem', zIndex: 1000 }}>
            {isOpen ? (
                <div className="glass-panel" style={{ width: '300px', padding: '1.5rem', boxShadow: '0 10px 30px rgba(0,0,0,0.5)', border: critical ? '1px solid var(--danger)' : '1px solid var(--warning)' }}>
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
                        <h4 style={{ margin: 0, color: critical ? 'var(--danger)' : 'var(--warning)', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                            <FaExclamationTriangle />
                            {critical ? 'Critical Missing' : 'Optional Missing'}
                        </h4>
                        <button onClick={() => setIsOpen(false)} className="btn-text" style={{ padding: '0.25rem', minWidth: 'auto' }}>
                            <FaTimes />
                        </button>
                    </div>

                    <div style={{ maxHeight: '300px', overflowY: 'auto' }}>
                        {missing.map((dep: any, idx: number) => (
                            <div key={idx} style={{ marginBottom: '1rem', paddingBottom: '1rem', borderBottom: idx < missing.length - 1 ? '1px solid var(--glass-border)' : 'none' }}>
                                <div style={{ fontWeight: 500, marginBottom: '0.25rem' }}>{dep.name}</div>
                                <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>
                                    Missing: <code>{dep.command}</code>
                                </div>
                                {dep.install_hint && (
                                    <div style={{ background: 'rgba(0,0,0,0.2)', padding: '0.5rem', borderRadius: '4px', fontSize: '0.8rem', fontFamily: 'monospace', userSelect: 'all' }}>
                                        {dep.install_hint}
                                    </div>
                                )}
                            </div>
                        ))}
                    </div>
                </div>
            ) : (
                <button
                    onClick={() => setIsOpen(true)}
                    style={{
                        width: '56px',
                        height: '56px',
                        borderRadius: '50%',
                        background: critical ? 'var(--danger)' : 'var(--warning)',
                        border: 'none',
                        color: 'white', // Icon color
                        fontSize: '1.5rem',
                        cursor: 'pointer',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        boxShadow: '0 4px 12px rgba(0,0,0,0.3)',
                        animation: critical ? 'pulse 2s infinite' : 'none'
                    }}
                    title={`${missing.length} Missing Dependencies`}
                >
                    <FaExclamationTriangle />
                </button>
            )}
            {critical && !isOpen && (
                <style dangerouslySetInnerHTML={{
                    __html: `
                    @keyframes pulse {
                        0% { box-shadow: 0 0 0 0 rgba(248, 113, 113, 0.7); }
                        70% { box-shadow: 0 0 0 15px rgba(248, 113, 113, 0); }
                        100% { box-shadow: 0 0 0 0 rgba(248, 113, 113, 0); }
                    }
                 `}} />
            )}
        </div>
    );
}
