/* eslint-disable react-refresh/only-export-components */
import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import type { ClusterNode, JoinTokenResponse } from '../types';
import { useAuth } from './AuthContext';
import { useToast } from './ToastContext';

interface ClusterContextType {
    activeNode: ClusterNode | null;
    setActiveNode: (node: ClusterNode | null) => void;
    nodes: ClusterNode[];
    isLoading: boolean;
    refreshNodes: () => Promise<void>;
    generateJoinToken: () => Promise<JoinTokenResponse | null>;
    deleteNode: (id: string) => Promise<boolean>;
}

const ClusterContext = createContext<ClusterContextType | undefined>(undefined);

export function ClusterProvider({ children }: { children: React.ReactNode }) {
    const { isAuthenticated, token } = useAuth();
    const { addToast } = useToast();
    const [nodes, setNodes] = useState<ClusterNode[]>([]);
    const [activeNode, setActiveNodeState] = useState<ClusterNode | null>(() => {
        const saved = localStorage.getItem('wadm_active_node');
        if (saved) {
            try {
                return JSON.parse(saved);
            } catch {
                return null;
            }
        }
        return null;
    });
    const [isLoading, setIsLoading] = useState(false);

    const setActiveNode = useCallback((node: ClusterNode | null) => {
        setActiveNodeState(node);
        if (node) {
            localStorage.setItem('wadm_active_node', JSON.stringify(node));
        } else {
            localStorage.removeItem('wadm_active_node');
        }
    }, []);

    const refreshNodes = useCallback(async () => {
        if (!isAuthenticated) return;
        setIsLoading(true);
        try {
            const res = await fetch('/api/cluster/nodes', {
                headers: {
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                credentials: 'include',
            });
            if (res.ok) {
                const data: ClusterNode[] = await res.json();
                setNodes(data);

                // Update activeNode reference if it exists in data
                setActiveNodeState(prev => {
                    if (!prev) return null;
                    const matched = data.find(n => n.id === prev.id);
                    if (matched) {
                        localStorage.setItem('wadm_active_node', JSON.stringify(matched));
                        return matched;
                    }
                    // Node removed from cluster
                    localStorage.removeItem('wadm_active_node');
                    return null;
                });
            }
        } catch (e) {
            console.error('Failed to fetch cluster nodes:', e);
        } finally {
            setIsLoading(false);
        }
    }, [isAuthenticated, token]);

    useEffect(() => {
        if (!isAuthenticated) return;

        const timer = setTimeout(() => {
            void refreshNodes();
        }, 0);

        const interval = setInterval(() => {
            void refreshNodes();
        }, 5000);

        return () => {
            clearTimeout(timer);
            clearInterval(interval);
        };
    }, [isAuthenticated, refreshNodes]);

    const generateJoinToken = useCallback(async (): Promise<JoinTokenResponse | null> => {
        try {
            const res = await fetch('/api/cluster/nodes/generate-token', {
                method: 'POST',
                headers: {
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                    'Content-Type': 'application/json',
                },
                credentials: 'include',
            });
            if (res.ok) {
                const data: JoinTokenResponse = await res.json();
                return data;
            }
            const errData = await res.json().catch(() => ({}));
            addToast(errData.error || 'Failed to generate cluster join token', 'error');
            return null;
        } catch (e) {
            console.error('Error generating join token:', e);
            addToast('Network error while generating token', 'error');
            return null;
        }
    }, [token, addToast]);

    const deleteNode = useCallback(async (id: string): Promise<boolean> => {
        try {
            const res = await fetch(`/api/cluster/nodes/${id}`, {
                method: 'DELETE',
                headers: {
                    ...(token ? { Authorization: `Bearer ${token}` } : {}),
                },
                credentials: 'include',
            });
            if (res.ok) {
                addToast('Node removed from cluster', 'success');
                if (activeNode?.id === id) {
                    setActiveNode(null);
                }
                await refreshNodes();
                return true;
            }
            const err = await res.json().catch(() => ({}));
            addToast(err.error || 'Failed to delete node', 'error');
            return false;
        } catch (e) {
            console.error('Error deleting node:', e);
            addToast('Network error while removing node', 'error');
            return false;
        }
    }, [token, activeNode, setActiveNode, refreshNodes, addToast]);

    return (
        <ClusterContext.Provider
            value={{
                activeNode,
                setActiveNode,
                nodes,
                isLoading,
                refreshNodes,
                generateJoinToken,
                deleteNode,
            }}
        >
            {children}
        </ClusterContext.Provider>
    );
}

export function useCluster() {
    const context = useContext(ClusterContext);
    if (!context) {
        throw new Error('useCluster must be used within a ClusterProvider');
    }
    return context;
}
