import React, { useState, useEffect, useRef } from 'react';
import { useServerStatus } from '../context/ServerStatusContext';
import { 
  FaSync, FaPowerOff, FaCheckCircle, 
  FaExclamationTriangle, FaSpinner, FaRedo 
} from 'react-icons/fa';

export const ServerStatusOverlay: React.FC = () => {
  const { 
    isRebooting, 
    isShuttingDown, 
    isServerOffline, 
    offlineReason, 
    checkConnection 
  } = useServerStatus();

  const [elapsed, setElapsed] = useState(0);
  const [rebootOnline, setRebootOnline] = useState(false);
  const [isRetrying, setIsRetrying] = useState(false);
  const [pingAttempts, setPingAttempts] = useState(0);
  const startTimeRef = useRef<number>(Date.now());

  // Timer for reboot elapsed time
  useEffect(() => {
    if (!isRebooting) {
      setElapsed(0);
      setRebootOnline(false);
      setPingAttempts(0);
      return;
    }

    startTimeRef.current = Date.now();
    const timer = setInterval(() => {
      setElapsed(Math.floor((Date.now() - startTimeRef.current) / 1000));
    }, 1000);

    return () => clearInterval(timer);
  }, [isRebooting]);

  // Active polling during reboot (wait at least 8s after reboot trigger before checking for comeback)
  useEffect(() => {
    if (!isRebooting || rebootOnline) return;

    const pollInterval = setInterval(async () => {
      // Only start checking after 6 seconds to give the server time to terminate
      if (Date.now() - startTimeRef.current < 6000) return;

      setPingAttempts(prev => prev + 1);
      try {
        const res = await fetch('/api/stats', { cache: 'no-store' });
        if (res.ok) {
          setRebootOnline(true);
          clearInterval(pollInterval);
          setTimeout(() => {
            window.location.reload();
          }, 1500);
        }
      } catch {
        // Still rebooting
      }
    }, 2000);

    return () => clearInterval(pollInterval);
  }, [isRebooting, rebootOnline]);

  // Periodic ping when server is offline
  useEffect(() => {
    if (!isServerOffline || isRebooting || isShuttingDown) return;

    const interval = setInterval(async () => {
      const ok = await checkConnection();
      if (ok) {
        window.location.reload();
      }
    }, 4000);

    return () => clearInterval(interval);
  }, [isServerOffline, isRebooting, isShuttingDown, checkConnection]);

  const handleManualRetry = async () => {
    setIsRetrying(true);
    const online = await checkConnection();
    setIsRetrying(false);
    if (online) {
      window.location.reload();
    }
  };

  // If everything is normal, render nothing
  if (!isRebooting && !isShuttingDown && !isServerOffline) {
    return null;
  }

  // 1. REBOOTING OVERLAY (Firmware upgrade style)
  if (isRebooting) {
    let statusText = 'Initiating reboot sequence...';
    if (elapsed > 4 && elapsed <= 14) {
      statusText = 'Restarting operating system & systemd services...';
    } else if (elapsed > 14 && !rebootOnline) {
      statusText = `Waiting for network & WADM service to initialize... (Probe #${pingAttempts})`;
    } else if (rebootOnline) {
      statusText = 'Server is back online! Reloading interface...';
    }

    return (
      <div style={{
        position: 'fixed',
        inset: 0,
        zIndex: 99999,
        background: 'rgba(10, 15, 29, 0.96)',
        backdropFilter: 'blur(20px)',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '2rem',
        color: '#fff',
        userSelect: 'none'
      }}>
        <div style={{
          maxWidth: '520px',
          width: '100%',
          background: 'rgba(23, 32, 54, 0.8)',
          border: '1px solid rgba(56, 189, 248, 0.25)',
          borderRadius: '16px',
          padding: '2.5rem',
          boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.7), 0 0 35px rgba(56, 189, 248, 0.15)',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          textAlign: 'center'
        }}>
          {/* Animated Header Icon */}
          <div style={{
            width: '80px',
            height: '80px',
            borderRadius: '50%',
            background: rebootOnline ? 'rgba(52, 211, 153, 0.15)' : 'rgba(56, 189, 248, 0.15)',
            border: `2px solid ${rebootOnline ? '#34d399' : 'rgba(56, 189, 248, 0.4)'}`,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            marginBottom: '1.5rem',
            boxShadow: `0 0 25px ${rebootOnline ? 'rgba(52, 211, 153, 0.3)' : 'rgba(56, 189, 248, 0.25)'}`
          }}>
            {rebootOnline ? (
              <FaCheckCircle style={{ fontSize: '2.5rem', color: '#34d399' }} />
            ) : (
              <FaSync className="animate-spin" style={{ fontSize: '2.2rem', color: 'var(--accent-color)' }} />
            )}
          </div>

          <h2 style={{ fontSize: '1.75rem', fontWeight: 800, margin: '0 0 0.5rem 0', letterSpacing: '-0.02em' }}>
            {rebootOnline ? 'System Reconnected!' : 'System is Rebooting'}
          </h2>
          
          <p style={{ color: 'var(--text-secondary)', fontSize: '0.95rem', margin: '0 0 2rem 0', lineHeight: 1.5 }}>
            {rebootOnline 
              ? 'WADM server responded with 200 OK. Preparing your session...'
              : 'The server is currently restarting hardware and services. Please do not close this window or power off the machine.'}
          </p>

          {/* Firmware-Style Pulsing Progress Bar */}
          <div style={{
            width: '100%',
            height: '8px',
            background: 'rgba(255, 255, 255, 0.08)',
            borderRadius: '4px',
            overflow: 'hidden',
            marginBottom: '1.5rem',
            position: 'relative'
          }}>
            <div style={{
              height: '100%',
              width: rebootOnline ? '100%' : `${Math.min(15 + elapsed * 2.8, 95)}%`,
              background: rebootOnline 
                ? 'linear-gradient(90deg, #10b981, #34d399)'
                : 'linear-gradient(90deg, #0284c7, #38bdf8, #818cf8)',
              borderRadius: '4px',
              transition: 'width 0.5s ease-out',
              boxShadow: '0 0 12px rgba(56, 189, 248, 0.5)'
            }} />
          </div>

          {/* Live Status and Timer */}
          <div style={{
            width: '100%',
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            fontSize: '0.85rem',
            color: 'var(--text-secondary)',
            background: 'rgba(0, 0, 0, 0.25)',
            padding: '0.75rem 1rem',
            borderRadius: '8px',
            border: '1px solid rgba(255, 255, 255, 0.05)'
          }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: rebootOnline ? '#34d399' : '#e2e8f0' }}>
              <span style={{
                width: '8px',
                height: '8px',
                borderRadius: '50%',
                background: rebootOnline ? '#34d399' : '#38bdf8',
                boxShadow: `0 0 8px ${rebootOnline ? '#34d399' : '#38bdf8'}`
              }} />
              {statusText}
            </span>
            <span style={{ fontFamily: 'monospace', fontWeight: 700, color: 'var(--accent-color)' }}>
              {elapsed}s
            </span>
          </div>
        </div>
      </div>
    );
  }

  // 2. SHUTDOWN OVERLAY
  if (isShuttingDown) {
    return (
      <div style={{
        position: 'fixed',
        inset: 0,
        zIndex: 99999,
        background: 'rgba(10, 15, 29, 0.97)',
        backdropFilter: 'blur(20px)',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        padding: '2rem',
        color: '#fff'
      }}>
        <div style={{
          maxWidth: '480px',
          width: '100%',
          background: 'rgba(23, 32, 54, 0.8)',
          border: '1px solid rgba(248, 113, 113, 0.3)',
          borderRadius: '16px',
          padding: '2.5rem',
          textAlign: 'center',
          boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.7)'
        }}>
          <div style={{
            width: '72px',
            height: '72px',
            borderRadius: '50%',
            background: 'rgba(248, 113, 113, 0.15)',
            border: '2px solid rgba(248, 113, 113, 0.4)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            margin: '0 auto 1.5rem auto'
          }}>
            <FaPowerOff style={{ fontSize: '2rem', color: 'var(--danger)' }} />
          </div>

          <h2 style={{ fontSize: '1.75rem', fontWeight: 800, margin: '0 0 0.5rem 0' }}>Server Powered Down</h2>
          <p style={{ color: 'var(--text-secondary)', fontSize: '0.95rem', margin: '0 0 2rem 0', lineHeight: 1.5 }}>
            The shutdown sequence has completed. The server hardware is now powered off.
            To use WADM again, manually turn on the server machine.
          </p>

          <button
            onClick={handleManualRetry}
            disabled={isRetrying}
            className="btn primary"
            style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '0.5rem', width: '100%', padding: '0.75rem' }}
          >
            {isRetrying ? <FaSpinner className="animate-spin" /> : <FaRedo />}
            <span>{isRetrying ? 'Checking Connection...' : 'Check If Server Is Online'}</span>
          </button>
        </div>
      </div>
    );
  }

  // 3. UNEXPECTED SERVER OFFLINE (502 / Connection dropped)
  return (
    <div style={{
      position: 'fixed',
      bottom: '1.5rem',
      right: '1.5rem',
      zIndex: 99999,
      maxWidth: '420px',
      width: 'calc(100vw - 3rem)',
      background: 'rgba(23, 32, 54, 0.95)',
      backdropFilter: 'blur(16px)',
      border: '1px solid rgba(248, 113, 113, 0.4)',
      borderRadius: '12px',
      padding: '1.25rem',
      boxShadow: '0 20px 25px -5px rgba(0, 0, 0, 0.6), 0 0 20px rgba(248, 113, 113, 0.2)',
      color: '#fff'
    }}>
      <div style={{ display: 'flex', alignItems: 'flex-start', gap: '0.75rem' }}>
        <div style={{
          width: '36px',
          height: '36px',
          borderRadius: '8px',
          background: 'rgba(248, 113, 113, 0.15)',
          color: 'var(--danger)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: 0,
          marginTop: '0.2rem'
        }}>
          <FaExclamationTriangle />
        </div>
        <div style={{ flex: 1 }}>
          <div style={{ fontWeight: 700, fontSize: '1rem', marginBottom: '0.25rem', color: '#fca5a5' }}>
            Server is Offline
          </div>
          <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', lineHeight: 1.4, marginBottom: '0.75rem' }}>
            {offlineReason || 'Unable to communicate with WADM backend service. The server may be restarting or experiencing network issues.'}
          </div>
          <div style={{ display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
            <button
              onClick={handleManualRetry}
              disabled={isRetrying}
              className="btn-sm"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: '0.4rem',
                background: 'rgba(248, 113, 113, 0.2)',
                color: '#fca5a5',
                border: '1px solid rgba(248, 113, 113, 0.4)',
                borderRadius: '6px',
                padding: '0.35rem 0.75rem',
                fontSize: '0.8rem',
                cursor: isRetrying ? 'not-allowed' : 'pointer',
                fontWeight: 600
              }}
            >
              {isRetrying ? <FaSpinner className="animate-spin" /> : <FaRedo />}
              <span>{isRetrying ? 'Reconnecting...' : 'Retry Connection'}</span>
            </button>
            <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>
              Auto-reconnecting...
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};
