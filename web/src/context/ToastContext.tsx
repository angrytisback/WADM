/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useState, type ReactNode, useCallback } from 'react';

type ToastType = 'success' | 'error' | 'info' | 'warning';

interface Toast {
    id: number;
    message: string;
    type: ToastType;
}

interface ToastContextType {
    addToast: (message: string, type?: ToastType) => void;
}

const ToastContext = createContext<ToastContextType | undefined>(undefined);

export function useToast() {
    const context = useContext(ToastContext);
    if (!context) {
        throw new Error('useToast must be used within a ToastProvider');
    }
    return context;
}

export function ToastProvider({ children }: { children: ReactNode }) {
    const [toasts, setToasts] = useState<Toast[]>([]);

    const removeToast = (id: number) => {
        setToasts((prev) => prev.filter((toast) => toast.id !== id));
    };

    const addToast = useCallback((message: string, type: ToastType = 'info') => {
        const id = Date.now();
        setToasts((prev) => [...prev, { id, message, type }]);
        setTimeout(() => removeToast(id), 5000);
    }, []);

    return (
        <ToastContext.Provider value={{ addToast }}>
            {children}
            <div style={{
                position: 'fixed',
                top: '20px',
                right: '20px',
                zIndex: 9999,
                display: 'flex',
                flexDirection: 'column',
                gap: '10px',
                pointerEvents: 'none' // Allow clicks to pass through container
            }}>
                {toasts.map((toast) => (
                    <div
                        key={toast.id}
                        style={{
                            minWidth: '300px',
                            background: 'rgba(23, 23, 23, 0.95)',
                            backdropFilter: 'blur(10px)',
                            border: '1px solid var(--glass-border)',
                            borderRadius: '8px',
                            padding: '1rem',
                            color: 'white',
                            boxShadow: '0 4px 12px rgba(0,0,0,0.5)',
                            animation: 'slideIn 0.3s ease-out',
                            pointerEvents: 'auto', // Re-enable clicks
                            display: 'flex',
                            alignItems: 'center',
                            gap: '12px'
                        }}
                    >
                        <div style={{
                            width: '8px',
                            height: '8px',
                            borderRadius: '50%',
                            background: getToastColor(toast.type),
                            flexShrink: 0
                        }} />
                        <div style={{ fontSize: '0.9rem' }}>{toast.message}</div>
                    </div>
                ))}
            </div>
        </ToastContext.Provider>
    );
}

function getToastColor(type: ToastType): string {
    switch (type) {
        case 'success': return 'var(--success)';
        case 'error': return 'var(--error)';
        case 'warning': return 'var(--warning)';
        default: return 'var(--primary)';
    }
}
