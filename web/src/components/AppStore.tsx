import React, { useEffect, useState, useCallback } from 'react';
import {
  FaPlay,
  FaSpinner,
  FaKey,
  FaTrash,
  FaTimes,
  FaCopy,
  FaCheck,
  FaExternalLinkAlt,
  FaShieldAlt,
  FaExclamationTriangle,
  FaNetworkWired,
  FaGlobe,
  FaFolder
} from 'react-icons/fa';
import { useToast } from '../context/ToastContext';
import { useJobs } from '../context/JobContext';
import { useAuth } from '../context/AuthContext';
import type { AppTemplate, AppInstallRequest } from '../types';

interface AppStatus {
  installed: boolean;
  access_url?: string | null;
  access_mode?: string | null;
  internal_port?: number | null;
}

interface CredentialState {
  appId: string;
  appName: string;
  credentials: string;
  accessUrl?: string | null;
  internalPort?: number | null;
}

export const AppStore: React.FC = () => {
  const [apps, setApps] = useState<AppTemplate[]>([]);
  const [statusMap, setStatusMap] = useState<Record<string, AppStatus>>({});
  const [loading, setLoading] = useState(true);
  const [installing, setInstalling] = useState<Record<string, boolean>>({});
  const [activeCreds, setActiveCreds] = useState<CredentialState | null>(null);
  const [copied, setCopied] = useState(false);

  // Install Modal State
  const [installModalApp, setInstallModalApp] = useState<AppTemplate | null>(null);
  const [accessMode, setAccessMode] = useState<'path' | 'subdomain'>('path');
  const [customDomain, setCustomDomain] = useState('');
  const [allowExposedPorts, setAllowExposedPorts] = useState(false);

  const { addToast } = useToast();
  const { trackJob } = useJobs();
  const { canOperate, isAdmin } = useAuth();

  const checkAppStatus = useCallback(async (appId: string): Promise<AppStatus> => {
    try {
      const res = await fetch(`/api/apps/${appId}/credentials`);
      if (res.ok) {
        const data = await res.json();
        return {
          installed: Boolean(data.installed),
          access_url: data.access_url ?? null,
          access_mode: data.access_mode ?? null,
          internal_port: data.internal_port ?? null,
        };
      }
    } catch {
      // ignore
    }
    return { installed: false };
  }, []);

  const fetchApps = useCallback(async () => {
    try {
      const res = await fetch('/api/apps');
      if (!res.ok) throw new Error('Failed to fetch apps');
      const data: AppTemplate[] = await res.json();
      setApps(data);

      const newStatusMap: Record<string, AppStatus> = {};
      for (const app of data) {
        newStatusMap[app.id] = await checkAppStatus(app.id);
      }
      setStatusMap(newStatusMap);
    } catch {
      addToast('Failed to fetch apps', 'error');
    } finally {
      setLoading(false);
    }
  }, [addToast, checkAppStatus]);

  useEffect(() => {
    fetchApps();
  }, [fetchApps]);

  const openInstallModal = (app: AppTemplate) => {
    setInstallModalApp(app);
    setAccessMode(app.default_access_mode || 'path');
    setCustomDomain('');
    setAllowExposedPorts(false);
  };

  const handleConfirmInstall = async () => {
    if (!installModalApp) return;

    const hasExposedPorts = installModalApp.ports.exposed_network_ports.length > 0;
    if (hasExposedPorts && !allowExposedPorts) {
      addToast('Firewall port authorization is required to install this service', 'error');
      return;
    }

    if (accessMode === 'subdomain' && !customDomain.trim()) {
      addToast('Please enter a valid subdomain (e.g. cloud.example.com)', 'error');
      return;
    }

    const appId = installModalApp.id;
    const appName = installModalApp.name;
    const approvedPorts = installModalApp.ports.exposed_network_ports.map((p) => p.port);

    const payload: AppInstallRequest = {
      id: appId,
      access_mode: accessMode,
      domain: accessMode === 'subdomain' ? customDomain.trim() : undefined,
      allow_exposed_ports: allowExposedPorts,
      approved_ports: approvedPorts,
    };

    setInstalling((prev) => ({ ...prev, [appId]: true }));
    setInstallModalApp(null);

    try {
      const res = await fetch('/api/apps/install', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload),
      });
      const data = await res.json();
      if (res.ok) {
        if (data && data.job_id) {
          trackJob(data.job_id, `Deploying ${appName}`);
        }
        addToast(typeof data === 'string' ? data : `${appName} deployment started`, 'success');

        // Update status map
        const status = await checkAppStatus(appId);
        setStatusMap((prev) => ({ ...prev, [appId]: status }));

        // Show credentials modal
        showCredentials(appId, appName);
      } else {
        const errorMsg = data?.message || data?.error || (typeof data === 'string' ? data : `Failed to install ${appName}`);
        addToast(errorMsg, 'error');
      }
    } catch {
      addToast(`Failed to install ${appName}`, 'error');
    } finally {
      setInstalling((prev) => ({ ...prev, [appId]: false }));
    }
  };

  const uninstallApp = async (id: string, name: string) => {
    if (
      !window.confirm(
        `Are you sure you want to uninstall and remove ${name}? This will stop and remove its containers and reverse proxy routes.`
      )
    )
      return;

    setInstalling((prev) => ({ ...prev, [id]: true }));
    try {
      const res = await fetch(`/api/apps/${id}/uninstall`, {
        method: 'POST',
      });
      if (res.ok) {
        addToast(`${name} has been removed.`, 'success');
        setStatusMap((prev) => ({ ...prev, [id]: { installed: false } }));
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
            accessUrl: data.access_url ?? null,
            internalPort: data.internal_port ?? null,
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
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
          <h2 style={{ fontSize: '1.75rem', fontWeight: 700, margin: 0, color: 'var(--text-primary)' }}>App Store</h2>
          <span style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '0.35rem',
            background: 'rgba(16, 185, 129, 0.15)',
            border: '1px solid rgba(16, 185, 129, 0.4)',
            color: '#10b981',
            padding: '2px 10px',
            borderRadius: '16px',
            fontSize: '0.75rem',
            fontWeight: 600,
          }}>
            <FaShieldAlt style={{ fontSize: '0.7rem' }} /> Zero External Port Exposure
          </span>
        </div>
        <p style={{ color: 'var(--text-secondary)', fontSize: '0.95rem', marginTop: '0.35rem' }}>
          1-Click isolated applications routed strictly through WADM&apos;s internal reverse proxy. Web services never bind to external ports; firewall rules require explicit consent.
        </p>
      </div>

      {loading ? (
        <div style={{ display: 'flex', justifyContent: 'center', padding: '3rem' }}>
          <FaSpinner className="animate-spin" style={{ fontSize: '2rem', color: 'var(--accent-color)' }} />
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(320px, 1fr))', gap: '1.5rem' }}>
          {apps.map((app) => {
            const status = statusMap[app.id] || { installed: false };
            const isInstalled = status.installed;
            const isBusy = installing[app.id];
            const hasExposedPorts = app.ports.exposed_network_ports && app.ports.exposed_network_ports.length > 0;
            const hasWebProxy = Boolean(app.ports.internal_web_port);

            return (
              <div
                key={app.id}
                className="glass-panel"
                style={{
                  padding: '1.5rem',
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                  textAlign: 'center',
                  position: 'relative',
                }}
              >
                {isInstalled && (
                  <span
                    style={{
                      position: 'absolute',
                      top: '12px',
                      right: '12px',
                      background: 'rgba(16, 185, 129, 0.2)',
                      color: '#10b981',
                      border: '1px solid #10b981',
                      padding: '2px 8px',
                      borderRadius: '12px',
                      fontSize: '0.75rem',
                      fontWeight: 600,
                    }}
                  >
                    Installed
                  </span>
                )}

                <img
                  src={app.icon}
                  alt={app.name}
                  style={{ width: '64px', height: '64px', marginBottom: '1rem', objectFit: 'contain' }}
                />
                <h3 style={{ fontSize: '1.2rem', fontWeight: 700, color: 'var(--text-primary)', marginBottom: '0.35rem' }}>
                  {app.name}
                </h3>

                {/* Security & Network Badges */}
                <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.35rem', justifyContent: 'center', marginBottom: '0.75rem' }}>
                  {hasWebProxy && (
                    <span
                      title="Bound exclusively to 127.0.0.1. Traffic routed via internal reverse proxy."
                      style={{
                        display: 'inline-flex',
                        alignItems: 'center',
                        gap: '0.3rem',
                        fontSize: '0.7rem',
                        color: '#34d399',
                        background: 'rgba(52, 211, 153, 0.1)',
                        border: '1px solid rgba(52, 211, 153, 0.3)',
                        borderRadius: '6px',
                        padding: '2px 6px',
                        fontFamily: 'monospace',
                      }}
                    >
                      <FaShieldAlt style={{ fontSize: '0.65rem' }} /> Reverse Proxy (127.0.0.1)
                    </span>
                  )}
                  {hasExposedPorts &&
                    app.ports.exposed_network_ports.map((p) => (
                      <span
                        key={p.port}
                        title={p.reason}
                        style={{
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: '0.3rem',
                          fontSize: '0.7rem',
                          color: '#fbbf24',
                          background: 'rgba(251, 191, 36, 0.12)',
                          border: '1px solid rgba(251, 191, 36, 0.35)',
                          borderRadius: '6px',
                          padding: '2px 6px',
                          fontFamily: 'monospace',
                        }}
                      >
                        <FaNetworkWired style={{ fontSize: '0.65rem' }} /> {p.port}/{p.protocol}
                      </span>
                    ))}
                </div>

                <p style={{ fontSize: '0.9rem', color: 'var(--text-secondary)', marginBottom: '1.25rem', flexGrow: 1 }}>
                  {app.description}
                </p>

                {/* Action Buttons */}
                <div style={{ width: '100%', display: 'flex', gap: '0.5rem', flexDirection: 'column' }}>
                  {!isInstalled ? (
                    <button
                      onClick={() => openInstallModal(app)}
                      disabled={isBusy || !canOperate()}
                      className="btn primary"
                      title={!canOperate() ? 'Operator role required to install applications' : 'Configure & Install'}
                      style={{
                        width: '100%',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        gap: '0.5rem',
                        padding: '0.6rem 1rem',
                        opacity: !canOperate() ? 0.5 : 1,
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
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem', width: '100%' }}>
                      {status.access_url && (
                        <a
                          href={status.access_url}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="btn primary"
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            gap: '0.5rem',
                            padding: '0.55rem 0.75rem',
                            fontSize: '0.85rem',
                            textDecoration: 'none',
                          }}
                          title="Open application in a new tab"
                        >
                          <FaExternalLinkAlt style={{ fontSize: '0.75rem' }} />
                          <span>Open App</span>
                        </a>
                      )}
                      <div style={{ display: 'flex', gap: '0.5rem', width: '100%' }}>
                        <button
                          onClick={() => showCredentials(app.id, app.name)}
                          disabled={!isAdmin()}
                          className="btn secondary"
                          style={{
                            flex: 1,
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            gap: '0.4rem',
                            padding: '0.55rem 0.75rem',
                            fontSize: '0.85rem',
                            opacity: !isAdmin() ? 0.5 : 1,
                          }}
                          title={!isAdmin() ? 'Admin role required to view credentials' : 'View generated login credentials'}
                        >
                          <FaKey />
                          <span>Credentials</span>
                        </button>
                        <button
                          onClick={() => uninstallApp(app.id, app.name)}
                          disabled={isBusy || !isAdmin()}
                          className="btn danger"
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'center',
                            gap: '0.4rem',
                            padding: '0.55rem 0.75rem',
                            fontSize: '0.85rem',
                            opacity: !isAdmin() ? 0.5 : 1,
                          }}
                          title={!isAdmin() ? 'Admin role required to uninstall application' : 'Uninstall and remove container'}
                        >
                          {isBusy ? <FaSpinner className="animate-spin" /> : <FaTrash />}
                          <span>Remove</span>
                        </button>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Install & Network Consent Modal */}
      {installModalApp && (
        <div
          style={{
            position: 'fixed',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            background: 'rgba(0, 0, 0, 0.75)',
            backdropFilter: 'blur(6px)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            zIndex: 9999,
            padding: '1rem',
          }}
        >
          <div
            className="glass-panel"
            style={{
              background: '#0f172a',
              border: '1px solid var(--glass-border)',
              borderRadius: '16px',
              width: '100%',
              maxWidth: '560px',
              padding: '1.75rem',
              boxShadow: '0 25px 50px -12px rgba(0, 0, 0, 0.7)',
            }}
          >
            {/* Modal Header */}
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                marginBottom: '1.25rem',
                borderBottom: '1px solid rgba(255,255,255,0.1)',
                paddingBottom: '0.75rem',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                <img
                  src={installModalApp.icon}
                  alt={installModalApp.name}
                  style={{ width: '36px', height: '36px', objectFit: 'contain' }}
                />
                <div>
                  <h3 style={{ margin: 0, fontSize: '1.2rem', color: '#f8fafc' }}>
                    Deploy {installModalApp.name}
                  </h3>
                  <span style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                    Container Deployment &amp; Network Routing
                  </span>
                </div>
              </div>
              <button
                onClick={() => setInstallModalApp(null)}
                style={{
                  background: 'transparent',
                  border: 'none',
                  color: 'var(--text-secondary)',
                  cursor: 'pointer',
                  fontSize: '1.1rem',
                }}
              >
                <FaTimes />
              </button>
            </div>

            {/* Reverse Proxy Routing Configuration */}
            {installModalApp.ports.internal_web_port && (
              <div style={{ marginBottom: '1.5rem' }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '0.5rem' }}>
                  <FaShieldAlt style={{ color: '#34d399' }} />
                  <span style={{ fontSize: '0.95rem', fontWeight: 600, color: 'var(--text-primary)' }}>
                    Reverse Proxy Access Mode
                  </span>
                </div>
                <div
                  style={{
                    background: 'rgba(52, 211, 153, 0.08)',
                    border: '1px solid rgba(52, 211, 153, 0.25)',
                    borderRadius: '8px',
                    padding: '0.75rem',
                    fontSize: '0.8rem',
                    color: '#a7f3d0',
                    marginBottom: '0.75rem',
                  }}
                >
                  <strong>Zero External Port Exposure:</strong> This container will bind exclusively to{' '}
                  <code>127.0.0.1</code> on an isolated internal port and routes through WADM&apos;s TLS termination.
                </div>

                <div style={{ display: 'flex', flexDirection: 'column', gap: '0.5rem' }}>
                  <label
                    style={{
                      display: 'flex',
                      alignItems: 'flex-start',
                      gap: '0.75rem',
                      padding: '0.75rem',
                      background: accessMode === 'path' ? 'rgba(56, 189, 248, 0.1)' : 'rgba(255, 255, 255, 0.02)',
                      border: `1px solid ${accessMode === 'path' ? 'var(--accent-color)' : 'rgba(255, 255, 255, 0.08)'}`,
                      borderRadius: '8px',
                      cursor: 'pointer',
                    }}
                  >
                    <input
                      type="radio"
                      name="accessMode"
                      value="path"
                      checked={accessMode === 'path'}
                      onChange={() => setAccessMode('path')}
                      style={{ marginTop: '3px' }}
                    />
                    <div>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', fontWeight: 600, fontSize: '0.88rem', color: '#f8fafc' }}>
                        <FaFolder style={{ color: 'var(--accent-color)' }} />
                        <span>Subpath Access (<code>/apps/{installModalApp.id}</code>)</span>
                      </div>
                      <div style={{ fontSize: '0.78rem', color: 'var(--text-secondary)', marginTop: '2px' }}>
                        Accessible immediately through WADM on the current domain without any DNS configuration.
                      </div>
                    </div>
                  </label>

                  <label
                    style={{
                      display: 'flex',
                      alignItems: 'flex-start',
                      gap: '0.75rem',
                      padding: '0.75rem',
                      background: accessMode === 'subdomain' ? 'rgba(56, 189, 248, 0.1)' : 'rgba(255, 255, 255, 0.02)',
                      border: `1px solid ${accessMode === 'subdomain' ? 'var(--accent-color)' : 'rgba(255, 255, 255, 0.08)'}`,
                      borderRadius: '8px',
                      cursor: 'pointer',
                    }}
                  >
                    <input
                      type="radio"
                      name="accessMode"
                      value="subdomain"
                      checked={accessMode === 'subdomain'}
                      onChange={() => setAccessMode('subdomain')}
                      style={{ marginTop: '3px' }}
                    />
                    <div style={{ width: '100%' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', fontWeight: 600, fontSize: '0.88rem', color: '#f8fafc' }}>
                        <FaGlobe style={{ color: 'var(--accent-color)' }} />
                        <span>Dedicated Subdomain</span>
                      </div>
                      <div style={{ fontSize: '0.78rem', color: 'var(--text-secondary)', marginTop: '2px', marginBottom: accessMode === 'subdomain' ? '0.5rem' : 0 }}>
                        Requires pointing a DNS A/CNAME record to this server&apos;s public IP.
                      </div>
                      {accessMode === 'subdomain' && (
                        <input
                          type="text"
                          placeholder="e.g. cloud.yourdomain.com"
                          value={customDomain}
                          onChange={(e) => setCustomDomain(e.target.value)}
                          style={{
                            width: '100%',
                            background: '#020617',
                            border: '1px solid rgba(255, 255, 255, 0.15)',
                            borderRadius: '6px',
                            color: '#fff',
                            padding: '0.45rem 0.65rem',
                            fontSize: '0.85rem',
                            boxSizing: 'border-box',
                          }}
                        />
                      )}
                    </div>
                  </label>
                </div>
              </div>
            )}

            {/* Strict Port Authorization & Firewall Consent */}
            {installModalApp.ports.exposed_network_ports.length > 0 && (
              <div
                style={{
                  background: 'rgba(239, 68, 68, 0.08)',
                  border: '1px solid rgba(239, 68, 68, 0.3)',
                  borderRadius: '10px',
                  padding: '1rem',
                  marginBottom: '1.5rem',
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', color: '#f87171', marginBottom: '0.5rem' }}>
                  <FaExclamationTriangle style={{ fontSize: '1rem' }} />
                  <strong style={{ fontSize: '0.9rem' }}>External Port Authorization Required</strong>
                </div>
                <p style={{ fontSize: '0.8rem', color: '#fca5a5', margin: '0 0 0.75rem 0', lineHeight: 1.4 }}>
                  This application requires opening external network port(s) in your system firewall (UFW) to accept non-HTTP incoming traffic:
                </p>

                <div style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem', marginBottom: '0.85rem' }}>
                  {installModalApp.ports.exposed_network_ports.map((p) => (
                    <div
                      key={p.port}
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        background: 'rgba(0, 0, 0, 0.3)',
                        borderRadius: '6px',
                        padding: '0.4rem 0.65rem',
                        fontSize: '0.8rem',
                      }}
                    >
                      <span style={{ fontFamily: 'monospace', fontWeight: 600, color: '#f8fafc' }}>
                        Port {p.port}/{p.protocol}
                      </span>
                      <span style={{ color: 'var(--text-secondary)', fontSize: '0.75rem' }}>
                        {p.reason}
                      </span>
                    </div>
                  ))}
                </div>

                <label
                  style={{
                    display: 'flex',
                    alignItems: 'flex-start',
                    gap: '0.5rem',
                    cursor: 'pointer',
                    fontSize: '0.82rem',
                    color: '#f8fafc',
                    fontWeight: 500,
                  }}
                >
                  <input
                    type="checkbox"
                    checked={allowExposedPorts}
                    onChange={(e) => setAllowExposedPorts(e.target.checked)}
                    style={{ marginTop: '2px' }}
                  />
                  <span>
                    I explicitly authorize opening these port(s) in the host firewall (UFW) and creating audited security rules.
                  </span>
                </label>
              </div>
            )}

            {/* Modal Actions */}
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.75rem', borderTop: '1px solid rgba(255,255,255,0.08)', paddingTop: '1rem' }}>
              <button
                onClick={() => setInstallModalApp(null)}
                className="btn secondary"
                style={{ padding: '0.5rem 1rem' }}
              >
                Cancel
              </button>
              <button
                onClick={handleConfirmInstall}
                disabled={
                  (installModalApp.ports.exposed_network_ports.length > 0 && !allowExposedPorts) ||
                  (accessMode === 'subdomain' && !customDomain.trim())
                }
                className="btn primary"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: '0.5rem',
                  padding: '0.5rem 1.25rem',
                  opacity:
                    (installModalApp.ports.exposed_network_ports.length > 0 && !allowExposedPorts) ||
                    (accessMode === 'subdomain' && !customDomain.trim())
                      ? 0.5
                      : 1,
                }}
              >
                <FaPlay style={{ fontSize: '0.75rem' }} />
                <span>Confirm &amp; Deploy</span>
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Credentials Modal */}
      {activeCreds && (
        <div
          style={{
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
            padding: '1rem',
          }}
        >
          <div
            className="glass-panel"
            style={{
              background: '#0f172a',
              border: '1px solid var(--glass-border)',
              borderRadius: '12px',
              width: '100%',
              maxWidth: '520px',
              padding: '1.5rem',
              boxShadow: '0 20px 25px -5px rgba(0, 0, 0, 0.5)',
            }}
          >
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                marginBottom: '1rem',
                borderBottom: '1px solid rgba(255,255,255,0.1)',
                paddingBottom: '0.75rem',
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                <FaKey style={{ color: 'var(--accent-color)' }} />
                <h3 style={{ margin: 0, fontSize: '1.15rem', color: '#f8fafc' }}>
                  {activeCreds.appName} - Credentials &amp; Routing
                </h3>
              </div>
              <button
                onClick={() => setActiveCreds(null)}
                style={{
                  background: 'transparent',
                  border: 'none',
                  color: 'var(--text-secondary)',
                  cursor: 'pointer',
                  fontSize: '1.1rem',
                }}
              >
                <FaTimes />
              </button>
            </div>

            {activeCreds.accessUrl && (
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  background: 'rgba(56, 189, 248, 0.1)',
                  border: '1px solid rgba(56, 189, 248, 0.25)',
                  borderRadius: '8px',
                  padding: '0.65rem 0.85rem',
                  marginBottom: '1rem',
                  fontSize: '0.85rem',
                }}
              >
                <div>
                  <span style={{ color: 'var(--text-secondary)', fontSize: '0.75rem', display: 'block' }}>
                    Reverse Proxy URL
                  </span>
                  <a
                    href={activeCreds.accessUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    style={{ color: '#38bdf8', fontWeight: 600, textDecoration: 'none' }}
                  >
                    {activeCreds.accessUrl}
                  </a>
                </div>
                {activeCreds.internalPort && (
                  <span
                    style={{
                      fontSize: '0.75rem',
                      fontFamily: 'monospace',
                      color: 'var(--text-secondary)',
                    }}
                  >
                    127.0.0.1:{activeCreds.internalPort}
                  </span>
                )}
              </div>
            )}

            <p style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', marginBottom: '1rem' }}>
              Secure credentials generated by WADM during initial setup. Keep these safe:
            </p>

            <pre
              style={{
                background: '#020617',
                border: '1px solid rgba(255, 255, 255, 0.08)',
                borderRadius: '8px',
                padding: '1rem',
                color: '#38bdf8',
                fontFamily: 'monospace',
                fontSize: '0.9rem',
                overflowX: 'auto',
                whiteSpace: 'pre-wrap',
                marginBottom: '1.25rem',
              }}
            >
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
