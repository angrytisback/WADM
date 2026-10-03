/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useState, useEffect, useRef, useCallback } from 'react';
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

    // We use a ref to hold the token so the fetch interceptor can access the latest value
    // without needing to be re-bound on every render (which causes race conditions with child effects).
    const tokenRef = useRef<string | null>(token);

    const login = useCallback((newToken: string) => {
        localStorage.setItem('wadm_token', newToken);
        tokenRef.current = newToken; // Update immediately
        setToken(newToken);
        setIsAuthenticated(true);
        setSetupRequired(false);
    }, []);

    const logout = useCallback(() => {
        localStorage.removeItem('wadm_token');
        tokenRef.current = null; // Update immediately
        setToken(null);
        setIsAuthenticated(false);
    }, []);

    const checkStatus = useCallback(async () => {
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
    }, []);

    // Monkey patch window.fetch to insert token automatically.
    useEffect(() => {
        const originalFetch = window.fetch;
        window.fetch = async (...args) => {
            const [resource, config] = args;
            const newConfig = config || {};

            const currentToken = tokenRef.current;
            const urlString = typeof resource === 'string' ? resource : resource instanceof Request ? resource.url : '';
            const isInternalOrRelative = urlString.startsWith('/') || urlString.startsWith(window.location.origin);

            if (currentToken && isInternalOrRelative) {
                newConfig.headers = {
                    ...newConfig.headers,
                    'Authorization': `Bearer ${currentToken}`
                };
            }

            const response = await originalFetch(resource, newConfig);

            if (response.status === 401) {
                logout();
            }

            return response;
        };

        return () => {
            window.fetch = originalFetch;
        };
    }, [logout]);

    useEffect(() => {
        const initAuth = async () => {
            if (!token) {
                await checkStatus();
            }
        };
        initAuth();
    }, [token, checkStatus]);

    return (
        <AuthContext.Provider value={{ isAuthenticated, token, login, logout, checkStatus, setupRequired }}>
            {children}
        </AuthContext.Provider>
    );
}
