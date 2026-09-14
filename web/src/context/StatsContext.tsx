import React, { createContext, useContext, useState, useEffect, useRef } from 'react';
import type { SystemStats } from '../types';
import { useAuth } from './AuthContext';
import { useServerStatus } from './ServerStatusContext';

export interface PerformanceData {
    time: string;
    cpu: number;
    memory: number;
    swap: number;
    network_rx: number; // in KB/s for chart
    network_tx: number; // in KB/s for chart
    ram_used: number;
    ram_total: number;
    swap_used: number;
    swap_total: number;
    cpu_temp: number;
    gpus: {
        load: number;
        vram_percent: number;
        vram_used: number;
        vram_total: number;
        temp: number;
        name: string;
    }[];
}

interface StatsContextType {
    stats: SystemStats | null;
    history: PerformanceData[];
}

const StatsContext = createContext<StatsContextType | undefined>(undefined);

export function StatsProvider({ children }: { children: React.ReactNode }) {
    const { isAuthenticated, logout } = useAuth();
    const { setServerOffline } = useServerStatus();
    const [stats, setStats] = useState<SystemStats | null>(null);
    // Initialize with empty data to fill the chart initially
    const [history, setHistory] = useState<PerformanceData[]>(() =>
        Array(60).fill({
            time: '', cpu: 0, memory: 0, swap: 0, network_rx: 0, network_tx: 0,
            ram_used: 0, ram_total: 0, swap_used: 0, swap_total: 0,
            cpu_temp: 0,
            gpus: []
        })
    );
    const lastNetworkRef = useRef<{ rx: number, tx: number, time: number } | null>(null);

    useEffect(() => {
        if (!isAuthenticated) return;

        const fetchStats = async () => {
            try {
                const response = await fetch('/api/stats');
                if (response.ok) {
                    const data: SystemStats = await response.json();
                    setStats(data);

                    const now = Date.now();
                    let rxRate = 0;
                    let txRate = 0;

                    if (lastNetworkRef.current) {
                        const timeDelta = (now - lastNetworkRef.current.time) / 1000;
                        if (timeDelta > 0) {
                            const rxDelta = data.network_rx - lastNetworkRef.current.rx;
                            const txDelta = data.network_tx - lastNetworkRef.current.tx;
                            if (rxDelta >= 0 && txDelta >= 0) {
                                rxRate = rxDelta / timeDelta;
                                txRate = txDelta / timeDelta;
                            }
                        }
                    }
                    lastNetworkRef.current = { rx: data.network_rx, tx: data.network_tx, time: now };

                    setHistory(prev => {
                        const newDataPoint: PerformanceData = {
                            time: new Date().toLocaleTimeString(),
                            cpu: data.cpu_usage,
                            memory: (data.ram_used / data.ram_total) * 100,
                            swap: data.swap_total > 0 ? (data.swap_used / data.swap_total) * 100 : 0,
                            network_rx: rxRate / 1024, // KB/s
                            network_tx: txRate / 1024, // KB/s
                            ram_used: data.ram_used,
                            ram_total: data.ram_total,
                            swap_used: data.swap_used,
                            swap_total: data.swap_total,
                            cpu_temp: data.cpu_temp,
                            gpus: data.gpus.map(g => ({
                                load: g.load,
                                vram_percent: g.vram_total > 0 ? (g.vram_used / g.vram_total) * 100 : 0,
                                vram_used: g.vram_used,
                                vram_total: g.vram_total,
                                temp: g.temp,
                                name: g.name
                            }))
                        };

                        const newHistory = [...prev, newDataPoint];
                        if (newHistory.length > 60) {
                            newHistory.shift();
                        }
                        return newHistory;
                    });
                    setServerOffline(false);
                } else if (response.status === 401) {
                    logout();
                } else if (response.status >= 500) {
                    setServerOffline(true, `Server error: HTTP ${response.status}`);
                }
            } catch {
                setServerOffline(true, 'Connection to WADM server lost (502 / Offline)');
            }
        };

        fetchStats();
        const interval = setInterval(fetchStats, 2000);

        return () => clearInterval(interval);
    }, [isAuthenticated, logout, setServerOffline]);

    return (
        <StatsContext.Provider value={{ stats, history }}>
            {children}
        </StatsContext.Provider>
    );
}

export function useStats() {
    const context = useContext(StatsContext);
    if (context === undefined) {
        throw new Error('useStats must be used within a StatsProvider');
    }
    return context;
}
