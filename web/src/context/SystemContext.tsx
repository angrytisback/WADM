/* eslint-disable react-refresh/only-export-components */
import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import type { SystemInfo } from '../types';
import { useAuth } from './AuthContext';
import { useCluster } from './ClusterContext';

interface SystemContextType {
    systemInfo: SystemInfo | null;
    loading: boolean;
    refreshSystemInfo: () => Promise<void>;
}

const SystemContext = createContext<SystemContextType | undefined>(undefined);

export function SystemProvider({ children }: { children: React.ReactNode }) {
    const { isAuthenticated, token } = useAuth();
    const { activeNode } = useCluster();
    const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
    const [loading, setLoading] = useState(false);

    const refreshSystemInfo = useCallback(async () => {
        if (!isAuthenticated) return;
        setLoading(true);
        try {
            const url = activeNode ? `/api/cluster/nodes/${activeNode.id}/system` : '/api/system';
            const res = await fetch(url, {
                headers: {
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                credentials: 'include',
            });
            if (res.ok) {
                const data = await res.json();
                setSystemInfo(data);
            }
        } catch (err) {
            console.error("Failed to fetch system info:", err);
        } finally {
            setLoading(false);
        }
    }, [isAuthenticated, token, activeNode]);

    useEffect(() => {
        if (isAuthenticated) {
            refreshSystemInfo();
        } else {
            setSystemInfo(null);
        }
    }, [isAuthenticated, activeNode, refreshSystemInfo]);

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
