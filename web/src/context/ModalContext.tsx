import { createContext, useContext, useState, type ReactNode, useCallback } from 'react';

interface ModalOptions {
    title?: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    type?: 'danger' | 'info' | 'warning';
}

interface ModalContextType {
    confirm: (options: ModalOptions) => Promise<boolean>;
}

const ModalContext = createContext<ModalContextType | undefined>(undefined);

export function useModal() {
    const context = useContext(ModalContext);
    if (!context) {
        throw new Error('useModal must be used within a ModalProvider');
    }
    return context;
}

export function ModalProvider({ children }: { children: ReactNode }) {
    const [config, setConfig] = useState<ModalOptions | null>(null);
    const [resolver, setResolver] = useState<((value: boolean) => void) | null>(null);

    const confirm = useCallback((options: ModalOptions) => {
        setConfig(options);
        return new Promise<boolean>((resolve) => {
            setResolver(() => resolve);
        });
    }, []);

    const handleConfirm = () => {
        resolver?.(true);
        setConfig(null);
        setResolver(null);
    };

    const handleCancel = () => {
        resolver?.(false);
        setConfig(null);
        setResolver(null);
    };

    return (
        <ModalContext.Provider value={{ confirm }}>
            {children}
            {config && (
                <div style={{
                    position: 'fixed',
                    top: 0,
                    left: 0,
                    right: 0,
                    bottom: 0,
                    background: 'rgba(0, 0, 0, 0.7)',
                    backdropFilter: 'blur(8px)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    zIndex: 9999,
                    animation: 'fadeIn 0.2s ease-out'
                }}>
                    <div className="glass-panel" style={{
                        width: '400px',
                        maxWidth: '90%',
                        padding: '2rem',
                        textAlign: 'center',
                        boxShadow: '0 20px 40px rgba(0,0,0,0.5)',
                        border: `1px solid ${config.type === 'danger' ? 'rgba(239, 68, 68, 0.3)' : 'var(--glass-border)'}`,
                        animation: 'scaleUp 0.2s cubic-bezier(0.34, 1.56, 0.64, 1)'
                    }}>
                        <h3 style={{ 
                            marginTop: 0, 
                            marginBottom: '1rem',
                            color: config.type === 'danger' ? '#f87171' : 'var(--text-primary)'
                        }}>
                            {config.title || 'Are you sure?'}
                        </h3>
                        <p style={{ 
                            color: 'var(--text-secondary)', 
                            lineHeight: '1.6',
                            marginBottom: '2rem' 
                        }}>
                            {config.message}
                        </p>
                        
                        <div style={{ display: 'flex', gap: '1rem', justifyContent: 'center' }}>
                            <button className="btn-secondary" onClick={handleCancel} style={{ flex: 1 }}>
                                {config.cancelText || 'Cancel'}
                            </button>
                            <button 
                                className={config.type === 'danger' ? 'btn-primary danger' : 'btn-primary'} 
                                onClick={handleConfirm}
                                style={{ 
                                    flex: 1,
                                    background: config.type === 'danger' ? '#ef4444' : 'var(--accent-color)',
                                    borderColor: config.type === 'danger' ? '#ef4444' : 'var(--accent-color)',
                                    color: config.type === 'danger' ? 'white' : 'var(--bg-color)'
                                }}
                            >
                                {config.confirmText || 'Confirm'}
                            </button>
                        </div>
                    </div>
                    
                    <style>{`
                        @keyframes fadeIn {
                            from { opacity: 0; }
                            to { opacity: 1; }
                        }
                        @keyframes scaleUp {
                            from { transform: scale(0.9); opacity: 0; }
                            to { transform: scale(1); opacity: 1; }
                        }
                    `}</style>
                </div>
            )}
        </ModalContext.Provider>
    );
}
