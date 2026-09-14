import React, { createContext, useContext, useState, useEffect } from 'react';
import type { DependencyReport } from '../types';

interface DependencyContextType {
    report: DependencyReport | null;
    loading: boolean;
    refreshDependencies: () => Promise<void>;
}

const DependencyContext = createContext<DependencyContextType | undefined>(undefined);

export const DependencyProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
    const [report, setReport] = useState<DependencyReport | null>(null);
    const [loading, setLoading] = useState(true);

    const refreshDependencies = async () => {
        setLoading(true);
        try {
            const token = localStorage.getItem('wadm_token');
            const headers: HeadersInit = token ? { 'Authorization': `Bearer ${token}` } : {};
            const res = await fetch('/api/system/dependencies', { headers });
            if (res.ok) {
                const data = await res.json();
                setReport(data);
            }
        } catch (err) {
            console.error('Failed to fetch dependencies', err);
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        const token = localStorage.getItem('wadm_token');
        if (token) {
            refreshDependencies();
        }
    }, []);

    return (
        <DependencyContext.Provider value={{ report, loading, refreshDependencies }}>
            {children}
        </DependencyContext.Provider>
    );
};

export const useDependencies = () => {
    const context = useContext(DependencyContext);
    if (!context) {
        throw new Error('useDependencies must be used within a DependencyProvider');
    }
    return context;
};
