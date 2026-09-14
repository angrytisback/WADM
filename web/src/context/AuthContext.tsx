/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useState, useEffect, useRef } from 'react';
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

    const login = (newToken: string) => {
        localStorage.setItem('wadm_token', newToken);
        tokenRef.current = newToken; // Update immediately
        setToken(newToken);
        setIsAuthenticated(true);
        setSetupRequired(false);
    };

    const logout = () => {
        localStorage.removeItem('wadm_token');
        tokenRef.current = null; // Update immediately
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

    // Monkey patch window.fetch to insert token automatically.
    // This effect runs ONCE.
    useEffect(() => {
        const originalFetch = window.fetch;
        window.fetch = async (...args) => {
            const [resource, config] = args;
            const newConfig = config || {};

            const currentToken = tokenRef.current;
            if (currentToken) {
                newConfig.headers = {
                    ...newConfig.headers,
                    'Authorization': `Bearer ${currentToken}`
                };
            }

            const response = await originalFetch(resource, newConfig);

            if (response.status === 401) {
                // Token expired or invalid
                // We cannot call logout() directly here easily because it updates state.
                // But we can manually clear storage and force reload or dispatch event.
                // However, since we define logout inside the component, we can try to call it if we extract logic.
                // BUT, calling state setter from here is fine as long as we don't have stale closures on 'logout'.
                // 'logout' function is recreated on every render? Yes.
                // So we can't use 'logout' from the closure if this effect runs once.

                // Workaround: We will just clear storage and reload the page if it's a 401 on an API call (except login/status).
                // Actually, let's just emit a custom event or let the component handle it?
                // Step 175 StatsContext handles 401 by calling logout() passed via context. 
                // That logout() is the one from the *latest* render context.
                // So we DON'T need to handle 401 logout *here* inside the interceptor if components handle it.
                // BUT the original code did handle it here: "logout();".
                // If we want to support global 401 logout from the interceptor running once, we need a mutable ref to the logout function.

                // Let's rely on components (like StatsContext) handling 401 for now, or use a ref for logout too.
                if (logoutRef.current) {
                    logoutRef.current();
                }
            }

            return response;
        };

        return () => {
            window.fetch = originalFetch;
        };
    }, []);

    // Ref for logout to be used inside the static interceptor
    const logoutRef = useRef(logout);
    useEffect(() => {
        logoutRef.current = logout;
    }, [logout]);


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
