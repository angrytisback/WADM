import React, { useState, useEffect, useCallback } from 'react';
import {
  FaUsers,
  FaUserPlus,
  FaKey,
  FaTrash,
  FaSync,
  FaTimes,
  FaUserShield,
} from 'react-icons/fa';
import type { User, UserRole } from '../types';
import { useAuth } from '../context/AuthContext';
import { useToast } from '../context/ToastContext';

export const UserManagement: React.FC = () => {
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState<boolean>(true);

  // Modals state
  const [showCreateModal, setShowCreateModal] = useState<boolean>(false);
  const [showRoleModal, setShowRoleModal] = useState<User | null>(null);
  const [showPasswordModal, setShowPasswordModal] = useState<User | null>(null);
  const [deleteConfirmUser, setDeleteConfirmUser] = useState<User | null>(null);

  // Form states
  const [createUsername, setCreateUsername] = useState<string>('');
  const [createPassword, setCreatePassword] = useState<string>('');
  const [createRole, setCreateRole] = useState<UserRole>('operator');

  const [newRole, setNewRole] = useState<UserRole>('operator');
  const [newPassword, setNewPassword] = useState<string>('');

  const [actionLoading, setActionLoading] = useState<boolean>(false);

  const { user: currentUser } = useAuth();
  const { addToast } = useToast();

  const fetchUsers = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch('/api/users');
      if (!res.ok) {
        throw new Error('Failed to load users');
      }
      const data: User[] = await res.json();
      setUsers(data);
    } catch (err) {
      console.error(err);
      addToast('Failed to load users', 'error');
    } finally {
      setLoading(false);
    }
  }, [addToast]);

  useEffect(() => {
    fetchUsers();
  }, [fetchUsers]);

  const handleCreateUser = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!createUsername.trim() || !createPassword.trim()) {
      addToast('Username and password are required', 'warning');
      return;
    }

    setActionLoading(true);
    try {
      const res = await fetch('/api/users', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          username: createUsername.trim(),
          password: createPassword,
          role: createRole,
        }),
      });

      if (!res.ok) {
        const err = await res.json().catch(() => ({ error: 'Failed to create user' }));
        throw new Error(err.error || 'Failed to create user');
      }

      addToast(`User ${createUsername} created successfully`, 'success');
      setShowCreateModal(false);
      setCreateUsername('');
      setCreatePassword('');
      setCreateRole('operator');
      fetchUsers();
    } catch (err) {
      addToast(err instanceof Error ? err.message : 'Failed to create user', 'error');
    } finally {
      setActionLoading(false);
    }
  };

  const handleUpdateRole = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!showRoleModal) return;

    setActionLoading(true);
    try {
      const res = await fetch(`/api/users/${showRoleModal.id}/role`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ role: newRole }),
      });

      if (!res.ok) {
        const err = await res.json().catch(() => ({ error: 'Failed to update role' }));
        throw new Error(err.error || 'Failed to update role');
      }

      addToast(`Updated role for ${showRoleModal.username}`, 'success');
      setShowRoleModal(null);
      fetchUsers();
    } catch (err) {
      addToast(err instanceof Error ? err.message : 'Failed to update role', 'error');
    } finally {
      setActionLoading(false);
    }
  };

  const handleUpdatePassword = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!showPasswordModal) return;

    if (!newPassword.trim()) {
      addToast('Password cannot be empty', 'warning');
      return;
    }

    setActionLoading(true);
    try {
      const res = await fetch(`/api/users/${showPasswordModal.id}/password`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ password: newPassword }),
      });

      if (!res.ok) {
        const err = await res.json().catch(() => ({ error: 'Failed to reset password' }));
        throw new Error(err.error || 'Failed to reset password');
      }

      addToast(`Password reset for ${showPasswordModal.username}`, 'success');
      setShowPasswordModal(null);
      setNewPassword('');
    } catch (err) {
      addToast(err instanceof Error ? err.message : 'Failed to reset password', 'error');
    } finally {
      setActionLoading(false);
    }
  };

  const handleDeleteUser = async () => {
    if (!deleteConfirmUser) return;

    setActionLoading(true);
    try {
      const res = await fetch(`/api/users/${deleteConfirmUser.id}`, {
        method: 'DELETE',
      });

      if (!res.ok) {
        const err = await res.json().catch(() => ({ error: 'Failed to delete user' }));
        throw new Error(err.error || 'Failed to delete user');
      }

      addToast(`User ${deleteConfirmUser.username} deleted`, 'success');
      setDeleteConfirmUser(null);
      fetchUsers();
    } catch (err) {
      addToast(err instanceof Error ? err.message : 'Failed to delete user', 'error');
    } finally {
      setActionLoading(false);
    }
  };

  const getRoleBadge = (role: UserRole) => {
    let bg = 'rgba(148, 163, 184, 0.2)';
    let color = '#94a3b8';
    if (role === 'admin') {
      bg = 'rgba(239, 68, 68, 0.15)';
      color = '#f87171';
    } else if (role === 'operator') {
      bg = 'rgba(56, 189, 248, 0.15)';
      color = '#38bdf8';
    } else if (role === 'viewer') {
      bg = 'rgba(168, 85, 247, 0.15)';
      color = '#c084fc';
    }

    return (
      <span style={{
        padding: '2px 8px',
        borderRadius: '4px',
        fontSize: '0.75rem',
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

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem', height: '100%' }}>
      {/* Top Header Card */}
      <div className="glass-panel" style={{ padding: '1.25rem', borderRadius: '12px' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
            <FaUsers style={{ fontSize: '1.4rem', color: 'var(--accent-color)' }} />
            <div>
              <h3 style={{ fontSize: '1.2rem', fontWeight: 700, margin: 0 }}>Role-Based Access Control (RBAC)</h3>
              <p style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', margin: '2px 0 0 0' }}>
                Manage team accounts, define roles (Viewer, Operator, Admin), and enforce least-privilege security
              </p>
            </div>
          </div>
          <div style={{ display: 'flex', gap: '0.5rem' }}>
            <button
              className="btn-secondary"
              onClick={() => fetchUsers()}
              disabled={loading}
              style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', padding: '0.5rem 0.8rem' }}
            >
              <FaSync className={loading ? 'animate-spin' : ''} /> Refresh
            </button>
            <button
              className="btn-primary"
              onClick={() => setShowCreateModal(true)}
              style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', padding: '0.5rem 1rem' }}
            >
              <FaUserPlus /> Add User
            </button>
          </div>
        </div>
      </div>

      {/* Users Table */}
      <div className="glass-panel" style={{ flex: 1, display: 'flex', flexDirection: 'column', borderRadius: '12px', overflow: 'hidden' }}>
        <div style={{ overflowX: 'auto', flex: 1 }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', textAlign: 'left', fontSize: '0.85rem' }}>
            <thead>
              <tr style={{ borderBottom: '1px solid var(--glass-border)', background: 'rgba(255, 255, 255, 0.02)' }}>
                <th style={{ padding: '0.75rem 1rem', color: 'var(--text-secondary)', fontWeight: 600 }}>ID</th>
                <th style={{ padding: '0.75rem 1rem', color: 'var(--text-secondary)', fontWeight: 600 }}>Username</th>
                <th style={{ padding: '0.75rem 1rem', color: 'var(--text-secondary)', fontWeight: 600 }}>Role</th>
                <th style={{ padding: '0.75rem 1rem', color: 'var(--text-secondary)', fontWeight: 600 }}>Created Date</th>
                <th style={{ padding: '0.75rem 1rem', color: 'var(--text-secondary)', fontWeight: 600, textAlign: 'right' }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {loading ? (
                <tr>
                  <td colSpan={5} style={{ textAlign: 'center', padding: '3rem', color: 'var(--text-secondary)' }}>
                    <FaSync className="animate-spin" style={{ fontSize: '1.5rem', marginBottom: '0.5rem' }} />
                    <div>Loading accounts...</div>
                  </td>
                </tr>
              ) : users.length === 0 ? (
                <tr>
                  <td colSpan={5} style={{ textAlign: 'center', padding: '3rem', color: 'var(--text-secondary)' }}>
                    No users configured.
                  </td>
                </tr>
              ) : (
                users.map((u) => {
                  const isCurrent = currentUser?.username === u.username;
                  return (
                    <tr
                      key={u.id}
                      style={{
                        borderBottom: '1px solid rgba(255, 255, 255, 0.05)',
                        transition: 'background 0.2s',
                      }}
                      onMouseEnter={(e) => (e.currentTarget.style.background = 'rgba(255, 255, 255, 0.03)')}
                      onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                    >
                      <td style={{ padding: '0.75rem 1rem', color: 'var(--text-secondary)', fontFamily: 'monospace' }}>
                        #{u.id}
                      </td>
                      <td style={{ padding: '0.75rem 1rem' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                          <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>{u.username}</span>
                          {isCurrent && (
                            <span style={{ fontSize: '0.7rem', padding: '1px 6px', borderRadius: '4px', background: 'rgba(56, 189, 248, 0.2)', color: '#38bdf8' }}>
                              You
                            </span>
                          )}
                        </div>
                      </td>
                      <td style={{ padding: '0.75rem 1rem' }}>
                        {getRoleBadge(u.role)}
                      </td>
                      <td style={{ padding: '0.75rem 1rem', color: 'var(--text-secondary)' }}>
                        {new Date(u.created_at).toLocaleString()}
                      </td>
                      <td style={{ padding: '0.75rem 1rem', textAlign: 'right' }}>
                        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.5rem' }}>
                          <button
                            className="btn-secondary"
                            onClick={() => { setShowRoleModal(u); setNewRole(u.role); }}
                            style={{ padding: '0.35rem 0.6rem', fontSize: '0.75rem', display: 'flex', alignItems: 'center', gap: '0.3rem' }}
                            title="Change Role"
                          >
                            <FaUserShield /> Role
                          </button>
                          <button
                            className="btn-secondary"
                            onClick={() => { setShowPasswordModal(u); setNewPassword(''); }}
                            style={{ padding: '0.35rem 0.6rem', fontSize: '0.75rem', display: 'flex', alignItems: 'center', gap: '0.3rem' }}
                            title="Reset Password"
                          >
                            <FaKey /> Password
                          </button>
                          <button
                            className="btn-secondary"
                            disabled={isCurrent}
                            onClick={() => setDeleteConfirmUser(u)}
                            style={{
                              padding: '0.35rem 0.6rem',
                              fontSize: '0.75rem',
                              display: 'flex',
                              alignItems: 'center',
                              gap: '0.3rem',
                              color: isCurrent ? 'var(--text-secondary)' : '#ef4444',
                              borderColor: isCurrent ? undefined : 'rgba(239, 68, 68, 0.3)',
                              opacity: isCurrent ? 0.4 : 1,
                              cursor: isCurrent ? 'not-allowed' : 'pointer'
                            }}
                            title={isCurrent ? "Cannot delete currently logged in account" : "Delete User"}
                          >
                            <FaTrash />
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Create User Modal */}
      {showCreateModal && (
        <div style={{
          position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
          background: 'rgba(0, 0, 0, 0.75)', backdropFilter: 'blur(4px)',
          display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1000
        }}>
          <div className="glass-panel" style={{ width: '400px', maxWidth: '90%', borderRadius: '12px', padding: '1.5rem', background: '#0f172a' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.25rem' }}>
              <h3 style={{ margin: 0, fontSize: '1.1rem', fontWeight: 700 }}>Add New User</h3>
              <button onClick={() => setShowCreateModal(false)} style={{ background: 'none', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer' }}>
                <FaTimes />
              </button>
            </div>
            <form onSubmit={handleCreateUser} style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              <div>
                <label style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', display: 'block', marginBottom: '4px' }}>Username</label>
                <input
                  type="text"
                  required
                  value={createUsername}
                  onChange={(e) => setCreateUsername(e.target.value)}
                  placeholder="e.g. jdoe"
                  style={{ width: '100%', padding: '0.5rem', borderRadius: '6px', background: 'rgba(255, 255, 255, 0.05)', border: '1px solid var(--glass-border)', color: '#fff' }}
                />
              </div>
              <div>
                <label style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', display: 'block', marginBottom: '4px' }}>Password</label>
                <input
                  type="password"
                  required
                  value={createPassword}
                  onChange={(e) => setCreatePassword(e.target.value)}
                  placeholder="Secure password..."
                  style={{ width: '100%', padding: '0.5rem', borderRadius: '6px', background: 'rgba(255, 255, 255, 0.05)', border: '1px solid var(--glass-border)', color: '#fff' }}
                />
              </div>
              <div>
                <label style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', display: 'block', marginBottom: '4px' }}>Role</label>
                <select
                  value={createRole}
                  onChange={(e) => setCreateRole(e.target.value as UserRole)}
                  style={{ width: '100%', padding: '0.5rem', borderRadius: '6px', background: 'rgba(255, 255, 255, 0.05)', border: '1px solid var(--glass-border)', color: '#fff' }}
                >
                  <option value="viewer" style={{ background: '#1e293b' }}>Viewer (Read-only)</option>
                  <option value="operator" style={{ background: '#1e293b' }}>Operator (Service, Docker, Backups)</option>
                  <option value="admin" style={{ background: '#1e293b' }}>Admin (Full root/terminal/firewall access)</option>
                </select>
              </div>
              <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.5rem', marginTop: '0.5rem' }}>
                <button type="button" className="btn-secondary" onClick={() => setShowCreateModal(false)} disabled={actionLoading}>
                  Cancel
                </button>
                <button type="submit" className="btn-primary" disabled={actionLoading}>
                  {actionLoading ? 'Creating...' : 'Create Account'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Change Role Modal */}
      {showRoleModal && (
        <div style={{
          position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
          background: 'rgba(0, 0, 0, 0.75)', backdropFilter: 'blur(4px)',
          display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1000
        }}>
          <div className="glass-panel" style={{ width: '380px', maxWidth: '90%', borderRadius: '12px', padding: '1.5rem', background: '#0f172a' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.25rem' }}>
              <h3 style={{ margin: 0, fontSize: '1.1rem', fontWeight: 700 }}>Update Role: {showRoleModal.username}</h3>
              <button onClick={() => setShowRoleModal(null)} style={{ background: 'none', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer' }}>
                <FaTimes />
              </button>
            </div>
            <form onSubmit={handleUpdateRole} style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              <div>
                <label style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', display: 'block', marginBottom: '4px' }}>Select Role</label>
                <select
                  value={newRole}
                  onChange={(e) => setNewRole(e.target.value as UserRole)}
                  style={{ width: '100%', padding: '0.5rem', borderRadius: '6px', background: 'rgba(255, 255, 255, 0.05)', border: '1px solid var(--glass-border)', color: '#fff' }}
                >
                  <option value="viewer" style={{ background: '#1e293b' }}>Viewer (Read-only)</option>
                  <option value="operator" style={{ background: '#1e293b' }}>Operator (Service, Docker, Backups)</option>
                  <option value="admin" style={{ background: '#1e293b' }}>Admin (Full root/terminal/firewall access)</option>
                </select>
              </div>
              <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.5rem', marginTop: '0.5rem' }}>
                <button type="button" className="btn-secondary" onClick={() => setShowRoleModal(null)} disabled={actionLoading}>
                  Cancel
                </button>
                <button type="submit" className="btn-primary" disabled={actionLoading}>
                  {actionLoading ? 'Saving...' : 'Update Role'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Reset Password Modal */}
      {showPasswordModal && (
        <div style={{
          position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
          background: 'rgba(0, 0, 0, 0.75)', backdropFilter: 'blur(4px)',
          display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1000
        }}>
          <div className="glass-panel" style={{ width: '380px', maxWidth: '90%', borderRadius: '12px', padding: '1.5rem', background: '#0f172a' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.25rem' }}>
              <h3 style={{ margin: 0, fontSize: '1.1rem', fontWeight: 700 }}>Reset Password: {showPasswordModal.username}</h3>
              <button onClick={() => setShowPasswordModal(null)} style={{ background: 'none', border: 'none', color: 'var(--text-secondary)', cursor: 'pointer' }}>
                <FaTimes />
              </button>
            </div>
            <form onSubmit={handleUpdatePassword} style={{ display: 'flex', flexDirection: 'column', gap: '1rem' }}>
              <div>
                <label style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', display: 'block', marginBottom: '4px' }}>New Password</label>
                <input
                  type="password"
                  required
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  placeholder="Enter new password..."
                  style={{ width: '100%', padding: '0.5rem', borderRadius: '6px', background: 'rgba(255, 255, 255, 0.05)', border: '1px solid var(--glass-border)', color: '#fff' }}
                />
              </div>
              <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.5rem', marginTop: '0.5rem' }}>
                <button type="button" className="btn-secondary" onClick={() => setShowPasswordModal(null)} disabled={actionLoading}>
                  Cancel
                </button>
                <button type="submit" className="btn-primary" disabled={actionLoading}>
                  {actionLoading ? 'Saving...' : 'Update Password'}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Delete Confirmation Modal */}
      {deleteConfirmUser && (
        <div style={{
          position: 'fixed', top: 0, left: 0, right: 0, bottom: 0,
          background: 'rgba(0, 0, 0, 0.75)', backdropFilter: 'blur(4px)',
          display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 1000
        }}>
          <div className="glass-panel" style={{ width: '380px', maxWidth: '90%', borderRadius: '12px', padding: '1.5rem', background: '#0f172a' }}>
            <h3 style={{ margin: 0, fontSize: '1.1rem', fontWeight: 700, color: '#ef4444' }}>Delete Account</h3>
            <p style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', margin: '1rem 0' }}>
              Are you sure you want to delete user <strong style={{ color: '#fff' }}>{deleteConfirmUser.username}</strong>? This action is permanent and cannot be undone.
            </p>
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.5rem' }}>
              <button className="btn-secondary" onClick={() => setDeleteConfirmUser(null)} disabled={actionLoading}>
                Cancel
              </button>
              <button
                className="btn-primary"
                onClick={handleDeleteUser}
                disabled={actionLoading}
                style={{ background: '#ef4444', borderColor: '#ef4444' }}
              >
                {actionLoading ? 'Deleting...' : 'Delete User'}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
export default UserManagement;
