import { useState, useEffect } from 'react';
import { FaShieldAlt, FaExclamationTriangle } from 'react-icons/fa';

interface RootWarningModalProps {
    isRoot: boolean;
    hasSudo: boolean;
}

export function RootWarningModal({ isRoot, hasSudo }: RootWarningModalProps) {
    const [isOpen, setIsOpen] = useState(false);

    useEffect(() => {
        const dismissed = sessionStorage.getItem('root_warning_dismissed');
        if (!isRoot && dismissed !== 'true') {
            setIsOpen(true);
        }
    }, [isRoot]);

    const handleDismiss = () => {
        sessionStorage.setItem('root_warning_dismissed', 'true');
        setIsOpen(false);
    };

    if (!isOpen) return null;

    return (
        <div style={{
            position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
            background: 'rgba(0,0,0,0.8)',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            zIndex: 3000,
            backdropFilter: 'blur(8px)'
        }}>
            <div className="glass-panel" style={{ width: '450px', maxWidth: '90%', padding: '2.5rem', textAlign: 'center', border: '1px solid rgba(245, 158, 11, 0.3)' }}>
                <div style={{ 
                    fontSize: '4rem', 
                    color: '#fbbf24', 
                    marginBottom: '1.5rem',
                    animation: 'pulse-warning 2s infinite'
                }}>
                    <FaShieldAlt />
                </div>
                
                <h2 style={{ marginTop: 0, color: '#fbbf24', fontSize: '1.8rem' }}>Insufficient Privileges</h2>
                <p style={{ color: 'var(--text-secondary)', lineHeight: '1.6', marginBottom: '2rem' }}>
                    WADM is currently running without <strong>root</strong> privileges. 
                    Many features like service management, package installation, and firewall configuration may not function correctly.
                </p>

                {!hasSudo && (
                    <div style={{ 
                        background: 'rgba(239, 68, 68, 0.1)', 
                        border: '1px solid rgba(239, 68, 68, 0.2)',
                        padding: '1rem',
                        borderRadius: '12px',
                        marginBottom: '2rem',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '1rem',
                        textAlign: 'left'
                    }}>
                        <FaExclamationTriangle style={{ color: '#f87171', fontSize: '1.5rem', flexShrink: 0 }} />
                        <span style={{ fontSize: '0.9rem', color: '#f87171' }}>
                            Sudo access was also not detected for the current user.
                        </span>
                    </div>
                )}

                <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                    <button className="btn-primary" onClick={handleDismiss} style={{ background: '#fbbf24', color: '#000', fontWeight: 700 }}>
                        I Understand
                    </button>
                    <p style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                        To run as root, restart the application using <code>sudo</code>.
                    </p>
                </div>
            </div>
            
            <style>{`
                @keyframes pulse-warning {
                    0% { transform: scale(1); opacity: 1; }
                    50% { transform: scale(1.05); opacity: 0.8; }
                    100% { transform: scale(1); opacity: 1; }
                }
            `}</style>
        </div>
    );
}
