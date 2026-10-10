import React, { useState, useEffect, useCallback } from 'react';
import {
  FaPuzzlePiece,
  FaPlay,
  FaStop,
  FaSync,
  FaCheckCircle,
  FaTimesCircle,
  FaExclamationTriangle,
  FaTerminal,
  FaExchangeAlt,
  FaCircle,
  FaStore
} from 'react-icons/fa';
import { useToast } from '../context/ToastContext';
import { useAuth } from '../context/AuthContext';
import { PluginStore } from './PluginStore';
import type { PluginRuntimeInfo } from '../types';

interface PluginsManagementProps {
  onSelectPlugin?: (pluginId: string) => void;
}

export const PluginsManagement: React.FC<PluginsManagementProps> = ({ onSelectPlugin }) => {
  const [plugins, setPlugins] = useState<PluginRuntimeInfo[]>([]);
  const [activeTab, setActiveTab] = useState<'installed' | 'store'>('installed');
  const [loading, setLoading] = useState(false);
  const [actionLoading, setActionLoading] = useState<Record<string, string>>({});
  const [latencies, setLatencies] = useState<Record<string, number>>({});
  const { addToast } = useToast();
  const { canOperate } = useAuth();

  const fetchPlugins = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch('/api/plugins');
      if (res.ok) {
        const data: PluginRuntimeInfo[] = await res.json();
        setPlugins(data);
      } else {
        addToast('Failed to load plugins', 'error');
      }
    } catch (err) {
      console.error(err);
      addToast('Error connecting to plugin engine', 'error');
    } finally {
      setLoading(false);
    }
  }, [addToast]);

  useEffect(() => {
    fetchPlugins();
    const interval = setInterval(fetchPlugins, 5000);
    return () => clearInterval(interval);
  }, [fetchPlugins]);

  const handleEnable = async (id: string) => {
    setActionLoading(prev => ({ ...prev, [id]: 'starting' }));
    try {
      const res = await fetch(`/api/plugins/${id}/enable`, { method: 'POST' });
      if (res.ok) {
        addToast(`Plugin '${id}' enabled successfully`, 'success');
        await fetchPlugins();
      } else {
        const err = await res.json();
        addToast(err.error || `Failed to start plugin '${id}'`, 'error');
        await fetchPlugins();
      }
    } catch {
      addToast(`Error communicating with plugin '${id}'`, 'error');
    } finally {
      setActionLoading(prev => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
    }
  };

  const handleDisable = async (id: string) => {
    setActionLoading(prev => ({ ...prev, [id]: 'stopping' }));
    try {
      const res = await fetch(`/api/plugins/${id}/disable`, { method: 'POST' });
      if (res.ok) {
        addToast(`Plugin '${id}' stopped`, 'info');
        await fetchPlugins();
      } else {
        const err = await res.json();
        addToast(err.error || `Failed to stop plugin '${id}'`, 'error');
      }
    } catch {
      addToast(`Error stopping plugin '${id}'`, 'error');
    } finally {
      setActionLoading(prev => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
    }
  };

  const handlePing = async (id: string) => {
    setActionLoading(prev => ({ ...prev, [id]: 'pinging' }));
    try {
      const res = await fetch(`/api/plugins/${id}/ping`, { method: 'POST' });
      if (res.ok) {
        const data = await res.json();
        const latency = data.latency_ms;
        setLatencies(prev => ({ ...prev, [id]: latency }));
        addToast(`Plugin '${id}' responded in ${latency} ms`, 'success');
      } else {
        const err = await res.json();
        addToast(err.error || `Ping failed for '${id}'`, 'error');
      }
    } catch {
      addToast(`Ping failed for '${id}'`, 'error');
    } finally {
      setActionLoading(prev => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
    }
  };

  const runningCount = plugins.filter(p => p.state === 'running').length;
  const stoppedCount = plugins.filter(p => p.state === 'stopped').length;
  const crashedCount = plugins.filter(p => p.state === 'crashed').length;

  return (
    <div style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
      {/* Top Header Card */}
      <div className="glass-card" style={{ padding: '1.5rem', display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '1rem' }}>
        <div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', marginBottom: '0.5rem' }}>
            <FaPuzzlePiece style={{ fontSize: '1.5rem', color: 'var(--accent-color)' }} />
            <h2 style={{ margin: 0, fontSize: '1.4rem', fontWeight: 700 }}>Dynamic Plugin Host</h2>
          </div>
          <p style={{ margin: 0, color: 'var(--text-secondary)', fontSize: '0.9rem' }}>
            Out-of-Process sidecar plugins communicating over Unix Domain Sockets with JSON-RPC 2.0 fault isolation.
          </p>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <button
            className="btn-secondary"
            onClick={fetchPlugins}
            disabled={loading}
            style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}
          >
            <FaSync className={loading ? 'spin' : ''} />
            <span>Refresh</span>
          </button>
        </div>
      </div>

      {/* Navigation Tab Bar */}
      <div style={{ display: 'flex', gap: '0.6rem', borderBottom: '1px solid rgba(255, 255, 255, 0.08)', paddingBottom: '0.5rem' }}>
        <button
          className={activeTab === 'installed' ? 'btn-primary' : 'btn-secondary'}
          onClick={() => setActiveTab('installed')}
          style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.88rem', padding: '0.5rem 1rem' }}
        >
          <FaPuzzlePiece />
          <span>Installed Sidecars ({plugins.length})</span>
        </button>
        <button
          className={activeTab === 'store' ? 'btn-primary' : 'btn-secondary'}
          onClick={() => setActiveTab('store')}
          style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.88rem', padding: '0.5rem 1rem' }}
        >
          <FaStore />
          <span>Plugin Store</span>
        </button>
      </div>

      {activeTab === 'store' ? (
        <PluginStore onPluginInstalled={fetchPlugins} />
      ) : (
        <>
          {/* Metric Summary Cards */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: '1rem' }}>
        <div className="glass-card" style={{ padding: '1.2rem' }}>
          <div style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginBottom: '0.5rem' }}>Total Installed</div>
          <div style={{ fontSize: '1.8rem', fontWeight: 800 }}>{plugins.length}</div>
        </div>
        <div className="glass-card" style={{ padding: '1.2rem', borderColor: 'rgba(52, 211, 153, 0.3)' }}>
          <div style={{ color: 'var(--success)', fontSize: '0.85rem', marginBottom: '0.5rem', display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
            <FaCheckCircle /> Running Sidecars
          </div>
          <div style={{ fontSize: '1.8rem', fontWeight: 800, color: 'var(--success)' }}>{runningCount}</div>
        </div>
        <div className="glass-card" style={{ padding: '1.2rem' }}>
          <div style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginBottom: '0.5rem', display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
            <FaCircle style={{ fontSize: '0.6rem' }} /> Stopped
          </div>
          <div style={{ fontSize: '1.8rem', fontWeight: 800, color: 'var(--text-secondary)' }}>{stoppedCount}</div>
        </div>
        {crashedCount > 0 && (
          <div className="glass-card" style={{ padding: '1.2rem', borderColor: 'rgba(248, 113, 113, 0.4)' }}>
            <div style={{ color: 'var(--danger)', fontSize: '0.85rem', marginBottom: '0.5rem', display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
              <FaTimesCircle /> Crashed
            </div>
            <div style={{ fontSize: '1.8rem', fontWeight: 800, color: 'var(--danger)' }}>{crashedCount}</div>
          </div>
        )}
      </div>

      {/* Plugins List */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
        <h3 style={{ margin: 0, fontSize: '1.1rem', fontWeight: 600, color: 'var(--text-secondary)' }}>Installed Plugins</h3>

        {plugins.length === 0 ? (
          <div className="glass-card" style={{ padding: '3rem', textAlign: 'center', color: 'var(--text-secondary)' }}>
            <FaPuzzlePiece style={{ fontSize: '3rem', opacity: 0.3, marginBottom: '1rem' }} />
            <p style={{ margin: 0, fontSize: '1rem' }}>No plugins found in data/plugins.</p>
            <p style={{ margin: '0.5rem 0 0 0', fontSize: '0.85rem', opacity: 0.7 }}>
              Install curated sidecar plugins from the Plugin Store or place bundles into data/plugins.
            </p>
            <button
              className="btn-primary"
              onClick={() => setActiveTab('store')}
              style={{ marginTop: '1.25rem', display: 'inline-flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.9rem' }}
            >
              <FaStore />
              <span>Browse Plugin Store</span>
            </button>
          </div>
        ) : (
          plugins.map(p => {
            const isRunning = p.state === 'running';
            const isCrashed = p.state === 'crashed';
            const op = actionLoading[p.manifest.id];

            return (
              <div
                key={p.manifest.id}
                className="glass-card"
                style={{
                  padding: '1.5rem',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: '1rem',
                  borderLeft: isRunning
                    ? '4px solid var(--success)'
                    : isCrashed
                    ? '4px solid var(--danger)'
                    : '4px solid rgba(255, 255, 255, 0.2)'
                }}
              >
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', flexWrap: 'wrap', gap: '1rem' }}>
                  <div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem', flexWrap: 'wrap' }}>
                      <h4 style={{ margin: 0, fontSize: '1.2rem', fontWeight: 700 }}>{p.manifest.name}</h4>
                      <span style={{ fontSize: '0.8rem', padding: '0.2rem 0.5rem', borderRadius: '4px', background: 'rgba(255,255,255,0.08)', color: 'var(--text-secondary)' }}>
                        v{p.manifest.version}
                      </span>
                      <span style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>by {p.manifest.author}</span>

                      {/* State Badge */}
                      <span
                        style={{
                          fontSize: '0.75rem',
                          fontWeight: 700,
                          padding: '0.25rem 0.6rem',
                          borderRadius: '12px',
                          display: 'inline-flex',
                          alignItems: 'center',
                          gap: '0.35rem',
                          textTransform: 'uppercase',
                          letterSpacing: '0.04em',
                          background: isRunning
                            ? 'rgba(52, 211, 153, 0.15)'
                            : isCrashed
                            ? 'rgba(248, 113, 113, 0.15)'
                            : 'rgba(148, 163, 184, 0.15)',
                          color: isRunning
                            ? 'var(--success)'
                            : isCrashed
                            ? 'var(--danger)'
                            : 'var(--text-secondary)'
                        }}
                      >
                        <span
                          style={{
                            width: '6px',
                            height: '6px',
                            borderRadius: '50%',
                            background: isRunning ? 'var(--success)' : isCrashed ? 'var(--danger)' : 'var(--text-secondary)'
                          }}
                        />
                        {p.state}
                      </span>
                    </div>

                    <p style={{ margin: '0.5rem 0 0 0', color: 'var(--text-secondary)', fontSize: '0.9rem' }}>
                      {p.manifest.description}
                    </p>
                  </div>

                  {/* Actions */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', flexWrap: 'wrap' }}>
                    {isRunning ? (
                      <>
                        <button
                          className="btn-secondary"
                          onClick={() => handlePing(p.manifest.id)}
                          disabled={!!op || !canOperate()}
                          title={!canOperate() ? "Operator role required to manage plugins" : ""}
                          style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', fontSize: '0.85rem', opacity: !canOperate() ? 0.5 : 1 }}
                        >
                          <FaExchangeAlt />
                          <span>{op === 'pinging' ? 'Pinging...' : 'Ping IPC'}</span>
                        </button>
                        {onSelectPlugin && (
                          <button
                            className="btn-primary"
                            onClick={() => onSelectPlugin(p.manifest.id)}
                            style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', fontSize: '0.85rem' }}
                          >
                            <FaTerminal />
                            <span>Open Console</span>
                          </button>
                        )}
                        <button
                          className="btn-secondary danger"
                          onClick={() => handleDisable(p.manifest.id)}
                          disabled={!!op || !canOperate()}
                          title={!canOperate() ? "Operator role required to manage plugins" : ""}
                          style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', fontSize: '0.85rem', opacity: !canOperate() ? 0.5 : 1 }}
                        >
                          <FaStop />
                          <span>{op === 'stopping' ? 'Stopping...' : 'Stop'}</span>
                        </button>
                      </>
                    ) : (
                      <button
                        className="btn-primary"
                        onClick={() => handleEnable(p.manifest.id)}
                        disabled={!!op || !canOperate()}
                        title={!canOperate() ? "Operator role required to manage plugins" : ""}
                        style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', fontSize: '0.85rem', opacity: !canOperate() ? 0.5 : 1 }}
                      >
                        <FaPlay />
                        <span>{op === 'starting' ? 'Starting...' : 'Start'}</span>
                      </button>
                    )}
                  </div>
                </div>

                {/* Runtime Details Bar */}
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: '1.5rem',
                    flexWrap: 'wrap',
                    padding: '0.75rem 1rem',
                    borderRadius: '8px',
                    background: 'rgba(0, 0, 0, 0.2)',
                    fontSize: '0.8rem',
                    color: 'var(--text-secondary)'
                  }}
                >
                  <div>
                    <span style={{ opacity: 0.6 }}>ID:</span> <code style={{ color: 'var(--text-primary)' }}>{p.manifest.id}</code>
                  </div>
                  {p.pid && (
                    <div>
                      <span style={{ opacity: 0.6 }}>PID:</span> <strong style={{ color: 'var(--text-primary)' }}>{p.pid}</strong>
                    </div>
                  )}
                  {p.socket_path && (
                    <div style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', maxWidth: '350px' }}>
                      <span style={{ opacity: 0.6 }}>Socket:</span> <code style={{ color: 'var(--text-primary)' }}>{p.socket_path}</code>
                    </div>
                  )}
                  {latencies[p.manifest.id] !== undefined && (
                    <div>
                      <span style={{ opacity: 0.6 }}>Latency:</span> <strong style={{ color: 'var(--accent-color)' }}>{latencies[p.manifest.id]} ms</strong>
                    </div>
                  )}
                  {p.manifest.capabilities?.length > 0 && (
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.3rem' }}>
                      <span style={{ opacity: 0.6 }}>Caps:</span>
                      {p.manifest.capabilities.map(cap => (
                        <span key={cap} style={{ padding: '0.1rem 0.4rem', borderRadius: '4px', background: 'rgba(56, 189, 248, 0.1)', color: 'var(--accent-color)', fontSize: '0.75rem' }}>
                          {cap}
                        </span>
                      ))}
                    </div>
                  )}
                </div>

                {/* Error Banner */}
                {p.error && (
                  <div
                    style={{
                      padding: '0.75rem 1rem',
                      borderRadius: '8px',
                      background: 'rgba(248, 113, 113, 0.1)',
                      border: '1px solid rgba(248, 113, 113, 0.3)',
                      color: 'var(--danger)',
                      fontSize: '0.85rem',
                      display: 'flex',
                      alignItems: 'center',
                      gap: '0.5rem'
                    }}
                  >
                    <FaExclamationTriangle />
                    <span>{p.error}</span>
                  </div>
                )}
              </div>
            );
          })
        )}
      </div>
        </>
      )}
    </div>
  );
};
