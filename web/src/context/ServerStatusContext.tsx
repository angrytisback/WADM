import React, { createContext, useContext, useState, useCallback, useRef } from 'react';

interface ServerStatusContextType {
  isRebooting: boolean;
  isShuttingDown: boolean;
  isServerOffline: boolean;
  offlineReason: string | null;
  triggerReboot: () => void;
  triggerShutdown: () => void;
  setServerOffline: (offline: boolean, reason?: string) => void;
  checkConnection: () => Promise<boolean>;
}

const ServerStatusContext = createContext<ServerStatusContextType | undefined>(undefined);

export const ServerStatusProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [isRebooting, setIsRebooting] = useState(false);
  const [isShuttingDown, setIsShuttingDown] = useState(false);
  const [isServerOffline, setIsServerOfflineState] = useState(false);
  const [offlineReason, setOfflineReason] = useState<string | null>(null);
  const failureCountRef = useRef(0);

  const triggerReboot = useCallback(() => {
    setIsRebooting(true);
    setIsShuttingDown(false);
    setIsServerOfflineState(false);
  }, []);

  const triggerShutdown = useCallback(() => {
    setIsShuttingDown(true);
    setIsRebooting(false);
    setIsServerOfflineState(false);
  }, []);

  const setServerOffline = useCallback((offline: boolean, reason?: string) => {
    if (offline) {
      failureCountRef.current += 1;
      // Trigger offline screen after 3 consecutive failures if not in reboot/shutdown mode
      if (failureCountRef.current >= 3) {
        setIsServerOfflineState(true);
        setOfflineReason(reason || 'Unable to connect to WADM server (502 / Network Error)');
      }
    } else {
      failureCountRef.current = 0;
      setIsServerOfflineState(false);
      setOfflineReason(null);
    }
  }, []);

  const checkConnection = useCallback(async (): Promise<boolean> => {
    try {
      const res = await fetch('/api/stats', { cache: 'no-store' });
      if (res.ok) {
        failureCountRef.current = 0;
        setIsServerOfflineState(false);
        setOfflineReason(null);
        return true;
      }
      return false;
    } catch {
      return false;
    }
  }, []);

  return (
    <ServerStatusContext.Provider
      value={{
        isRebooting,
        isShuttingDown,
        isServerOffline,
        offlineReason,
        triggerReboot,
        triggerShutdown,
        setServerOffline,
        checkConnection
      }}
    >
      {children}
    </ServerStatusContext.Provider>
  );
};

export const useServerStatus = () => {
  const context = useContext(ServerStatusContext);
  if (!context) {
    throw new Error('useServerStatus must be used within a ServerStatusProvider');
  }
  return context;
};
