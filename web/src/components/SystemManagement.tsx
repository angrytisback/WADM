import React, { useState, useEffect, useRef } from 'react';
import { 
  FaPowerOff, FaSync, FaClock, FaTrash, FaMemory, 
  FaBolt, FaSearch, FaEraser, FaTerminal, FaDownload, 
  FaHistory, FaBroom, FaExchangeAlt
} from 'react-icons/fa';
import { useToast } from '../context/ToastContext';
import { useSystem } from '../context/SystemContext';
import { useModal } from '../context/ModalContext';
import { useServerStatus } from '../context/ServerStatusContext';

interface LogEntry {
  id: string;
  timestamp: string;
  action: string;
  status: 'success' | 'error' | 'info';
  output: string;
}

interface PowerStatus {
  scheduled: boolean;
  mode?: string;
  scheduled_time_usec?: number;
}

const SystemManagement: React.FC = () => {
  const { confirm } = useModal();
  const { addToast } = useToast();
  const { systemInfo } = useSystem();
  const { triggerReboot, triggerShutdown } = useServerStatus();
  const canManage = systemInfo?.is_root || systemInfo?.has_sudo;
  const privilegeHint = !canManage ? "Root or Sudo privileges required for this action" : "";
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [scheduleHours, setScheduleHours] = useState<number>(0);
  const [scheduleMinutes, setScheduleMinutes] = useState<number>(30);
  const [scheduleMode, setScheduleMode] = useState<'shutdown' | 'reboot'>('shutdown');
  const [powerStatus, setPowerStatus] = useState<PowerStatus | null>(null);
  const [isPowerActionLoading, setIsPowerActionLoading] = useState(false);
  const [dnsStats, setDnsStats] = useState<string>('');
  const logEndRef = useRef<HTMLDivElement>(null);

  // Load logs from localStorage on mount
  useEffect(() => {
    const savedLogs = localStorage.getItem('wadm_system_logs');
    if (savedLogs) {
      try {
        setLogs(JSON.parse(savedLogs));
      } catch (e) {
        console.error("Failed to parse saved logs", e);
      }
    }
    fetchDnsStats();
  }, []);

  // Save logs to localStorage whenever they change
  useEffect(() => {
    localStorage.setItem('wadm_system_logs', JSON.stringify(logs));
    scrollToBottom();
  }, [logs]);

  const scrollToBottom = () => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  const addLog = (action: string, output: string, status: 'success' | 'error' | 'info' = 'info') => {
    const newLog: LogEntry = {
      id: Date.now().toString(),
      timestamp: new Date().toLocaleString(),
      action,
      status,
      output: typeof output === 'string' ? output : JSON.stringify(output)
    };
    setLogs(prev => [...prev, newLog]);
  };

  const clearLogs = async () => {
    const ok = await confirm({
      title: 'Clear History',
      message: 'Are you sure you want to clear the log history?',
      type: 'danger'
    });
    if (ok) {
      setLogs([]);
      localStorage.removeItem('wadm_system_logs');
    }
  };

  const downloadLogs = () => {
    const logContent = logs.map(l => `[${l.timestamp}] ${l.action.toUpperCase()} (${l.status.toUpperCase()}):\n${l.output}\n${'-'.repeat(40)}`).join('\n');
    const blob = new Blob([logContent], { type: 'text/plain' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `wadm_system_logs_${new Date().toISOString().slice(0, 10)}.txt`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const fetchDnsStats = async () => {
    try {
      const res = await fetch('/api/system/dns');
      if (res.ok) {
        const data = await res.json();
        setDnsStats(data.stats);
      }
    } catch (e) {
      console.error(e);
    }
  };

  const fetchPowerStatus = async () => {
    try {
      const res = await fetch('/api/system/power/status');
      if (res.ok) {
        const data = await res.json();
        setPowerStatus(data);
      }
    } catch (e) {
      console.error(e);
    }
  };

  useEffect(() => {
    fetchPowerStatus();
    const interval = setInterval(fetchPowerStatus, 5000);
    return () => clearInterval(interval);
  }, []);

  const handlePowerAction = async (action: string) => {
    if (action === 'reboot') {
      const ok = await confirm({
        title: 'Confirm System Reboot',
        message: 'Are you sure you want to reboot the server? All current connections will be terminated while the system restarts.',
        type: 'danger',
        confirmText: 'Reboot Now'
      });
      if (!ok) return;

      setIsPowerActionLoading(true);
      addLog('Power Action: reboot', 'Dispatching system reboot...', 'info');
      try {
        await fetch('/api/system/power', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ action: 'reboot' })
        });
        addToast('Reboot initiated. Reconnecting...', 'success');
        triggerReboot();
      } catch {
        // Even if fetch throws because server died immediately, trigger reboot screen
        triggerReboot();
      } finally {
        setIsPowerActionLoading(false);
      }
      return;
    }

    if (action === 'shutdown') {
      const ok = await confirm({
        title: 'Confirm System Shutdown',
        message: 'Are you sure you want to power down the server now? You will need physical access to turn it back on.',
        type: 'danger',
        confirmText: 'Shutdown Now'
      });
      if (!ok) return;

      setIsPowerActionLoading(true);
      addLog('Power Action: shutdown', 'Dispatching system poweroff...', 'info');
      try {
        await fetch('/api/system/power', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ action: 'shutdown' })
        });
        addToast('Server is shutting down', 'warning');
        triggerShutdown();
      } catch {
        triggerShutdown();
      } finally {
        setIsPowerActionLoading(false);
      }
      return;
    }

    if (action === 'schedule') {
      const totalMins = scheduleHours * 60 + scheduleMinutes;
      if (totalMins <= 0) {
        addToast('Please enter a duration greater than 0 minutes', 'warning');
        return;
      }

      const modeText = scheduleMode === 'reboot' ? 'Reboot' : 'Shutdown';
      const durationText = `${scheduleHours > 0 ? `${scheduleHours}h ` : ''}${scheduleMinutes}m`;
      const ok = await confirm({
        title: `Schedule System ${modeText}`,
        message: `Are you sure you want to schedule a system ${modeText.toLowerCase()} in ${durationText}?`,
        type: 'warning',
        confirmText: `Schedule ${modeText}`
      });
      if (!ok) return;

      setIsPowerActionLoading(true);
      const actionName = scheduleMode === 'reboot' ? 'schedule_reboot' : 'schedule_shutdown';
      try {
        const res = await fetch('/api/system/power', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ action: actionName, hours: scheduleHours, minutes: scheduleMinutes })
        });
        const data = await res.json();
        if (res.ok) {
          addToast(typeof data === 'string' ? data : `${modeText} scheduled successfully`, 'success');
          addLog(`Power Action: ${actionName}`, typeof data === 'string' ? data : `${modeText} scheduled in ${durationText}`, 'success');
          fetchPowerStatus();
        } else {
          addToast(data, 'error');
        }
      } catch (e: any) {
        addToast('Failed to schedule power action', 'error');
        addLog('Power Action: schedule', e.toString(), 'error');
      } finally {
        setIsPowerActionLoading(false);
      }
      return;
    }

    if (action === 'cancel') {
      const ok = await confirm({
        title: 'Cancel Scheduled Action',
        message: 'Are you sure you want to cancel the pending shutdown/reboot schedule?',
        type: 'info',
        confirmText: 'Cancel Schedule'
      });
      if (!ok) return;

      setIsPowerActionLoading(true);
      try {
        const res = await fetch('/api/system/power', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ action: 'cancel' })
        });
        const data = await res.json();
        if (res.ok) {
          addToast(typeof data === 'string' ? data : 'Scheduled sequence cancelled', 'info');
          addLog('Power Action: cancel', typeof data === 'string' ? data : 'Cancelled', 'info');
          fetchPowerStatus();
        } else {
          addToast(data, 'error');
        }
      } catch (e: any) {
        addToast('Failed to cancel power action', 'error');
        addLog('Power Action: cancel', e.toString(), 'error');
      } finally {
        setIsPowerActionLoading(false);
      }
      return;
    }
  };

  const handleMaintenance = async (action: string) => {
    setIsLoading(true);
    addLog(`Maintenance: ${action}`, `Starting ${action} protocol...`, 'info');
    
    try {
      const res = await fetch('/api/system/maintenance', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ action })
      });
      const data = await res.json();
      if (res.ok && data.success !== false) {
        const msg = data.message || (typeof data === 'string' ? data : "Maintenance protocol completed successfully.");
        const details = data.details || msg;
        addToast(msg, 'success');
        addLog(`Maintenance: ${action}`, details, 'success');
      } else {
        const errMsg = data.message || (typeof data === 'string' ? data : "Maintenance protocol failed.");
        const errDetails = data.details || errMsg;
        addToast(errMsg, 'error');
        addLog(`Maintenance: ${action}`, errDetails, 'error');
      }
    } catch (e: any) {
      addToast('Operation failed', 'error');
      addLog(`Maintenance: ${action}`, e.toString(), 'error');
    } finally {
      setIsLoading(false);
    }
  };

  const handleFlushDns = async () => {
    addLog(`DNS: flush`, `Flushing DNS caches...`, 'info');
    try {
      const res = await fetch('/api/system/dns/flush', { method: 'POST' });
      const data = await res.json();
      if (res.ok) {
        addToast(data, 'success');
        addLog(`DNS: flush`, data, 'success');
        fetchDnsStats();
      } else {
        addToast(data, 'error');
        addLog(`DNS: flush`, data, 'error');
      }
    } catch (e: any) {
      addToast('DNS flush failed', 'error');
      addLog(`DNS: flush`, e.toString(), 'error');
    }
  };

  return (
    <div className="fade-in" style={{ 
      display: 'grid', 
      gridTemplateColumns: '1fr 350px', 
      gap: '2rem', 
      height: 'calc(100vh - 120px)',
      overflow: 'hidden'
    }}>
      
      {/* LEFT COLUMN: Operations */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: '2rem', overflowY: 'auto', paddingRight: '1rem' }} className="no-scrollbar">
        
        {/* Power Management */}
        <section>
          <div className="glass-panel" style={{ padding: '2rem' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '2rem' }}>
              <div style={{ 
                width: '48px', height: '48px', borderRadius: '12px', 
                background: 'rgba(248, 113, 113, 0.1)', color: 'var(--danger)',
                display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: '1.5rem'
              }}>
                <FaPowerOff />
              </div>
              <div>
                <h3 style={{ fontSize: '1.4rem' }}>Power Management</h3>
                <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>Control system power state and scheduling</p>
              </div>
            </div>

            {/* Scheduled Power Action Active Banner */}
            {powerStatus?.scheduled && (
              <div style={{
                background: 'rgba(245, 158, 11, 0.15)',
                border: '1px solid rgba(245, 158, 11, 0.4)',
                borderRadius: '12px',
                padding: '1rem 1.5rem',
                marginBottom: '1.5rem',
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center',
                color: '#fef3c7'
              }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                  <FaClock style={{ color: 'var(--warning)', fontSize: '1.3rem' }} />
                  <div>
                    <strong style={{ fontSize: '1rem' }}>
                      Active Scheduled {powerStatus.mode?.toUpperCase() === 'REBOOT' ? 'Reboot' : 'Shutdown'}
                    </strong>
                    <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
                      A system power sequence has been scheduled by the operating system.
                    </div>
                  </div>
                </div>
                <button
                  onClick={() => handlePowerAction('cancel')}
                  disabled={isPowerActionLoading || !canManage}
                  className="btn-sm"
                  style={{
                    background: 'rgba(239, 68, 68, 0.2)',
                    color: '#fca5a5',
                    border: '1px solid rgba(239, 68, 68, 0.4)',
                    padding: '0.4rem 0.8rem',
                    borderRadius: '6px',
                    fontWeight: 600,
                    cursor: 'pointer'
                  }}
                >
                  Cancel Schedule
                </button>
              </div>
            )}

            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))', gap: '1.5rem' }}>
              {/* Instant Controls */}
              <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                <h4 style={{ margin: 0, fontSize: '0.95rem', color: 'var(--text-secondary)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                  Instant Actions
                </h4>
                <button 
                  className="btn-primary" 
                  style={{ 
                    display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '1rem',
                    background: 'linear-gradient(135deg, rgba(251, 191, 36, 0.2) 0%, rgba(251, 191, 36, 0.1) 100%)',
                    borderColor: 'rgba(251, 191, 36, 0.3)', color: 'var(--warning)',
                    padding: '1.25rem',
                    cursor: (isPowerActionLoading || !canManage) ? 'not-allowed' : 'pointer'
                  }}
                  onClick={() => handlePowerAction('reboot')}
                  disabled={isPowerActionLoading || !canManage}
                  title={privilegeHint}
                >
                  <FaSync className={isPowerActionLoading ? 'animate-spin' : ''} style={{ fontSize: '1.2rem' }} /> 
                  <span style={{ fontWeight: 800, letterSpacing: '0.05em' }}>REBOOT SYSTEM</span>
                </button>
                <button 
                  className="btn-primary" 
                  style={{ 
                    display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '1rem',
                    background: 'linear-gradient(135deg, rgba(248, 113, 113, 0.2) 0%, rgba(248, 113, 113, 0.1) 100%)',
                    borderColor: 'rgba(248, 113, 113, 0.3)', color: 'var(--danger)',
                    padding: '1.25rem',
                    cursor: (isPowerActionLoading || !canManage) ? 'not-allowed' : 'pointer'
                  }}
                  onClick={() => handlePowerAction('shutdown')}
                  disabled={isPowerActionLoading || !canManage}
                  title={privilegeHint}
                >
                  <FaPowerOff style={{ fontSize: '1.2rem' }} /> 
                  <span style={{ fontWeight: 800, letterSpacing: '0.05em' }}>SHUTDOWN NOW</span>
                </button>
              </div>

              {/* Scheduled Power Timer */}
              <div className="glass-panel" style={{ padding: '1.5rem', background: 'rgba(0,0,0,0.25)', display: 'flex', flexDirection: 'column', gap: '1rem' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <h4 style={{ margin: 0, fontSize: '0.95rem', display: 'flex', alignItems: 'center', gap: '0.5rem', color: 'var(--text-primary)' }}>
                    <FaClock style={{ color: 'var(--accent-color)' }} /> Scheduled Timer
                  </h4>
                  {/* Mode Selector */}
                  <div style={{ display: 'flex', background: 'rgba(255,255,255,0.05)', borderRadius: '6px', padding: '2px' }}>
                    <button
                      type="button"
                      onClick={() => setScheduleMode('shutdown')}
                      style={{
                        padding: '0.25rem 0.6rem',
                        fontSize: '0.75rem',
                        fontWeight: 600,
                        border: 'none',
                        borderRadius: '4px',
                        cursor: 'pointer',
                        background: scheduleMode === 'shutdown' ? 'var(--danger)' : 'transparent',
                        color: scheduleMode === 'shutdown' ? '#fff' : 'var(--text-secondary)'
                      }}
                    >
                      Shutdown
                    </button>
                    <button
                      type="button"
                      onClick={() => setScheduleMode('reboot')}
                      style={{
                        padding: '0.25rem 0.6rem',
                        fontSize: '0.75rem',
                        fontWeight: 600,
                        border: 'none',
                        borderRadius: '4px',
                        cursor: 'pointer',
                        background: scheduleMode === 'reboot' ? 'var(--warning)' : 'transparent',
                        color: scheduleMode === 'reboot' ? '#000' : 'var(--text-secondary)'
                      }}
                    >
                      Reboot
                    </button>
                  </div>
                </div>

                {/* Hours & Minutes Inputs */}
                <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0.75rem' }}>
                  <div>
                    <label style={{ display: 'block', fontSize: '0.75rem', color: 'var(--text-secondary)', marginBottom: '0.3rem' }}>
                      HOURS
                    </label>
                    <input 
                      type="number" 
                      min="0"
                      max="72"
                      value={scheduleHours} 
                      onChange={(e) => setScheduleHours(Math.max(0, parseInt(e.target.value) || 0))}
                      className="input-field"
                      style={{ textAlign: 'center', fontSize: '1.2rem', fontWeight: 'bold', width: '100%' }}
                    />
                  </div>
                  <div>
                    <label style={{ display: 'block', fontSize: '0.75rem', color: 'var(--text-secondary)', marginBottom: '0.3rem' }}>
                      MINUTES
                    </label>
                    <input 
                      type="number" 
                      min="0"
                      max="59"
                      value={scheduleMinutes} 
                      onChange={(e) => setScheduleMinutes(Math.max(0, parseInt(e.target.value) || 0))}
                      className="input-field"
                      style={{ textAlign: 'center', fontSize: '1.2rem', fontWeight: 'bold', width: '100%' }}
                    />
                  </div>
                </div>

                {/* Quick Preset Buttons */}
                <div style={{ display: 'flex', gap: '0.4rem', flexWrap: 'wrap' }}>
                  {[
                    { label: '+15m', h: 0, m: 15 },
                    { label: '+30m', h: 0, m: 30 },
                    { label: '+1h', h: 1, m: 0 },
                    { label: '+2h', h: 2, m: 0 },
                    { label: '+4h', h: 4, m: 0 },
                  ].map(p => (
                    <button
                      key={p.label}
                      type="button"
                      onClick={() => { setScheduleHours(p.h); setScheduleMinutes(p.m); }}
                      style={{
                        padding: '0.2rem 0.5rem',
                        fontSize: '0.75rem',
                        background: 'rgba(255,255,255,0.05)',
                        border: '1px solid var(--glass-border)',
                        color: 'var(--text-secondary)',
                        borderRadius: '4px',
                        cursor: 'pointer'
                      }}
                    >
                      {p.label}
                    </button>
                  ))}
                </div>

                {/* Action Buttons */}
                <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.5rem' }}>
                  <button 
                    className="btn-primary" 
                    style={{ flex: 2, padding: '0.75rem 1rem' }}
                    onClick={() => handlePowerAction('schedule')}
                    disabled={isPowerActionLoading || !canManage}
                    title={privilegeHint}
                  >
                    Schedule {scheduleMode === 'reboot' ? 'Reboot' : 'Shutdown'}
                  </button>
                  {powerStatus?.scheduled && (
                    <button 
                      className="btn-text" 
                      style={{ flex: 1, color: 'var(--danger)', padding: '0.75rem' }}
                      onClick={() => handlePowerAction('cancel')}
                      disabled={isPowerActionLoading || !canManage}
                      title={privilegeHint}
                    >
                      Abort
                    </button>
                  )}
                </div>
              </div>
            </div>
          </div>
        </section>

        {/* Maintenance Tools */}
        <section>
          <div className="glass-panel" style={{ padding: '2rem' }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', marginBottom: '2rem' }}>
              <div style={{ 
                width: '48px', height: '48px', borderRadius: '12px', 
                background: 'rgba(56, 189, 248, 0.1)', color: 'var(--accent-color)',
                display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: '1.5rem'
              }}>
                <FaBolt />
              </div>
              <div>
                <h3 style={{ fontSize: '1.4rem' }}>Maintenance Protocols</h3>
                <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>Optimize system performance and storage integrity</p>
              </div>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: '1rem' }}>
              {[
                { id: 'memory_flush', label: 'FLUSH RAM', desc: 'Free PageCache & Inodes', icon: FaMemory, color: 'var(--success)' },
                { id: 'cache_clean', label: 'CLEAN CACHE', desc: 'Package & Journal Caches', icon: FaTrash, color: 'var(--accent-color)' },
                { id: 'swap_flush', label: 'FLUSH SWAP', desc: 'Evacuate Swap to RAM', icon: FaExchangeAlt, color: '#a78bfa' },
                { id: 'trim', label: 'SSD TRIM', desc: 'Optimize SSD Blocks', icon: FaBolt, color: 'var(--warning)' },
                { id: 'full_clean', label: 'DEEP CLEAN', desc: 'Execute All Protocols', icon: FaBroom, color: '#ec4899' },
              ].map(tool => (
                <button 
                  key={tool.id}
                  className="glass-panel"
                  style={{ 
                    padding: '1.5rem 1rem', 
                    display: 'flex', 
                    flexDirection: 'column', 
                    alignItems: 'center', 
                    gap: '0.75rem',
                    cursor: 'pointer',
                    transition: 'all 0.3s',
                    textAlign: 'center'
                  }}
                  onClick={() => handleMaintenance(tool.id)}
                  disabled={isLoading || !canManage}
                  title={privilegeHint}
                >
                  <tool.icon style={{ fontSize: '2.2rem', color: tool.color }} />
                  <div>
                    <div style={{ fontWeight: 800, fontSize: '0.85rem', letterSpacing: '0.05em' }}>{tool.label}</div>
                    <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', marginTop: '0.2rem' }}>{tool.desc}</div>
                  </div>
                </button>
              ))}
            </div>
          </div>
        </section>

        {/* DNS Management */}
        <section>
          <div className="glass-panel" style={{ padding: '2rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '1.5rem' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                <div style={{ 
                  width: '48px', height: '48px', borderRadius: '12px', 
                  background: 'rgba(52, 211, 153, 0.1)', color: 'var(--success)',
                  display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: '1.5rem'
                }}>
                  <FaSearch />
                </div>
                <div>
                  <h3 style={{ fontSize: '1.4rem' }}>Network Resolver</h3>
                  <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem' }}>DNS statistics and cache management</p>
                </div>
              </div>
              <div style={{ display: 'flex', gap: '0.5rem' }}>
                <button className="btn-sm" onClick={fetchDnsStats}><FaSync /> Refresh</button>
                <button className="btn-sm danger" onClick={handleFlushDns} disabled={!canManage} title={privilegeHint}><FaEraser /> Flush Cache</button>
              </div>
            </div>
            
            <div className="glass-panel" style={{ 
              background: 'rgba(0,0,0,0.3)', 
              padding: '1.5rem', 
              borderRadius: '12px',
              border: '1px solid rgba(255,255,255,0.05)'
            }}>
              <pre style={{ 
                margin: 0, fontSize: '0.8rem', color: 'var(--accent-color)', 
                fontFamily: 'JetBrains Mono, monospace', overflowX: 'auto' 
              }}>
                {dnsStats || "Awaiting resolver data..."}
              </pre>
            </div>
          </div>
        </section>
      </div>

      {/* RIGHT COLUMN: Persistent Logs */}
      <div className="glass-panel" style={{ 
        display: 'flex', 
        flexDirection: 'column', 
        overflow: 'hidden',
        background: 'rgba(15, 23, 42, 0.4)',
        borderLeft: '1px solid var(--glass-border)'
      }}>
        <div style={{ 
          padding: '1.5rem', 
          borderBottom: '1px solid var(--glass-border)', 
          display: 'flex', 
          justifyContent: 'space-between', 
          alignItems: 'center',
          background: 'rgba(0,0,0,0.2)'
        }}>
          <h3 style={{ fontSize: '1rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
            <FaHistory /> Action History
          </h3>
          <div style={{ display: 'flex', gap: '0.5rem' }}>
            <button className="btn-sm" onClick={downloadLogs} title="Download Logs"><FaDownload /></button>
            <button className="btn-sm danger" onClick={clearLogs} title="Clear Logs"><FaTrash /></button>
          </div>
        </div>

        <div style={{ flex: 1, overflowY: 'auto', padding: '1rem', display: 'flex', flexDirection: 'column', gap: '1rem' }} className="no-scrollbar">
          {logs.length === 0 ? (
            <div style={{ height: '100%', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', color: 'var(--text-secondary)', opacity: 0.5 }}>
              <FaTerminal style={{ fontSize: '3rem', marginBottom: '1rem' }} />
              <p>No activity recorded</p>
            </div>
          ) : (
            logs.map(log => (
              <div key={log.id} style={{ 
                padding: '1rem', 
                borderRadius: '8px', 
                background: 'rgba(255,255,255,0.03)',
                borderLeft: `3px solid ${log.status === 'success' ? 'var(--success)' : log.status === 'error' ? 'var(--danger)' : 'var(--accent-color)'}`
              }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.7rem', color: 'var(--text-secondary)', marginBottom: '0.5rem' }}>
                  <span style={{ fontWeight: 'bold', color: 'var(--text-primary)' }}>{log.action}</span>
                  <span>{log.timestamp}</span>
                </div>
                <div style={{ 
                  fontSize: '0.75rem', 
                  fontFamily: 'monospace', 
                  color: log.status === 'error' ? 'var(--danger)' : 'var(--text-primary)',
                  wordBreak: 'break-all',
                  opacity: 0.9
                }}>
                  {log.output}
                </div>
              </div>
            ))
          )}
          <div ref={logEndRef} />
        </div>
      </div>

    </div>
  );
};

export default SystemManagement;
