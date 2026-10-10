/* eslint-disable react-refresh/only-export-components */
import React, { createContext, useContext, useState, useEffect, useRef, useCallback } from 'react';
import type { SystemStats } from '../types';
import { useAuth } from './AuthContext';
import { useCluster } from './ClusterContext';
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
    refreshInterval: number;
    setRefreshInterval: (ms: number) => void;
}

const StatsContext = createContext<StatsContextType | undefined>(undefined);

export function StatsProvider({ children }: { children: React.ReactNode }) {
    const { isAuthenticated, logout, token } = useAuth();
    const { setServerOffline } = useServerStatus();
    const { activeNode } = useCluster();
    const [stats, setStats] = useState<SystemStats | null>(null);
    const [refreshInterval, setRefreshIntervalState] = useState<number>(() => {
        const saved = localStorage.getItem('wadm_stats_interval');
        return saved ? parseInt(saved, 10) : 2000;
    });

    const setRefreshInterval = (ms: number) => {
        setRefreshIntervalState(ms);
        localStorage.setItem('wadm_stats_interval', ms.toString());
    };

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

    // Reset charts and stats when active node changes
    useEffect(() => {
        setStats(null);
        lastNetworkRef.current = null;
        setHistory(Array(60).fill({
            time: '', cpu: 0, memory: 0, swap: 0, network_rx: 0, network_tx: 0,
            ram_used: 0, ram_total: 0, swap_used: 0, swap_total: 0,
            cpu_temp: 0,
            gpus: []
        }));
    }, [activeNode?.id]);

    const processStatsUpdate = useCallback((data: SystemStats) => {
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
                memory: data.ram_total > 0 ? (data.ram_used / data.ram_total) * 100 : 0,
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
    }, [setServerOffline]);

    useEffect(() => {
        if (!isAuthenticated) {
            setStats(null);
            return;
        }

        // Remote Cluster Node polling mode
        if (activeNode) {
            let isCancelled = false;
            const fetchNodeStats = async () => {
                if (isCancelled) return;
                try {
                    const res = await fetch(`/api/cluster/nodes/${activeNode.id}/stats`, {
                        headers: {
                            ...(token ? { 'Authorization': `Bearer ${token}` } : {})
                        },
                        credentials: 'include',
                    });
                    if (res.ok) {
                        const data: SystemStats = await res.json();
                        processStatsUpdate(data);
                    } else if (res.status === 401) {
                        logout();
                    } else {
                        setServerOffline(true, `Cluster node '${activeNode.name}' is unreachable`);
                    }
                } catch {
                    if (!isCancelled) {
                        setServerOffline(true, `Failed to query node '${activeNode.name}'`);
                    }
                }
            };

            fetchNodeStats();
            const interval = setInterval(fetchNodeStats, refreshInterval);
            return () => {
                isCancelled = true;
                clearInterval(interval);
            };
        }

        // Local Master SSE mode
        let isCancelled = false;
        let abortController = new AbortController();
        let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
        let reconnectDelay = 1000;

        const connectStream = async () => {
            if (isCancelled) return;
            abortController = new AbortController();

            try {
                const response = await fetch('/api/stats/stream', {
                    headers: {
                        'Accept': 'text/event-stream',
                        ...(token ? { 'Authorization': `Bearer ${token}` } : {})
                    },
                    credentials: 'include',
                    signal: abortController.signal,
                });

                if (response.status === 401) {
                    logout();
                    return;
                }

                if (!response.ok) {
                    throw new Error(`SSE stream connection failed with HTTP ${response.status}`);
                }

                if (!response.body) {
                    throw new Error('Response body is null or not a readable stream');
                }

                // Connection successfully established
                reconnectDelay = 1000;
                setServerOffline(false);

                const reader = response.body.getReader();
                const decoder = new TextDecoder('utf-8');
                let buffer = '';

                while (!isCancelled) {
                    const { done, value } = await reader.read();
                    if (done) break;

                    buffer += decoder.decode(value, { stream: true });
                    const events = buffer.split('\n\n');
                    // The trailing element is either incomplete or empty
                    buffer = events.pop() || '';

                    for (const event of events) {
                        const lines = event.split('\n');
                        for (const line of lines) {
                            if (line.startsWith('data:')) {
                                const jsonStr = line.replace(/^data:\s*/, '').trim();
                                if (jsonStr) {
                                    try {
                                        const parsed: SystemStats = JSON.parse(jsonStr);
                                        processStatsUpdate(parsed);
                                    } catch (err) {
                                        console.error('Error parsing SSE telemetry payload:', err);
                                    }
                                }
                            }
                        }
                    }
                }
            } catch (err: unknown) {
                if (isCancelled) return;

                if (err instanceof DOMException && err.name === 'AbortError') {
                    return;
                }

                console.warn('Telemetry SSE connection interrupted, attempting reconnect...', err);
                setServerOffline(true, 'Connection to WADM server lost (Reconnecting stream...)');
            }

            // Exponential backoff reconnect
            if (!isCancelled) {
                reconnectTimeout = setTimeout(() => {
                    reconnectDelay = Math.min(reconnectDelay * 1.5, 10000);
                    connectStream();
                }, reconnectDelay);
            }
        };

        connectStream();

        return () => {
            isCancelled = true;
            abortController.abort();
            if (reconnectTimeout) {
                clearTimeout(reconnectTimeout);
            }
        };
    }, [isAuthenticated, token, activeNode, refreshInterval, logout, setServerOffline, processStatsUpdate]);

    return (
        <StatsContext.Provider value={{ stats, history, refreshInterval, setRefreshInterval }}>
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
