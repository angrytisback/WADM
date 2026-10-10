import React, { useState, useEffect, useCallback } from 'react';
import {
  FaArrowLeft,
  FaPlay,
  FaStop,
  FaExchangeAlt,
  FaPaperPlane,
  FaTerminal,
  FaCheckCircle,
  FaExclamationTriangle,
  FaInfoCircle
} from 'react-icons/fa';
import { useToast } from '../context/ToastContext';
import type { PluginRuntimeInfo, PluginRpcResponse } from '../types';

interface PluginContainerProps {
  pluginId: string;
  onBack?: () => void;
}

interface RpcHistoryItem {
  id: number;
  method: string;
  params: unknown;
  response: unknown;
  latencyMs: number;
  timestamp: string;
}

export const PluginContainer: React.FC<PluginContainerProps> = ({ pluginId, onBack }) => {
  const [pluginInfo, setPluginInfo] = useState<PluginRuntimeInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [latency, setLatency] = useState<number | null>(null);

  // RPC Console state
  const [rpcMethod, setRpcMethod] = useState('service.status');
  const [rpcParams, setRpcParams] = useState('{}');
  const [rpcSending, setRpcSending] = useState(false);
  const [rpcResponse, setRpcResponse] = useState<PluginRpcResponse | null>(null);
  const [history, setHistory] = useState<RpcHistoryItem[]>([]);

  const { addToast } = useToast();

  const fetchPluginInfo = useCallback(async () => {
    try {
      const res = await fetch(`/api/plugins/${pluginId}`);
      if (res.ok) {
        const data: PluginRuntimeInfo = await res.json();
        setPluginInfo(data);
      } else {
        addToast(`Failed to load plugin '${pluginId}'`, 'error');
      }
    } catch {
      addToast(`Error communicating with backend`, 'error');
    } finally {
      setLoading(false);
    }
  }, [pluginId, addToast]);

  useEffect(() => {
    fetchPluginInfo();
  }, [fetchPluginInfo]);

  const handleStart = async () => {
    try {
      const res = await fetch(`/api/plugins/${pluginId}/enable`, { method: 'POST' });
      if (res.ok) {
        addToast(`Plugin '${pluginId}' started`, 'success');
        await fetchPluginInfo();
      } else {
        const err = await res.json();
        addToast(err.error || `Failed to start plugin`, 'error');
      }
    } catch {
      addToast('Error starting plugin', 'error');
    }
  };

  const handleStop = async () => {
    try {
      const res = await fetch(`/api/plugins/${pluginId}/disable`, { method: 'POST' });
      if (res.ok) {
        addToast(`Plugin '${pluginId}' stopped`, 'info');
        await fetchPluginInfo();
      } else {
        const err = await res.json();
        addToast(err.error || `Failed to stop plugin`, 'error');
      }
    } catch {
      addToast('Error stopping plugin', 'error');
    }
  };

  const handlePing = async () => {
    try {
      const res = await fetch(`/api/plugins/${pluginId}/ping`, { method: 'POST' });
      if (res.ok) {
        const data = await res.json();
        setLatency(data.latency_ms);
        addToast(`IPC Ping: ${data.latency_ms} ms`, 'success');
      } else {
        const err = await res.json();
        addToast(err.error || 'Ping failed', 'error');
      }
    } catch {
      addToast('Ping failed', 'error');
    }
  };

  const handleSendRpc = async (methodToUse?: string, paramsToUse?: string) => {
    const targetMethod = methodToUse || rpcMethod;
    const targetParamsStr = paramsToUse || rpcParams;

    let parsedParams: unknown = {};
    if (targetParamsStr.trim()) {
      try {
        parsedParams = JSON.parse(targetParamsStr);
      } catch {
        addToast('Invalid JSON in RPC parameters', 'error');
        return;
      }
    }

    setRpcSending(true);
    const startTime = performance.now();
    try {
      const payload = {
        jsonrpc: '2.0',
        id: Date.now(),
        method: targetMethod,
        params: parsedParams
      };

      const res = await fetch(`/api/plugins/${pluginId}/rpc`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });

      const latencyMs = Math.round((performance.now() - startTime) * 100) / 100;
      const data: PluginRpcResponse = await res.json();
      setRpcResponse(data);

      setHistory(prev => [
        {
          id: Date.now(),
          method: targetMethod,
          params: parsedParams,
          response: data,
          latencyMs,
          timestamp: new Date().toLocaleTimeString()
        },
        ...prev.slice(0, 9)
      ]);

      if (data.error) {
        addToast(`RPC Error: ${data.error.message}`, 'warning');
      } else {
        addToast(`RPC OK (${latencyMs} ms)`, 'success');
      }
    } catch {
      addToast('RPC request failed', 'error');
    } finally {
      setRpcSending(false);
    }
  };

  if (loading) {
    return (
      <div style={{ padding: '2rem', textAlign: 'center', color: 'var(--text-secondary)' }}>
        Loading plugin details...
      </div>
    );
  }

  if (!pluginInfo) {
    return (
      <div style={{ padding: '2rem' }}>
        <button className="btn-secondary" onClick={onBack} style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '1rem' }}>
          <FaArrowLeft /> Back
        </button>
        <div className="glass-card" style={{ padding: '2rem', textAlign: 'center' }}>
          <h3>Plugin not found</h3>
        </div>
      </div>
    );
  }

  const isRunning = pluginInfo.state === 'running';

  return (
    <div style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
      {/* Top Bar */}
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '1rem' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          {onBack && (
            <button className="btn-secondary" onClick={onBack} style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
              <FaArrowLeft />
              <span>Plugins</span>
            </button>
          )}
          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
              <h2 style={{ margin: 0, fontSize: '1.4rem', fontWeight: 700 }}>{pluginInfo.manifest.name}</h2>
              <span style={{ fontSize: '0.8rem', padding: '0.2rem 0.5rem', borderRadius: '4px', background: 'rgba(255,255,255,0.08)', color: 'var(--text-secondary)' }}>
                v{pluginInfo.manifest.version}
              </span>
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
                  background: isRunning ? 'rgba(52, 211, 153, 0.15)' : 'rgba(148, 163, 184, 0.15)',
                  color: isRunning ? 'var(--success)' : 'var(--text-secondary)'
                }}
              >
                <span
                  style={{
                    width: '6px',
                    height: '6px',
                    borderRadius: '50%',
                    background: isRunning ? 'var(--success)' : 'var(--text-secondary)'
                  }}
                />
                {pluginInfo.state}
              </span>
            </div>
            <p style={{ margin: '0.25rem 0 0 0', color: 'var(--text-secondary)', fontSize: '0.85rem' }}>
              {pluginInfo.manifest.description}
            </p>
          </div>
        </div>

        {/* Action Controls */}
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
          {isRunning ? (
            <>
              <button className="btn-secondary" onClick={handlePing} style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                <FaExchangeAlt />
                <span>Ping IPC</span>
              </button>
              <button className="btn-secondary danger" onClick={handleStop} style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                <FaStop />
                <span>Stop</span>
              </button>
            </>
          ) : (
            <button className="btn-primary" onClick={handleStart} style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
              <FaPlay />
              <span>Start Plugin</span>
            </button>
          )}
        </div>
      </div>

      {/* IPC & Environment Status Info */}
      <div
        className="glass-card"
        style={{
          padding: '1rem 1.25rem',
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
          gap: '1rem',
          fontSize: '0.85rem'
        }}
      >
        <div>
          <div style={{ color: 'var(--text-secondary)', fontSize: '0.75rem', marginBottom: '0.25rem' }}>ID & Executable</div>
          <code style={{ color: 'var(--accent-color)' }}>{pluginInfo.manifest.id}</code>
          <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', marginTop: '0.2rem' }}>
            {pluginInfo.manifest.executable}
          </div>
        </div>

        <div>
          <div style={{ color: 'var(--text-secondary)', fontSize: '0.75rem', marginBottom: '0.25rem' }}>Process PID</div>
          <strong>{pluginInfo.pid ? `${pluginInfo.pid}` : 'None'}</strong>
        </div>

        <div>
          <div style={{ color: 'var(--text-secondary)', fontSize: '0.75rem', marginBottom: '0.25rem' }}>UDS Socket Path</div>
          <code style={{ fontSize: '0.75rem', wordBreak: 'break-all' }}>{pluginInfo.socket_path || 'Not Bound'}</code>
        </div>

        <div>
          <div style={{ color: 'var(--text-secondary)', fontSize: '0.75rem', marginBottom: '0.25rem' }}>IPC Latency</div>
          <strong style={{ color: 'var(--accent-color)' }}>
            {latency !== null ? `${latency} ms` : 'Not Measured'}
          </strong>
        </div>
      </div>

      {!isRunning ? (
        <div className="glass-card" style={{ padding: '3rem', textAlign: 'center' }}>
          <FaInfoCircle style={{ fontSize: '2.5rem', opacity: 0.4, marginBottom: '1rem' }} />
          <h3>Plugin is currently stopped</h3>
          <p style={{ color: 'var(--text-secondary)', maxWidth: '400px', margin: '0.5rem auto 1.5rem auto' }}>
            Start the out-of-process sidecar to open its Unix Domain Socket and initiate JSON-RPC communication.
          </p>
          <button className="btn-primary" onClick={handleStart} style={{ display: 'inline-flex', alignItems: 'center', gap: '0.5rem' }}>
            <FaPlay /> Start Plugin
          </button>
        </div>
      ) : (
        /* Running Content & Interactive JSON-RPC Console */
        <div style={{ display: 'grid', gridTemplateColumns: '1fr', gap: '1.5rem' }}>
          {/* RPC Console Card */}
          <div className="glass-card" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '0.5rem' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                <FaTerminal style={{ color: 'var(--accent-color)' }} />
                <h3 style={{ margin: 0, fontSize: '1.1rem', fontWeight: 600 }}>JSON-RPC 2.0 Console</h3>
              </div>
              {/* Presets */}
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', flexWrap: 'wrap' }}>
                <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>Presets:</span>
                <button
                  className="btn-secondary"
                  style={{ fontSize: '0.75rem', padding: '0.2rem 0.5rem' }}
                  onClick={() => {
                    setRpcMethod('ping');
                    setRpcParams('{}');
                    handleSendRpc('ping', '{}');
                  }}
                >
                  ping
                </button>
                <button
                  className="btn-secondary"
                  style={{ fontSize: '0.75rem', padding: '0.2rem 0.5rem' }}
                  onClick={() => {
                    setRpcMethod('service.status');
                    setRpcParams('{}');
                    handleSendRpc('service.status', '{}');
                  }}
                >
                  service.status
                </button>
                <button
                  className="btn-secondary"
                  style={{ fontSize: '0.75rem', padding: '0.2rem 0.5rem' }}
                  onClick={() => {
                    setRpcMethod('service.action');
                    setRpcParams(JSON.stringify({ action: 'restart_worker' }, null, 2));
                  }}
                >
                  service.action
                </button>
              </div>
            </div>

            {/* Input Row */}
            <div style={{ display: 'grid', gridTemplateColumns: '200px 1fr auto', gap: '0.75rem', alignItems: 'flex-start' }}>
              <div>
                <label style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', display: 'block', marginBottom: '0.25rem' }}>Method</label>
                <input
                  type="text"
                  value={rpcMethod}
                  onChange={e => setRpcMethod(e.target.value)}
                  placeholder="e.g. ping"
                  style={{
                    width: '100%',
                    padding: '0.5rem 0.75rem',
                    borderRadius: '6px',
                    border: '1px solid var(--glass-border)',
                    background: 'rgba(0,0,0,0.3)',
                    color: 'var(--text-primary)',
                    fontFamily: 'monospace',
                    fontSize: '0.85rem'
                  }}
                />
              </div>

              <div>
                <label style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', display: 'block', marginBottom: '0.25rem' }}>Params (JSON)</label>
                <input
                  type="text"
                  value={rpcParams}
                  onChange={e => setRpcParams(e.target.value)}
                  placeholder="{}"
                  style={{
                    width: '100%',
                    padding: '0.5rem 0.75rem',
                    borderRadius: '6px',
                    border: '1px solid var(--glass-border)',
                    background: 'rgba(0,0,0,0.3)',
                    color: 'var(--text-primary)',
                    fontFamily: 'monospace',
                    fontSize: '0.85rem'
                  }}
                />
              </div>

              <div style={{ paddingTop: '1.2rem' }}>
                <button
                  className="btn-primary"
                  onClick={() => handleSendRpc()}
                  disabled={rpcSending}
                  style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.55rem 1rem' }}
                >
                  <FaPaperPlane />
                  <span>{rpcSending ? 'Calling...' : 'Invoke'}</span>
                </button>
              </div>
            </div>

            {/* Response Output */}
            {rpcResponse && (
              <div style={{ marginTop: '0.5rem' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '0.4rem' }}>
                  <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', fontWeight: 600 }}>RPC Response</span>
                  {rpcResponse.error ? (
                    <span style={{ fontSize: '0.75rem', color: 'var(--danger)', display: 'flex', alignItems: 'center', gap: '0.3rem' }}>
                      <FaExclamationTriangle /> Error ({rpcResponse.error.code})
                    </span>
                  ) : (
                    <span style={{ fontSize: '0.75rem', color: 'var(--success)', display: 'flex', alignItems: 'center', gap: '0.3rem' }}>
                      <FaCheckCircle /> Success
                    </span>
                  )}
                </div>
                <pre
                  style={{
                    background: '#090d16',
                    border: '1px solid var(--glass-border)',
                    borderRadius: '8px',
                    padding: '1rem',
                    fontSize: '0.85rem',
                    fontFamily: 'monospace',
                    overflowX: 'auto',
                    margin: 0,
                    color: rpcResponse.error ? '#fca5a5' : '#86efac',
                    maxHeight: '260px'
                  }}
                >
                  {JSON.stringify(rpcResponse, null, 2)}
                </pre>
              </div>
            )}

            {/* History Table */}
            {history.length > 0 && (
              <div style={{ marginTop: '1rem' }}>
                <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', fontWeight: 600, marginBottom: '0.5rem' }}>
                  Recent Calls
                </div>
                <div style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem' }}>
                  {history.map(item => (
                    <div
                      key={item.id}
                      style={{
                        padding: '0.4rem 0.75rem',
                        background: 'rgba(0,0,0,0.2)',
                        borderRadius: '6px',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        fontSize: '0.8rem',
                        cursor: 'pointer'
                      }}
                      onClick={() => {
                        setRpcMethod(item.method);
                        setRpcParams(JSON.stringify(item.params));
                        setRpcResponse(item.response as PluginRpcResponse);
                      }}
                    >
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                        <code style={{ color: 'var(--accent-color)' }}>{item.method}</code>
                        <span style={{ color: 'var(--text-secondary)', fontSize: '0.75rem' }}>{item.timestamp}</span>
                      </div>
                      <span style={{ color: 'var(--text-secondary)', fontSize: '0.75rem' }}>{item.latencyMs} ms</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
