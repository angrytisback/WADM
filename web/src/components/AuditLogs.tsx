import React, { useState, useEffect, useCallback } from 'react';
import {
  FaHistory,
  FaSearch,
  FaSync,
  FaCheckCircle,
  FaTimesCircle,
  FaBan,
  FaChevronLeft,
  FaChevronRight,
  FaFilter,
} from 'react-icons/fa';
import type { AuditLog, AuditLogPage } from '../types';
import { useToast } from '../context/ToastContext';

export const AuditLogs: React.FC = () => {
  const [logs, setLogs] = useState<AuditLog[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [page, setPage] = useState<number>(1);
  const [limit] = useState<number>(25);
  const [totalPages, setTotalPages] = useState<number>(1);
  const [totalCount, setTotalCount] = useState<number>(0);

  // Filter states
  const [filterUser, setFilterUser] = useState<string>('');
  const [filterAction, setFilterAction] = useState<string>('');
  const [filterStatus, setFilterStatus] = useState<string>('');
  const [searchTerm, setSearchTerm] = useState<string>('');

  const { addToast } = useToast();

  const fetchLogs = useCallback(async () => {
    setLoading(true);
    try {
      const params = new URLSearchParams();
      params.append('page', page.toString());
      params.append('limit', limit.toString());
      if (filterUser.trim()) params.append('user', filterUser.trim());
      if (filterAction.trim()) params.append('action', filterAction.trim());
      if (filterStatus.trim()) params.append('status', filterStatus.trim());
      if (searchTerm.trim()) params.append('search', searchTerm.trim());

      const res = await fetch(`/api/audit-logs?${params.toString()}`);
      if (!res.ok) {
        throw new Error('Failed to fetch audit logs');
      }
      const data: AuditLogPage = await res.json();
      setLogs(data.logs || []);
      setTotalPages(data.total_pages || 1);
      setTotalCount(data.total || 0);
    } catch (err) {
      console.error(err);
      addToast('Failed to load audit logs', 'error');
    } finally {
      setLoading(false);
    }
  }, [page, limit, filterUser, filterAction, filterStatus, searchTerm, addToast]);

  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  const handleResetFilters = () => {
    setFilterUser('');
    setFilterAction('');
    setFilterStatus('');
    setSearchTerm('');
    setPage(1);
  };

  const getStatusBadge = (status: string) => {
    const s = status.toUpperCase();
    if (s === 'SUCCESS') {
      return (
        <span style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: '4px',
          padding: '2px 8px',
          borderRadius: '4px',
          fontSize: '0.75rem',
          fontWeight: 600,
          background: 'rgba(34, 197, 94, 0.15)',
          color: '#22c55e',
          border: '1px solid rgba(34, 197, 94, 0.3)'
        }}>
          <FaCheckCircle style={{ fontSize: '0.7rem' }} /> SUCCESS
        </span>
      );
    } else if (s === 'DENIED') {
      return (
        <span style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: '4px',
          padding: '2px 8px',
          borderRadius: '4px',
          fontSize: '0.75rem',
          fontWeight: 600,
          background: 'rgba(239, 68, 68, 0.2)',
          color: '#ef4444',
          border: '1px solid rgba(239, 68, 68, 0.4)'
        }}>
          <FaBan style={{ fontSize: '0.7rem' }} /> DENIED
        </span>
      );
    } else {
      return (
        <span style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: '4px',
          padding: '2px 8px',
          borderRadius: '4px',
          fontSize: '0.75rem',
          fontWeight: 600,
          background: 'rgba(245, 158, 11, 0.15)',
          color: '#f59e0b',
          border: '1px solid rgba(245, 158, 11, 0.3)'
        }}>
          <FaTimesCircle style={{ fontSize: '0.7rem' }} /> {status}
        </span>
      );
    }
  };

  const getRoleBadge = (role: string) => {
    const r = role.toLowerCase();
    let bg = 'rgba(148, 163, 184, 0.2)';
    let color = '#94a3b8';
    if (r === 'admin') {
      bg = 'rgba(239, 68, 68, 0.15)';
      color = '#f87171';
    } else if (r === 'operator') {
      bg = 'rgba(56, 189, 248, 0.15)';
      color = '#38bdf8';
    } else if (r === 'viewer') {
      bg = 'rgba(168, 85, 247, 0.15)';
      color = '#c084fc';
    }

    return (
      <span style={{
        padding: '2px 6px',
        borderRadius: '4px',
        fontSize: '0.7rem',
        fontWeight: 600,
        textTransform: 'uppercase',
        background: bg,
        color: color,
        border: `1px solid ${color}40`,
      }}>
        {role}
      </span>
    );
  };

  const formatTimestamp = (ts: string) => {
    try {
      const d = new Date(ts);
      return d.toLocaleString();
    } catch {
      return ts;
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem', height: '100%' }}>
      {/* Top Header Card */}
      <div className="glass-panel" style={{ padding: '1.25rem', borderRadius: '12px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
            <FaHistory style={{ fontSize: '1.4rem', color: 'var(--accent-color)' }} />
            <div>
              <h3 style={{ fontSize: '1.2rem', fontWeight: 700, margin: 0 }}>Security Audit Log</h3>
              <p style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', margin: '2px 0 0 0' }}>
                Immutable ledger of authentication, operational commands, and authorization checks
              </p>
            </div>
          </div>
          <button
            className="btn-primary"
            onClick={() => fetchLogs()}
            disabled={loading}
            style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.5rem 1rem' }}
          >
            <FaSync className={loading ? 'animate-spin' : ''} /> Refresh
          </button>
        </div>

        {/* Filter Controls */}
        <div style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))',
          gap: '0.75rem',
          alignItems: 'center'
        }}>
          <div>
            <label style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', display: 'block', marginBottom: '4px' }}>
              Search Description / Details
            </label>
            <div style={{ position: 'relative' }}>
              <input
                type="text"
                placeholder="Search..."
                value={searchTerm}
                onChange={(e) => { setSearchTerm(e.target.value); setPage(1); }}
                style={{
                  width: '100%',
                  padding: '0.45rem 0.6rem 0.45rem 2rem',
                  fontSize: '0.85rem',
                  borderRadius: '6px',
                  background: 'rgba(255, 255, 255, 0.05)',
                  border: '1px solid var(--glass-border)',
                  color: 'var(--text-primary)'
                }}
              />
              <FaSearch style={{ position: 'absolute', left: '0.65rem', top: '50%', transform: 'translateY(-50%)', color: 'var(--text-secondary)', fontSize: '0.8rem' }} />
            </div>
          </div>

          <div>
            <label style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', display: 'block', marginBottom: '4px' }}>
              Filter by User
            </label>
            <input
              type="text"
              placeholder="Username..."
              value={filterUser}
              onChange={(e) => { setFilterUser(e.target.value); setPage(1); }}
              style={{
                width: '100%',
                padding: '0.45rem 0.6rem',
                fontSize: '0.85rem',
                borderRadius: '6px',
                background: 'rgba(255, 255, 255, 0.05)',
                border: '1px solid var(--glass-border)',
                color: 'var(--text-primary)'
              }}
            />
          </div>

          <div>
            <label style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', display: 'block', marginBottom: '4px' }}>
              Filter by Action
            </label>
            <input
              type="text"
              placeholder="e.g. SERVICE_ACTION, DB_QUERY..."
              value={filterAction}
              onChange={(e) => { setFilterAction(e.target.value); setPage(1); }}
              style={{
                width: '100%',
                padding: '0.45rem 0.6rem',
                fontSize: '0.85rem',
                borderRadius: '6px',
                background: 'rgba(255, 255, 255, 0.05)',
                border: '1px solid var(--glass-border)',
                color: 'var(--text-primary)'
              }}
            />
          </div>

          <div>
            <label style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', display: 'block', marginBottom: '4px' }}>
              Status
            </label>
            <select
              value={filterStatus}
              onChange={(e) => { setFilterStatus(e.target.value); setPage(1); }}
              style={{
                width: '100%',
                padding: '0.45rem 0.6rem',
                fontSize: '0.85rem',
                borderRadius: '6px',
                background: 'rgba(255, 255, 255, 0.05)',
                border: '1px solid var(--glass-border)',
                color: 'var(--text-primary)'
              }}
            >
              <option value="" style={{ background: '#1e293b' }}>All Statuses</option>
              <option value="SUCCESS" style={{ background: '#1e293b' }}>SUCCESS</option>
              <option value="DENIED" style={{ background: '#1e293b' }}>DENIED</option>
              <option value="FAILED" style={{ background: '#1e293b' }}>FAILED</option>
            </select>
          </div>

          <div style={{ display: 'flex', alignItems: 'flex-end', height: '100%' }}>
            <button
              onClick={handleResetFilters}
              className="btn-secondary"
              style={{
                width: '100%',
                padding: '0.45rem 0.8rem',
                fontSize: '0.85rem',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: '0.4rem'
              }}
            >
              <FaFilter style={{ fontSize: '0.75rem' }} /> Clear Filters
            </button>
          </div>
        </div>
      </div>

      {/* Audit Log Table */}
      <div className="glass-panel" style={{ flex: 1, display: 'flex', flexDirection: 'column', borderRadius: '12px', overflow: 'hidden' }}>
        <div style={{ overflowX: 'auto', flex: 1 }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', textAlign: 'left', fontSize: '0.85rem' }}>
            <thead>
              <tr style={{ borderBottom: '1px solid var(--glass-border)', background: 'rgba(255, 255, 255, 0.02)' }}>
                <th style={{ padding: '0.75rem 1rem', color: 'var(--text-secondary)', fontWeight: 600 }}>Timestamp</th>
                <th style={{ padding: '0.75rem 1rem', color: 'var(--text-secondary)', fontWeight: 600 }}>User</th>
                <th style={{ padding: '0.75rem 1rem', color: 'var(--text-secondary)', fontWeight: 600 }}>Action</th>
                <th style={{ padding: '0.75rem 1rem', color: 'var(--text-secondary)', fontWeight: 600 }}>Target / Resource</th>
                <th style={{ padding: '0.75rem 1rem', color: 'var(--text-secondary)', fontWeight: 600 }}>Status</th>
                <th style={{ padding: '0.75rem 1rem', color: 'var(--text-secondary)', fontWeight: 600 }}>IP Address</th>
                <th style={{ padding: '0.75rem 1rem', color: 'var(--text-secondary)', fontWeight: 600 }}>Details</th>
              </tr>
            </thead>
            <tbody>
              {loading ? (
                <tr>
                  <td colSpan={7} style={{ textAlign: 'center', padding: '3rem', color: 'var(--text-secondary)' }}>
                    <FaSync className="animate-spin" style={{ fontSize: '1.5rem', marginBottom: '0.5rem' }} />
                    <div>Loading audit trail...</div>
                  </td>
                </tr>
              ) : logs.length === 0 ? (
                <tr>
                  <td colSpan={7} style={{ textAlign: 'center', padding: '3rem', color: 'var(--text-secondary)' }}>
                    No audit log records match the current criteria.
                  </td>
                </tr>
              ) : (
                logs.map((entry) => (
                  <tr
                    key={entry.id}
                    style={{
                      borderBottom: '1px solid rgba(255, 255, 255, 0.05)',
                      transition: 'background 0.2s',
                    }}
                    onMouseEnter={(e) => (e.currentTarget.style.background = 'rgba(255, 255, 255, 0.03)')}
                    onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                  >
                    <td style={{ padding: '0.65rem 1rem', color: 'var(--text-secondary)', whiteSpace: 'nowrap', fontFamily: 'monospace', fontSize: '0.8rem' }}>
                      {formatTimestamp(entry.timestamp)}
                    </td>
                    <td style={{ padding: '0.65rem 1rem' }}>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>{entry.username}</span>
                        {getRoleBadge(entry.role)}
                      </div>
                    </td>
                    <td style={{ padding: '0.65rem 1rem' }}>
                      <code style={{
                        background: 'rgba(0, 0, 0, 0.3)',
                        padding: '2px 6px',
                        borderRadius: '4px',
                        fontSize: '0.8rem',
                        color: '#38bdf8'
                      }}>
                        {entry.action}
                      </code>
                    </td>
                    <td style={{ padding: '0.65rem 1rem', color: 'var(--text-primary)', maxWidth: '200px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                      {entry.resource || <span style={{ color: 'var(--text-secondary)', fontStyle: 'italic' }}>—</span>}
                    </td>
                    <td style={{ padding: '0.65rem 1rem' }}>
                      {getStatusBadge(entry.status)}
                    </td>
                    <td style={{ padding: '0.65rem 1rem', fontFamily: 'monospace', fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                      {entry.ip_address}
                    </td>
                    <td style={{ padding: '0.65rem 1rem', color: 'var(--text-secondary)', maxWidth: '300px', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={entry.details || ''}>
                      {entry.details || <span style={{ fontStyle: 'italic' }}>—</span>}
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>

        {/* Pagination Bar */}
        <div style={{
          padding: '0.75rem 1rem',
          borderTop: '1px solid var(--glass-border)',
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          background: 'rgba(255, 255, 255, 0.01)'
        }}>
          <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
            Showing page <strong style={{ color: 'var(--text-primary)' }}>{page}</strong> of <strong style={{ color: 'var(--text-primary)' }}>{totalPages}</strong> ({totalCount} total events)
          </div>
          <div style={{ display: 'flex', gap: '0.5rem' }}>
            <button
              className="btn-secondary"
              onClick={() => setPage((p) => Math.max(p - 1, 1))}
              disabled={page <= 1 || loading}
              style={{ padding: '0.4rem 0.75rem', fontSize: '0.8rem', display: 'flex', alignItems: 'center', gap: '0.3rem' }}
            >
              <FaChevronLeft style={{ fontSize: '0.7rem' }} /> Previous
            </button>
            <button
              className="btn-secondary"
              onClick={() => setPage((p) => Math.min(p + 1, totalPages))}
              disabled={page >= totalPages || loading}
              style={{ padding: '0.4rem 0.75rem', fontSize: '0.8rem', display: 'flex', alignItems: 'center', gap: '0.3rem' }}
            >
              Next <FaChevronRight style={{ fontSize: '0.7rem' }} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
export default AuditLogs;
