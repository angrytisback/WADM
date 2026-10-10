import React, { useState, useEffect, useCallback, useMemo } from 'react';
import {
  FaPuzzlePiece,
  FaShieldAlt,
  FaServer,
  FaLock,
  FaDatabase,
  FaDownload,
  FaTrash,
  FaSync,
  FaSearch,
  FaCheck,
  FaExternalLinkAlt,
  FaGithub,
  FaArrowUp,
  FaExclamationTriangle
} from 'react-icons/fa';
import { useToast } from '../context/ToastContext';
import { useJobs } from '../context/JobContext';
import { useAuth } from '../context/AuthContext';
import type { PluginStoreItemView } from '../types';

interface PluginStoreProps {
  onPluginInstalled?: () => void;
}

export const PluginStore: React.FC<PluginStoreProps> = ({ onPluginInstalled }) => {
  const [items, setItems] = useState<PluginStoreItemView[]>([]);
  const [loading, setLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string>('All');
  const [actionLoading, setActionLoading] = useState<Record<string, string>>({});

  const { addToast } = useToast();
  const { trackJob } = useJobs();
  const { isAdmin } = useAuth();

  const fetchCatalog = useCallback(async (showToast = false) => {
    setLoading(true);
    try {
      const res = await fetch('/api/plugins/store');
      if (res.ok) {
        const data: PluginStoreItemView[] = await res.json();
        setItems(data);
        if (showToast) {
          addToast('Plugin catalog loaded', 'info');
        }
      } else {
        addToast('Failed to load plugin store catalog', 'error');
      }
    } catch (err) {
      console.error(err);
      addToast('Error connecting to plugin store API', 'error');
    } finally {
      setLoading(false);
    }
  }, [addToast]);

  useEffect(() => {
    fetchCatalog();
  }, [fetchCatalog]);

  const handleRefreshCatalog = async () => {
    if (!isAdmin) {
      addToast('Administrator role required to refresh catalog cache', 'warning');
      return;
    }
    setRefreshing(true);
    try {
      const res = await fetch('/api/plugins/store/refresh', { method: 'POST' });
      if (res.ok) {
        addToast('Store catalog cache refreshed from remote repository', 'success');
        await fetchCatalog();
      } else {
        const err = await res.json();
        addToast(err.error || 'Failed to refresh catalog cache', 'error');
      }
    } catch {
      addToast('Network error while refreshing catalog', 'error');
    } finally {
      setRefreshing(false);
    }
  };

  const handleInstall = async (plugin: PluginStoreItemView) => {
    if (!isAdmin) {
      addToast('Administrator role required to install plugins', 'warning');
      return;
    }

    setActionLoading(prev => ({ ...prev, [plugin.id]: 'installing' }));
    try {
      const res = await fetch(`/api/plugins/store/${plugin.id}/install`, { method: 'POST' });
      const data = await res.json();

      if (res.status === 202 && data.job_id) {
        trackJob(data.job_id, `Installing ${plugin.name}`);
        addToast(`Installation of '${plugin.name}' queued`, 'info');
        if (onPluginInstalled) {
          onPluginInstalled();
        }
        await fetchCatalog();
      } else {
        addToast(data.message || data.error || `Failed to start installation for ${plugin.name}`, 'error');
      }
    } catch {
      addToast(`Error dispatching install job for ${plugin.name}`, 'error');
    } finally {
      setActionLoading(prev => {
        const next = { ...prev };
        delete next[plugin.id];
        return next;
      });
    }
  };

  const handleUpdate = async (plugin: PluginStoreItemView) => {
    if (!isAdmin) {
      addToast('Administrator role required to update plugins', 'warning');
      return;
    }

    setActionLoading(prev => ({ ...prev, [plugin.id]: 'updating' }));
    try {
      const res = await fetch(`/api/plugins/store/${plugin.id}/update`, { method: 'POST' });
      const data = await res.json();

      if (res.status === 202 && data.job_id) {
        trackJob(data.job_id, `Updating ${plugin.name} to v${plugin.version}`);
        addToast(`Update of '${plugin.name}' queued`, 'info');
        await fetchCatalog();
      } else {
        addToast(data.message || data.error || `Failed to start update for ${plugin.name}`, 'error');
      }
    } catch {
      addToast(`Error dispatching update job for ${plugin.name}`, 'error');
    } finally {
      setActionLoading(prev => {
        const next = { ...prev };
        delete next[plugin.id];
        return next;
      });
    }
  };

  const handleUninstall = async (plugin: PluginStoreItemView) => {
    if (!isAdmin) {
      addToast('Administrator role required to uninstall plugins', 'warning');
      return;
    }

    const confirmed = window.confirm(
      `Are you sure you want to completely uninstall '${plugin.name}'? This will stop its process and delete all related binary files.`
    );
    if (!confirmed) return;

    setActionLoading(prev => ({ ...prev, [plugin.id]: 'uninstalling' }));
    try {
      const res = await fetch(`/api/plugins/store/${plugin.id}`, { method: 'DELETE' });
      const data = await res.json();

      if (res.status === 202 && data.job_id) {
        trackJob(data.job_id, `Uninstalling ${plugin.name}`);
        addToast(`Uninstallation of '${plugin.name}' queued`, 'info');
        await fetchCatalog();
      } else {
        addToast(data.message || data.error || `Failed to uninstall ${plugin.name}`, 'error');
      }
    } catch {
      addToast(`Error dispatching uninstall job for ${plugin.name}`, 'error');
    } finally {
      setActionLoading(prev => {
        const next = { ...prev };
        delete next[plugin.id];
        return next;
      });
    }
  };

  const categories = useMemo(() => {
    const set = new Set<string>();
    items.forEach(i => {
      if (i.category) set.add(i.category);
    });
    return ['All', ...Array.from(set)];
  }, [items]);

  const filteredItems = useMemo(() => {
    return items.filter(i => {
      const matchesCategory = selectedCategory === 'All' || i.category.toLowerCase() === selectedCategory.toLowerCase();
      const q = searchQuery.trim().toLowerCase();
      const matchesSearch =
        !q ||
        i.name.toLowerCase().includes(q) ||
        i.id.toLowerCase().includes(q) ||
        i.description.toLowerCase().includes(q) ||
        i.author.toLowerCase().includes(q) ||
        i.capabilities.some(c => c.toLowerCase().includes(q));
      return matchesCategory && matchesSearch;
    });
  }, [items, selectedCategory, searchQuery]);

  const renderIcon = (iconKey: string) => {
    switch (iconKey) {
      case 'FaShieldAlt':
        return <FaShieldAlt style={{ fontSize: '1.75rem', color: '#10b981' }} />;
      case 'FaServer':
        return <FaServer style={{ fontSize: '1.75rem', color: '#3b82f6' }} />;
      case 'FaLock':
        return <FaLock style={{ fontSize: '1.75rem', color: '#f59e0b' }} />;
      case 'FaDatabase':
        return <FaDatabase style={{ fontSize: '1.75rem', color: '#8b5cf6' }} />;
      default:
        return <FaPuzzlePiece style={{ fontSize: '1.75rem', color: 'var(--accent-color)' }} />;
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
      {/* Header and Controls */}
      <div className="glass-card" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', gap: '1.2rem' }}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '1rem' }}>
          <div>
            <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem', marginBottom: '0.3rem' }}>
              <FaPuzzlePiece style={{ fontSize: '1.4rem', color: 'var(--accent-color)' }} />
              <h3 style={{ margin: 0, fontSize: '1.3rem', fontWeight: 700 }}>WADM Dynamic Plugin Store</h3>
            </div>
            <p style={{ margin: 0, color: 'var(--text-secondary)', fontSize: '0.88rem' }}>
              Discover, install, and manage official and community-curated sidecar plugins verified with SHA-256 integrity and Zip Slip isolation.
            </p>
          </div>

          <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
            <button
              className="btn-secondary"
              onClick={() => handleRefreshCatalog()}
              disabled={refreshing || !isAdmin}
              title={!isAdmin ? 'Admin role required to refresh catalog' : 'Force re-fetch remote index from GitHub'}
              style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.85rem' }}
            >
              <FaSync className={refreshing ? 'spin' : ''} />
              <span>{refreshing ? 'Refreshing...' : 'Refresh Catalog'}</span>
            </button>
          </div>
        </div>

        {/* Filter Bar */}
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '1rem' }}>
          {/* Category Tabs */}
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', flexWrap: 'wrap' }}>
            {categories.map(cat => (
              <button
                key={cat}
                onClick={() => setSelectedCategory(cat)}
                style={{
                  padding: '0.45rem 0.9rem',
                  borderRadius: '20px',
                  border: '1px solid',
                  borderColor: selectedCategory === cat ? 'var(--accent-color)' : 'rgba(255, 255, 255, 0.1)',
                  background: selectedCategory === cat ? 'var(--accent-color)' : 'rgba(255, 255, 255, 0.04)',
                  color: selectedCategory === cat ? '#fff' : 'var(--text-secondary)',
                  fontSize: '0.82rem',
                  fontWeight: selectedCategory === cat ? 600 : 500,
                  cursor: 'pointer',
                  transition: 'all 0.15s ease-in-out'
                }}
              >
                {cat}
              </button>
            ))}
          </div>

          {/* Search Input */}
          <div style={{ position: 'relative', minWidth: '260px' }}>
            <FaSearch style={{ position: 'absolute', left: '12px', top: '50%', transform: 'translateY(-50%)', color: 'var(--text-secondary)', fontSize: '0.85rem' }} />
            <input
              type="text"
              placeholder="Search plugins, tags, author..."
              value={searchQuery}
              onChange={e => setSearchQuery(e.target.value)}
              style={{
                width: '100%',
                padding: '0.5rem 0.9rem 0.5rem 2.2rem',
                borderRadius: '8px',
                border: '1px solid rgba(255, 255, 255, 0.12)',
                background: 'rgba(0, 0, 0, 0.25)',
                color: 'var(--text-primary)',
                fontSize: '0.85rem',
                outline: 'none'
              }}
            />
          </div>
        </div>
      </div>

      {/* Catalog Cards Grid */}
      {loading ? (
        <div className="glass-card" style={{ padding: '3rem', textAlign: 'center', color: 'var(--text-secondary)' }}>
          <FaSync className="spin" style={{ fontSize: '2rem', marginBottom: '1rem', color: 'var(--accent-color)' }} />
          <p style={{ margin: 0 }}>Connecting to plugin store repository...</p>
        </div>
      ) : filteredItems.length === 0 ? (
        <div className="glass-card" style={{ padding: '3rem', textAlign: 'center', color: 'var(--text-secondary)' }}>
          <FaPuzzlePiece style={{ fontSize: '3rem', opacity: 0.3, marginBottom: '1rem' }} />
          <h4 style={{ margin: '0 0 0.5rem 0', color: 'var(--text-primary)' }}>No matching plugins found</h4>
          <p style={{ margin: 0, fontSize: '0.9rem' }}>
            Try adjusting your search criteria or switching category filters.
          </p>
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(360px, 1fr))', gap: '1.25rem' }}>
          {filteredItems.map(plugin => {
            const op = actionLoading[plugin.id];
            const isInstalled = plugin.is_installed;
            const hasUpdate = plugin.has_update;

            return (
              <div
                key={plugin.id}
                className="glass-card"
                style={{
                  padding: '1.4rem',
                  display: 'flex',
                  flexDirection: 'column',
                  justifyContent: 'space-between',
                  gap: '1.2rem',
                  border: isInstalled ? '1px solid rgba(16, 185, 129, 0.3)' : '1px solid rgba(255, 255, 255, 0.08)',
                  background: isInstalled ? 'rgba(16, 185, 129, 0.02)' : 'rgba(255, 255, 255, 0.02)',
                  transition: 'transform 0.15s ease, box-shadow 0.15s ease'
                }}
              >
                {/* Card Top: Icon, Titles & Badges */}
                <div style={{ display: 'flex', flexDirection: 'column', gap: '0.8rem' }}>
                  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: '0.75rem' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.85rem' }}>
                      <div
                        style={{
                          width: '46px',
                          height: '46px',
                          borderRadius: '10px',
                          background: 'rgba(255, 255, 255, 0.05)',
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center'
                        }}
                      >
                        {renderIcon(plugin.icon)}
                      </div>
                      <div>
                        <h4 style={{ margin: 0, fontSize: '1.05rem', fontWeight: 700, color: 'var(--text-primary)' }}>
                          {plugin.name}
                        </h4>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginTop: '0.2rem' }}>
                          <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>
                            by {plugin.author}
                          </span>
                          <span style={{ fontSize: '0.7rem', padding: '0.1rem 0.45rem', borderRadius: '4px', background: 'rgba(255, 255, 255, 0.07)', color: 'var(--text-secondary)' }}>
                            {plugin.category}
                          </span>
                        </div>
                      </div>
                    </div>

                    {/* Status & Version Badges */}
                    <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end', gap: '0.3rem' }}>
                      <span
                        style={{
                          fontSize: '0.75rem',
                          fontWeight: 700,
                          padding: '0.2rem 0.55rem',
                          borderRadius: '6px',
                          background: isInstalled ? 'rgba(16, 185, 129, 0.15)' : 'rgba(59, 130, 246, 0.15)',
                          color: isInstalled ? '#10b981' : '#3b82f6',
                          border: `1px solid ${isInstalled ? 'rgba(16, 185, 129, 0.3)' : 'rgba(59, 130, 246, 0.3)'}`,
                          display: 'flex',
                          alignItems: 'center',
                          gap: '0.3rem'
                        }}
                      >
                        {isInstalled ? <FaCheck style={{ fontSize: '0.7rem' }} /> : null}
                        v{plugin.version}
                      </span>
                      {isInstalled && (
                        <span style={{ fontSize: '0.7rem', color: '#10b981' }}>
                          Installed {plugin.installed_version ? `(v${plugin.installed_version})` : ''}
                        </span>
                      )}
                    </div>
                  </div>

                  {/* Description */}
                  <p style={{ margin: 0, fontSize: '0.85rem', color: 'var(--text-secondary)', lineHeight: '1.45' }}>
                    {plugin.description}
                  </p>

                  {/* Capabilities Tags */}
                  {plugin.capabilities && plugin.capabilities.length > 0 && (
                    <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.35rem', marginTop: '0.2rem' }}>
                      {plugin.capabilities.map(cap => (
                        <span
                          key={cap}
                          style={{
                            fontSize: '0.7rem',
                            padding: '0.15rem 0.45rem',
                            borderRadius: '4px',
                            background: 'rgba(255, 255, 255, 0.04)',
                            border: '1px solid rgba(255, 255, 255, 0.08)',
                            color: 'var(--text-secondary)',
                            fontFamily: 'monospace'
                          }}
                        >
                          {cap}
                        </span>
                      ))}
                    </div>
                  )}
                </div>

                {/* Card Bottom: Links & Actions */}
                <div
                  style={{
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    borderTop: '1px solid rgba(255, 255, 255, 0.06)',
                    paddingTop: '0.9rem',
                    flexWrap: 'wrap',
                    gap: '0.6rem'
                  }}
                >
                  {/* External Links */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                    {plugin.repository && (
                      <a
                        href={plugin.repository}
                        target="_blank"
                        rel="noreferrer"
                        title="View Source on GitHub"
                        style={{
                          color: 'var(--text-secondary)',
                          fontSize: '1rem',
                          display: 'flex',
                          alignItems: 'center',
                          textDecoration: 'none',
                          transition: 'color 0.15s ease'
                        }}
                      >
                        <FaGithub />
                      </a>
                    )}
                    {plugin.homepage && (
                      <a
                        href={plugin.homepage}
                        target="_blank"
                        rel="noreferrer"
                        title="Plugin Homepage / Docs"
                        style={{
                          color: 'var(--text-secondary)',
                          fontSize: '0.85rem',
                          display: 'flex',
                          alignItems: 'center',
                          textDecoration: 'none',
                          transition: 'color 0.15s ease'
                        }}
                      >
                        <FaExternalLinkAlt />
                      </a>
                    )}
                  </div>

                  {/* Action Buttons */}
                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                    {isInstalled ? (
                      <>
                        {hasUpdate && (
                          <button
                            className="btn-primary"
                            disabled={!!op || !isAdmin}
                            onClick={() => handleUpdate(plugin)}
                            title={!isAdmin ? 'Admin role required' : `Update to version ${plugin.version}`}
                            style={{
                              display: 'flex',
                              alignItems: 'center',
                              gap: '0.4rem',
                              fontSize: '0.8rem',
                              padding: '0.4rem 0.8rem',
                              background: '#f59e0b',
                              borderColor: '#d97706',
                              color: '#fff'
                            }}
                          >
                            <FaArrowUp />
                            <span>{op === 'updating' ? 'Updating...' : 'Update'}</span>
                          </button>
                        )}
                        <button
                          className="btn-secondary"
                          disabled={!!op || !isAdmin}
                          onClick={() => handleUninstall(plugin)}
                          title={!isAdmin ? 'Admin role required' : 'Uninstall plugin and remove binaries'}
                          style={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: '0.4rem',
                            fontSize: '0.8rem',
                            padding: '0.4rem 0.8rem',
                            color: 'var(--danger)',
                            borderColor: 'rgba(239, 68, 68, 0.3)'
                          }}
                        >
                          <FaTrash />
                          <span>{op === 'uninstalling' ? 'Removing...' : 'Uninstall'}</span>
                        </button>
                      </>
                    ) : (
                      <button
                        className="btn-primary"
                        disabled={!!op || !isAdmin}
                        onClick={() => handleInstall(plugin)}
                        title={!isAdmin ? 'Admin role required' : `Download and install ${plugin.name}`}
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          gap: '0.45rem',
                          fontSize: '0.82rem',
                          padding: '0.45rem 0.95rem'
                        }}
                      >
                        <FaDownload />
                        <span>{op === 'installing' ? 'Installing...' : 'Install'}</span>
                      </button>
                    )}
                  </div>
                </div>

                {!isAdmin && (
                  <div style={{ fontSize: '0.72rem', color: '#f59e0b', display: 'flex', alignItems: 'center', gap: '0.3rem', marginTop: '-0.4rem' }}>
                    <FaExclamationTriangle /> Admin role required to install or manage store plugins
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
