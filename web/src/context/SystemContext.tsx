import React, { createContext, useContext, useState, useEffect } from 'react';
import type { SystemInfo } from '../types';
import { useAuth } from './AuthContext';

interface SystemContextType {
    systemInfo: SystemInfo | null;
    loading: boolean;
    refreshSystemInfo: () => Promise<void>;
}

const SystemContext = createContext<SystemContextType | undefined>(undefined);

export function SystemProvider({ children }: { children: React.ReactNode }) {
    const { isAuthenticated } = useAuth();
    const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
    const [loading, setLoading] = useState(false);

    const refreshSystemInfo = async () => {
        if (!isAuthenticated) return;
        setLoading(true);
        try {
            const res = await fetch('/api/system');
            if (res.ok) {
                const data = await res.json();
                setSystemInfo(data);
            }
        } catch (err) {
            console.error("Failed to fetch system info:", err);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        if (isAuthenticated) {
            refreshSystemInfo();
        } else {
            setSystemInfo(null);
        }
    }, [isAuthenticated]);

    return (
        <SystemContext.Provider value={{ systemInfo, loading, refreshSystemInfo }}>
            {children}
        </SystemContext.Provider>
    );
}

export function useSystem() {
    const context = useContext(SystemContext);
    if (context === undefined) {
        throw new Error('useSystem must be used within a SystemProvider');
    }
    return context;
}
