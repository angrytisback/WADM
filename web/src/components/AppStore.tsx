import React, { useEffect, useState, useCallback } from 'react';
import { FaPlay, FaSpinner, FaKey, FaTrash, FaTimes, FaCopy, FaCheck } from 'react-icons/fa';
import { useToast } from '../context/ToastContext';

interface AppTemplate {
  id: string;
  name: string;
  description: string;
  icon: string;
  default_ports?: number[];
}

interface CredentialState {
  appId: string;
  appName: string;
  credentials: string;
}

export const AppStore: React.FC = () => {
  const [apps, setApps] = useState<AppTemplate[]>([]);
  const [installedMap, setInstalledMap] = useState<Record<string, boolean>>({});
  const [loading, setLoading] = useState(true);
  const [installing, setInstalling] = useState<Record<string, boolean>>({});
  const [activeCreds, setActiveCreds] = useState<CredentialState | null>(null);
  const [copied, setCopied] = useState(false);
  const { addToast } = useToast();

  const checkAppStatus = useCallback(async (appId: string) => {
    try {
      const res = await fetch(`/api/apps/${appId}/credentials`);
      if (res.ok) {
        const data = await res.json();
        return data.installed;
      }
    } catch {
      // ignore
    }
    return false;
  }, []);

  const fetchApps = useCallback(async () => {
    try {
      const res = await fetch('/api/apps');
      if (!res.ok) throw new Error('Failed to fetch apps');
      const data: AppTemplate[] = await res.json();
      setApps(data);

      const statusMap: Record<string, boolean> = {};
      for (const app of data) {
        statusMap[app.id] = await checkAppStatus(app.id);
      }
      setInstalledMap(statusMap);
    } catch {
      addToast('Failed to fetch apps', 'error');
    } finally {
      setLoading(false);
    }
  }, [addToast, checkAppStatus]);

  useEffect(() => {
    fetchApps();
  }, [fetchApps]);

  const installApp = async (id: string, name: string) => {
    if (!window.confirm(`Are you sure you want to install ${name}?`)) return;
    
    setInstalling((prev) => ({ ...prev, [id]: true }));
    try {
      const res = await fetch('/api/apps/install', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ id })
      });
      const data = await res.json();
      if (res.ok) {
        addToast(typeof data === 'string' ? data : `${name} installation started`, 'success');
        setInstalledMap((prev) => ({ ...prev, [id]: true }));
        // Automatically fetch credentials after install
        showCredentials(id, name);
      } else {
        addToast(typeof data === 'string' ? data : `Failed to install ${name}`, 'error');
      }
    } catch {
      addToast(`Failed to install ${name}`, 'error');
    } finally {
      setInstalling((prev) => ({ ...prev, [id]: false }));
    }
  };

  const uninstallApp = async (id: string, name: string) => {
    if (!window.confirm(`Are you sure you want to uninstall and remove ${name}? This will stop and remove its containers.`)) return;

    setInstalling((prev) => ({ ...prev, [id]: true }));
    try {
      const res = await fetch(`/api/apps/${id}/uninstall`, {
        method: 'POST',
      });
      if (res.ok) {
        addToast(`${name} has been removed.`, 'success');
        setInstalledMap((prev) => ({ ...prev, [id]: false }));
      } else {
        addToast(`Failed to uninstall ${name}`, 'error');
      }
    } catch {
      addToast(`Failed to uninstall ${name}`, 'error');
    } finally {
      setInstalling((prev) => ({ ...prev, [id]: false }));
    }
  };

  const showCredentials = async (id: string, name: string) => {
    try {
      const res = await fetch(`/api/apps/${id}/credentials`);
      if (res.ok) {
        const data = await res.json();
        if (data.credentials) {
          setActiveCreds({
            appId: id,
            appName: name,
            credentials: data.credentials,
          });
          setCopied(false);
          return;
        }
      }
      addToast(`No stored credentials found for ${name}`, 'warning');
    } catch {
      addToast('Failed to retrieve credentials', 'error');
    }
  };

  const copyToClipboard = () => {
    if (activeCreds) {
      navigator.clipboard.writeText(activeCreds.credentials);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem', padding: '0 1rem' }} className="fade-in">
      <div>
        <h2 style={{ fontSize: '1.75rem', fontWeight: 700, margin: 0, color: 'var(--text-primary)' }}>App Store</h2>
        <p style={{ color: 'var(--text-secondary)', fontSize: '0.95rem', marginTop: '0.25rem' }}>
          One-click deploy and manage popular Dockerized applications with isolated volumes and secure auto-generated credentials.
        </p>
      </div>

      {loading ? (
        <div style={{ display: 'flex', justifyContent: 'center', padding: '3rem' }}>
          <FaSpinner className="animate-spin" style={{ fontSize: '2rem', color: 'var(--accent-color)' }} />
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(300px, 1fr))', gap: '1.5rem' }}>
          {apps.map((app) => {
            const isInstalled = installedMap[app.id];
            const isBusy = installing[app.id];

            return (
              <div key={app.id} className="glass-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', alignItems: 'center', textAlign: 'center', position: 'relative' }}>
                {isInstalled && (
                  <span style={{
                    position: 'absolute',
                    top: '12px',
                    right: '12px',
                    background: 'rgba(16, 185, 129, 0.2)',
                    color: '#10b981',
                    border: '1px solid #10b981',
                    padding: '2px 8px',
                    borderRadius: '12px',
                    fontSize: '0.75rem',
                    fontWeight: 600
                  }}>
                    Installed
                  </span>
                )}

                <img src={app.icon} alt={app.name} style={{ width: '64px', height: '64px', marginBottom: '1rem', objectFit: 'contain' }} />
                <h3 style={{ fontSize: '1.2rem', fontWeight: 700, color: 'var(--text-primary)', marginBottom: '0.35rem' }}>{app.name}</h3>
                {app.default_ports && app.default_ports.length > 0 && (
                  <div style={{ fontSize: '0.75rem', color: 'var(--accent-color)', marginBottom: '0.5rem', fontFamily: 'monospace' }}>
                    Port: {app.default_ports.join(', ')}
                  </div>
                )}
                <p style={{ fontSize: '0.9rem', color: 'var(--text-secondary)', marginBottom: '1.5rem', flexGrow: 1 }}>{app.description}</p>
                
                <div style={{ width: '100%', display: 'flex', gap: '0.5rem', flexDirection: 'column' }}>
                  {!isInstalled ? (
                    <button
                      onClick={() => installApp(app.id, app.name)}
                      disabled={isBusy}
                      className="btn primary"
                      style={{
                        width: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        gap: '0.5rem',
                        padding: '0.6rem 1rem'
                      }}
                    >
                      {isBusy ? (
                        <>
                          <FaSpinner className="animate-spin" />
                          <span>Installing...</span>
                        </>
                      ) : (
                        <>
                          <FaPlay style={{ fontSize: '0.8rem' }} />
                          <span>Deploy / Install</span>
                        </>
                      )}
                    </button>
                  ) : (
                    <div style={{ display: 'flex', gap: '0.5rem', width: '100%' }}>
                      <button
                        onClick={() => showCredentials(app.id, app.name)}
                        className="btn secondary"
                        style={{
                          flex: 1,
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center',
                          gap: '0.4rem',
                          padding: '0.55rem 0.75rem',
                          fontSize: '0.85rem'
                        }}
                        title="View generated login credentials"
                      >
                        <FaKey />
                        <span>Credentials</span>
                      </button>
                      <button
                        onClick={() => uninstallApp(app.id, app.name)}
                        disabled={isBusy}
                        className="btn danger"
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center',
                          gap: '0.4rem',
                          padding: '0.55rem 0.75rem',
                          fontSize: '0.85rem'
                        }}
                        title="Uninstall and remove container"
                      >
                        {isBusy ? <FaSpinner className="animate-spin" /> : <FaTrash />}
                        <span>Remove</span>
                      </button>
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Credentials Modal */}
      {activeCreds && (
        <div style={{
          position: 'fixed',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          background: 'rgba(0, 0, 0, 0.7)',
          backdropFilter: 'blur(4px)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          zIndex: 9999,
          padding: '1rem'
        }}>
          <div className="glass-panel" style={{
            background: '#0f172a',
            border: '1px solid var(--glass-border)',
            borderRadius: '12px',
            width: '100%',
            maxWidth: '520px',
            padding: '1.5rem',
            boxShadow: '0 20px 25px -5px rgba(0, 0, 0, 0.5)',
          }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem', borderBottom: '1px solid rgba(255,255,255,0.1)', paddingBottom: '0.75rem' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                <FaKey style={{ color: 'var(--accent-color)' }} />
                <h3 style={{ margin: 0, fontSize: '1.15rem', color: '#f8fafc' }}>
                  {activeCreds.appName} - Credentials
                </h3>
              </div>
              <button
                onClick={() => setActiveCreds(null)}
                style={{ background: 'transparent', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer', fontSize: '1.1rem' }}
              >
                <FaTimes />
              </button>
            </div>

            <p style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginBottom: '1rem' }}>
              Secure credentials generated by WADM during initial setup. Keep these safe:
            </p>

            <pre style={{
              background: '#020617',
              border: '1px solid rgba(255, 255, 255, 0.08)',
              borderRadius: '8px',
              padding: '1rem',
              color: '#38bdf8',
              fontFamily: 'monospace',
              fontSize: '0.9rem',
              overflowX: 'auto',
              whiteSpace: 'pre-wrap',
              marginBottom: '1.25rem'
            }}>
              {activeCreds.credentials}
            </pre>

            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.75rem' }}>
              <button
                onClick={copyToClipboard}
                className="btn primary"
                style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.5rem 1rem' }}
              >
                {copied ? <FaCheck /> : <FaCopy />}
                <span>{copied ? 'Copied!' : 'Copy to Clipboard'}</span>
              </button>
              <button
                onClick={() => setActiveCreds(null)}
                className="btn secondary"
                style={{ padding: '0.5rem 1rem' }}
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
