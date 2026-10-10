/* eslint-disable react-refresh/only-export-components */
import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import type { DependencyReport } from '../types';
import { useAuth } from './AuthContext';

interface DependencyContextType {
    report: DependencyReport | null;
    loading: boolean;
    refreshDependencies: () => Promise<void>;
}

const DependencyContext = createContext<DependencyContextType | undefined>(undefined);

export const DependencyProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
    const { isAuthenticated } = useAuth();
    const [report, setReport] = useState<DependencyReport | null>(null);
    const [loading, setLoading] = useState(true);

    const refreshDependencies = useCallback(async () => {
        setLoading(true);
        try {
            const res = await fetch('/api/system/dependencies', { credentials: 'include' });
            if (res.ok) {
                const data = await res.json();
                setReport(data);
            }
        } catch (err) {
            console.error('Failed to fetch dependencies', err);
        } finally {
            setLoading(false);
        }
    }, []);

    useEffect(() => {
        if (isAuthenticated) {
            refreshDependencies();
        }
    }, [isAuthenticated, refreshDependencies]);

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
