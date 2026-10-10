/* eslint-disable react-refresh/only-export-components */
import { createContext, useContext, useState, useEffect, useRef, useCallback } from 'react';
import type { ReactNode } from 'react';
import type { UserRole } from '../types';

interface JwtPayload {
    sub: string;
    role: UserRole;
    exp: number;
    iat: number;
}

function parseJwt(token: string | null): { username: string; role: UserRole } | null {
    if (!token) return null;
    try {
        const parts = token.split('.');
        if (parts.length !== 3) return null;
        const payloadStr = atob(parts[1].replace(/-/g, '+').replace(/_/g, '/'));
        const payload: JwtPayload = JSON.parse(payloadStr);
        return {
            username: payload.sub || 'user',
            role: payload.role || 'viewer',
        };
    } catch {
        return null;
    }
}

interface AuthContextType {
    isAuthenticated: boolean;
    isLoading: boolean;
    token: string | null;
    user: { username: string; role: UserRole } | null;
    role: UserRole;
    canOperate: () => boolean;
    isAdmin: () => boolean;
    login: (token?: string, user?: { username: string; role: UserRole }) => void;
    logout: () => void;
    checkStatus: () => Promise<boolean>;
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
    const [isLoading, setIsLoading] = useState<boolean>(true);
    const [token, setToken] = useState<string | null>(null);
    const [userInfo, setUserInfo] = useState<{ username: string; role: UserRole } | null>(null);
    const [isAuthenticated, setIsAuthenticated] = useState<boolean>(false);
    const [setupRequired, setSetupRequired] = useState<boolean>(false);

    // Ref to hold optional token for backward compatibility in headers
    const tokenRef = useRef<string | null>(token);

    const login = useCallback((newToken?: string, user?: { username: string; role: UserRole }) => {
        if (user) {
            setUserInfo(user);
        } else if (newToken) {
            setUserInfo(parseJwt(newToken));
        }
        if (newToken) {
            tokenRef.current = newToken;
            setToken(newToken);
        }
        setIsAuthenticated(true);
        setSetupRequired(false);
    }, []);

    const logout = useCallback(async () => {
        try {
            await fetch('/api/auth/logout', { method: 'POST', credentials: 'include' });
        } catch (err) {
            console.error("Logout request failed", err);
        } finally {
            tokenRef.current = null;
            setToken(null);
            setUserInfo(null);
            setIsAuthenticated(false);
        }
    }, []);

    const checkStatus = useCallback(async () => {
        try {
            const res = await fetch('/api/auth/status', { credentials: 'include' });
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

    // Intercept fetch to automatically attach credentials: 'include' for relative/internal requests
    useEffect(() => {
        const originalFetch = window.fetch;
        window.fetch = async (...args) => {
            const [resource, config] = args;
            const newConfig: RequestInit = { ...config };

            const urlString = typeof resource === 'string' ? resource : resource instanceof Request ? resource.url : '';
            const isInternalOrRelative = urlString.startsWith('/') || urlString.startsWith(window.location.origin);

            if (isInternalOrRelative) {
                newConfig.credentials = 'include';
                const currentToken = tokenRef.current;
                if (currentToken) {
                    newConfig.headers = {
                        ...newConfig.headers,
                        'Authorization': `Bearer ${currentToken}`
                    };
                }
            }

            const response = await originalFetch(resource, newConfig);

            if (
                response.status === 401 &&
                !urlString.includes('/api/auth/login') &&
                !urlString.includes('/api/auth/status') &&
                !urlString.includes('/api/auth/me') &&
                !urlString.includes('/api/auth/logout')
            ) {
                logout();
            }

            return response;
        };

        return () => {
            window.fetch = originalFetch;
        };
    }, [logout]);

    useEffect(() => {
        let isMounted = true;
        const initAuth = async () => {
            try {
                const res = await fetch('/api/auth/me', { credentials: 'include' });
                if (res.ok) {
                    const data = await res.json();
                    if (isMounted) {
                        setUserInfo({
                            username: data.username,
                            role: (data.role as UserRole) || 'viewer',
                        });
                        setIsAuthenticated(true);
                        setIsLoading(false);
                    }
                    return;
                }
            } catch {
                // Ignore network error on startup
            }

            if (isMounted) {
                await checkStatus();
                setIsAuthenticated(false);
                setUserInfo(null);
                setIsLoading(false);
            }
        };

        initAuth();
        return () => {
            isMounted = false;
        };
    }, [checkStatus]);

    const role: UserRole = userInfo?.role || 'viewer';
    const canOperate = useCallback(() => role === 'operator' || role === 'admin', [role]);
    const isAdmin = useCallback(() => role === 'admin', [role]);

    return (
        <AuthContext.Provider
            value={{
                isAuthenticated,
                isLoading,
                token,
                user: userInfo,
                role,
                canOperate,
                isAdmin,
                login,
                logout,
                checkStatus,
                setupRequired,
            }}
        >
            {children}
        </AuthContext.Provider>
    );
}
