/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useState, useEffect } from 'react';
import type { ReactNode } from 'react';

interface AuthContextType {
    isAuthenticated: boolean;
    token: string | null;
    login: (token: string) => void;
    logout: () => void;
    checkStatus: () => Promise<boolean>; // Returns true if authenticated or setup required (handled by App)
    setupRequired: boolean;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export function useAuth() {
    const context = useContext(AuthContext);
    if (!context) {
        throw new Error('useAuth must be used within an AuthProvider');
    }
    return context;
}

export function AuthProvider({ children }: { children: ReactNode }) {
    const [token, setToken] = useState<string | null>(localStorage.getItem('wadm_token'));
    const [isAuthenticated, setIsAuthenticated] = useState<boolean>(!!token);
    const [setupRequired, setSetupRequired] = useState<boolean>(false);

    const login = (newToken: string) => {
        localStorage.setItem('wadm_token', newToken);
        setToken(newToken);
        setIsAuthenticated(true);
        setSetupRequired(false);
    };

    const logout = () => {
        localStorage.removeItem('wadm_token');
        setToken(null);
        setIsAuthenticated(false);
    };

    const checkStatus = async () => {
        try {
            const res = await fetch('/api/auth/status');
            if (res.ok) {
                const data = await res.json();
                setSetupRequired(data.setup_required);
                return true;
            }
        } catch (err) {
            console.error("Auth status check failed", err);
        }
        return false;
    };

    // Intercept fetch to add header? Or just rely on user adding it? 
    // Ideally we'd wrap fetch or provide an axios instance, but global fetch patch is risky.
    // For now, let's assume components use a helper or manual header.
    // Actually, let's monkey patch window.fetch to insert token automatically.

    useEffect(() => {
        const originalFetch = window.fetch;
        window.fetch = async (...args) => {
            const [resource, config] = args;
            const newConfig = config || {};

            if (token) {
                newConfig.headers = {
                    ...newConfig.headers,
                    'Authorization': `Bearer ${token}`
                };
            }

            const response = await originalFetch(resource, newConfig);

            if (response.status === 401) {
                // Token expired or invalid
                logout();
            }

            return response;
        };

        return () => {
            window.fetch = originalFetch;
        };
    }, [token]);

    useEffect(() => {
        const initAuth = async () => {
            if (!token) {
                await checkStatus();
            }
        };
        initAuth();
    }, [token]);

    return (
        <AuthContext.Provider value={{ isAuthenticated, token, login, logout, checkStatus, setupRequired }}>
            {children}
        </AuthContext.Provider>
    );
}
