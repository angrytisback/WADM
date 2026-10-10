import { useState, useEffect } from 'react';
import Packages from './components/Packages';
import Services from './components/Services';
import Docker from './components/Docker';
import Database from './components/Database';
import SystemInfo from './components/SystemInfo';
import SystemManagement from './components/SystemManagement';
import Firewall from './components/Firewall';
import Login from './components/Login';
import Setup from './components/Setup';
import Dashboard from './components/Dashboard';
import SystemUsage from './components/SystemUsage';
import Settings from './components/Settings';
import Terminal from './components/Terminal';
import Logs from './components/Logs';
import { PluginsManagement } from './components/PluginsManagement';
import { PluginContainer } from './components/PluginContainer';
import type { PluginRuntimeInfo } from './types';
import { DependencyModal } from './components/DependencyModal';
import { DependencyWarning } from './components/DependencyWarning';
import { RootWarningModal } from './components/RootWarningModal';
import { ModalProvider } from './context/ModalContext';
import { JobProvider } from './context/JobContext';
import { JobProgressModal } from './components/JobProgressModal';
import { ToastProvider, useToast } from './context/ToastContext';
import { AuthProvider, useAuth } from './context/AuthContext';
import { DependencyProvider } from './context/DependencyContext';
import { StatsProvider, useStats } from './context/StatsContext';
import { SystemProvider, useSystem } from './context/SystemContext';
import { TerminalProvider } from './context/TerminalContext';
import { ServerStatusProvider } from './context/ServerStatusContext';
import { ServerStatusOverlay } from './components/ServerStatusOverlay';
import { AppStore } from './components/AppStore';
import { FileExplorer } from './components/FileExplorer';
import AuditLogs from './components/AuditLogs';
import UserManagement from './components/UserManagement';
import SSLManagement from './components/SSLManagement';
import { ClusterProvider } from './context/ClusterContext';
import { HeaderNodeSwitcher } from './components/HeaderNodeSwitcher';
import { ClusterOverview } from './components/ClusterOverview';
import {
  FaTachometerAlt, FaChartPie, FaInfoCircle, FaBoxOpen,
  FaCogs, FaShieldAlt, FaDocker, FaDatabase, FaTerminal,
  FaCog, FaSignOutAlt, FaBars, FaTimes, FaExclamationTriangle,
  FaListUl, FaLayerGroup, FaStore, FaFolderOpen, FaPuzzlePiece, FaPlug,
  FaHistory, FaUsers, FaLock, FaNetworkWired
} from 'react-icons/fa';

function MainContent() {
  const { isAuthenticated, isLoading, setupRequired, logout, user: authUser, role, isAdmin } = useAuth();
  const { stats } = useStats();
  const { systemInfo } = useSystem();
  const [activeTab, setActiveTab] = useState('dashboard');
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [plugins, setPlugins] = useState<PluginRuntimeInfo[]>([]);
  const { addToast } = useToast();

  useEffect(() => {
    if (!isAuthenticated) return;
    const fetchPlugins = () => {
      fetch('/api/plugins')
        .then(res => res.json())
        .then(data => {
          if (Array.isArray(data)) {
            setPlugins(data);
          }
        })
        .catch(() => {});
    };
    fetchPlugins();
    const interval = setInterval(fetchPlugins, 5000);
    return () => clearInterval(interval);
  }, [isAuthenticated]);

  const renderContent = () => {
    return (
      <div style={{ position: 'relative', height: '100%' }}>
        <div style={{ 
          display: activeTab === 'terminal' ? 'block' : 'none',
          position: activeTab === 'terminal' ? 'relative' : 'absolute',
          visibility: activeTab === 'terminal' ? 'visible' : 'hidden',
          top: 0, left: 0, right: 0, bottom: 0,
          zIndex: activeTab === 'terminal' ? 1 : -1,
          pointerEvents: activeTab === 'terminal' ? 'auto' : 'none'
        }}>
          <Terminal visible={activeTab === 'terminal'} />
        </div>

        <div style={{ display: activeTab === 'terminal' ? 'none' : 'block' }}>
          {(() => {
            if (activeTab === 'plugins') {
              return <PluginsManagement onSelectPlugin={(id) => setActiveTab(`plugin-${id}`)} />;
            }
            if (activeTab.startsWith('plugin-')) {
              const pluginId = activeTab.replace('plugin-', '');
              return <PluginContainer pluginId={pluginId} onBack={() => setActiveTab('plugins')} />;
            }
            switch (activeTab) {
              case 'cluster': return <ClusterOverview onSelectNodeAndNavigate={() => setActiveTab('dashboard')} />;
              case 'management': return <SystemManagement />;
              case 'info': return <SystemInfo />;
              case 'usage': return <SystemUsage />;
              case 'packages': return <Packages />;
              case 'services': return <Services />;
              case 'firewall': return <Firewall />;
              case 'docker': return <Docker />;
              case 'database': return <Database />;
              case 'logs': return <Logs />;
              case 'audit': return <AuditLogs />;
              case 'users': return isAdmin() ? <UserManagement /> : <div className="glass-panel" style={{ padding: '2rem', textAlign: 'center' }}>Admin access required</div>;
              case 'ssl': return isAdmin() ? <SSLManagement /> : <div className="glass-panel" style={{ padding: '2rem', textAlign: 'center' }}>Admin access required</div>;
              case 'settings': return <Settings />;
              case 'appstore': return <AppStore />;
              case 'files': return <FileExplorer />;
              default: return <Dashboard stats={stats} onNavigate={setActiveTab} />;
            }
          })()}
        </div>
      </div>
    );
  };

  if (isLoading) {
    return (
      <div style={{
        display: 'flex',
        justifyContent: 'center',
        alignItems: 'center',
        height: '100vh',
        width: '100%',
        background: 'var(--bg-dark, #0f172a)',
        color: 'var(--text-secondary, #94a3b8)',
        fontFamily: 'monospace',
      }}>
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '1rem' }}>
          <div style={{
            width: '36px',
            height: '36px',
            border: '3px solid rgba(255,255,255,0.1)',
            borderTopColor: 'var(--accent-color, #3b82f6)',
            borderRadius: '50%',
            animation: 'spin 1s linear infinite'
          }} />
          <span style={{ fontSize: '0.9rem', letterSpacing: '0.05em' }}>Loading WADM session...</span>
        </div>
      </div>
    );
  }

  if (setupRequired) {
    return <Setup />;
  }

  if (!isAuthenticated) {
    return <Login />;
  }

  const canManage = systemInfo?.is_root || systemInfo?.has_sudo;

  interface NavItem {
    id: string;
    label: string;
    icon: React.ComponentType<{ style?: React.CSSProperties }>;
    privileged?: boolean;
    adminOnly?: boolean;
  }

  const renderNavItem = (item: NavItem) => {
    const isPrivilegedDisabled = item.privileged && !canManage;
    const isAdminDisabled = item.adminOnly && !isAdmin();
    const isDisabled = isPrivilegedDisabled || isAdminDisabled;

    const disabledReason = isAdminDisabled
      ? "Admin role required"
      : isPrivilegedDisabled
      ? "Root or Sudo privileges required"
      : "";

    return (
      <div
        key={item.id}
        className={`nav-link ${activeTab === item.id ? 'active' : ''} ${isDisabled ? 'disabled' : ''}`}
        onClick={() => { 
          if (isDisabled) {
            addToast(`This section requires ${disabledReason}`, "warning");
            return;
          }
          setActiveTab(item.id); 
          setMobileMenuOpen(false); 
        }}
        style={{ 
          opacity: isDisabled ? 0.4 : 1,
          cursor: isDisabled ? 'not-allowed' : 'pointer',
          padding: '0.6rem 1rem'
        }}
        title={disabledReason}
      >
        <item.icon style={{ fontSize: '1.1rem', minWidth: '20px' }} />
        <span style={{ fontSize: '0.9rem' }}>{item.label}</span>
        {isAdminDisabled ? (
          <FaLock style={{ marginLeft: 'auto', fontSize: '0.75rem', color: 'var(--text-secondary)' }} />
        ) : isPrivilegedDisabled ? (
          <FaExclamationTriangle style={{ marginLeft: 'auto', fontSize: '0.8rem', color: 'var(--warning)' }} />
        ) : null}
      </div>
    );
  };

  const NAV_ITEMS = [
    { id: 'dashboard', label: 'Dashboard', icon: FaTachometerAlt },
    { id: 'cluster', label: 'Cluster / Nodes', icon: FaNetworkWired, privileged: true },
    { id: 'usage', label: 'System Usage', icon: FaChartPie },
    { id: 'info', label: 'System Info', icon: FaInfoCircle },
    { id: 'management', label: 'System', icon: FaCogs, privileged: true },
    { id: 'packages', label: 'Packages', icon: FaBoxOpen, privileged: true },
    { id: 'services', label: 'Services', icon: FaLayerGroup, privileged: true },
    { id: 'firewall', label: 'Firewall', icon: FaShieldAlt, privileged: true },
    { id: 'docker', label: 'Docker', icon: FaDocker, privileged: true },
    { id: 'database', label: 'Database', icon: FaDatabase, privileged: true },
    { id: 'appstore', label: 'App Store', icon: FaStore, privileged: true },
    { id: 'files', label: 'File Explorer', icon: FaFolderOpen, privileged: true },
    { id: 'terminal', label: 'Terminal', icon: FaTerminal, adminOnly: true },
    { id: 'logs', label: 'Logs', icon: FaListUl },
    { id: 'audit', label: 'Audit Logs', icon: FaHistory },
    { id: 'users', label: 'Users', icon: FaUsers, adminOnly: true },
    { id: 'ssl', label: 'SSL / TLS', icon: FaLock, adminOnly: true },
    { id: 'plugins', label: 'Plugins', icon: FaPuzzlePiece, privileged: true },
    { id: 'settings', label: 'Settings', icon: FaCog },
  ];

  const getHeaderTitle = () => {
    if (activeTab.startsWith('plugin-')) {
      const pId = activeTab.replace('plugin-', '');
      const p = plugins.find(x => x.manifest.id === pId);
      return p ? (p.manifest.ui.title || p.manifest.name) : 'Plugin';
    }
    const item = NAV_ITEMS.find(i => i.id === activeTab);
    if (item) return item.label;
    return activeTab.charAt(0).toUpperCase() + activeTab.slice(1);
  };

  return (
    <div style={{ display: 'flex', minHeight: '100vh', width: '100%', position: 'relative' }}>
      <DependencyModal />
      <DependencyWarning />
      {systemInfo && <RootWarningModal isRoot={systemInfo.is_root} hasSudo={systemInfo.has_sudo} />}
      <JobProgressModal />
      <div
        className={`mobile-overlay ${mobileMenuOpen ? 'open' : ''} `}
        onClick={() => setMobileMenuOpen(false)}
      />

      <aside className={`sidebar ${mobileMenuOpen ? 'open' : ''} `}>
        <div style={{
          display: 'flex', justifyContent: 'space-between', alignItems: 'center',
          paddingBottom: '1.5rem', borderBottom: '1px solid var(--glass-border)', marginBottom: '1rem'
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.8rem' }}>
            <div style={{
              width: '36px', height: '36px',
              borderRadius: '8px', display: 'flex', alignItems: 'center', justifyContent: 'center',
              overflow: 'hidden', boxShadow: '0 0 15px rgba(56, 189, 248, 0.2)', border: '1px solid var(--glass-border)',
              padding: '4px'
            }}>
              <img src="/logo.png" alt="WADM" style={{ width: '100%', height: '100%', objectFit: 'contain' }} />
            </div>
            <h1 style={{ fontSize: '1.5rem', fontWeight: 800, letterSpacing: '-0.02em', background: 'linear-gradient(to right, #fff, #94a3b8)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>WADM</h1>
          </div>
          <button className="mobile-menu-btn" onClick={() => setMobileMenuOpen(false)} style={{ fontSize: '1.2rem' }}>
            <FaTimes />
          </button>
        </div>

        <nav style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: '0.25rem', overflowY: 'auto' }} className="custom-scroll">
          <div style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', fontWeight: 700, padding: '0.5rem 1rem', marginTop: '0.5rem', letterSpacing: '0.05em' }}>MAIN</div>
          {NAV_ITEMS.filter(i => ['dashboard', 'cluster', 'usage', 'info'].includes(i.id)).map(item => renderNavItem(item))}
          
          <div style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', fontWeight: 700, padding: '0.5rem 1rem', marginTop: '0.8rem', letterSpacing: '0.05em' }}>RESOURCES</div>
          {NAV_ITEMS.filter(i => ['management', 'packages', 'services', 'firewall', 'docker', 'database', 'appstore', 'files'].includes(i.id)).map(item => renderNavItem(item))}
          
          <div style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', fontWeight: 700, padding: '0.5rem 1rem', marginTop: '0.8rem', letterSpacing: '0.05em' }}>TOOLS</div>
          {NAV_ITEMS.filter(i => ['terminal', 'logs', 'audit', 'users', 'settings'].includes(i.id)).map(item => renderNavItem(item))}

          <div style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', fontWeight: 700, padding: '0.5rem 1rem', marginTop: '0.8rem', letterSpacing: '0.05em' }}>EXTENSIONS</div>
          {NAV_ITEMS.filter(i => ['plugins'].includes(i.id)).map(item => renderNavItem(item))}
          {plugins.filter(p => p.state === 'running').map(p => (
            <div
              key={`plugin-${p.manifest.id}`}
              className={`nav-link ${activeTab === `plugin-${p.manifest.id}` ? 'active' : ''}`}
              onClick={() => { setActiveTab(`plugin-${p.manifest.id}`); setMobileMenuOpen(false); }}
              style={{ padding: '0.6rem 1rem' }}
              title={`Running plugin: ${p.manifest.name}`}
            >
              <FaPlug style={{ fontSize: '1.1rem', minWidth: '20px', color: 'var(--accent-color)' }} />
              <span style={{ fontSize: '0.9rem', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                {p.manifest.ui.title || p.manifest.name}
              </span>
              <span style={{ marginLeft: 'auto', width: '8px', height: '8px', borderRadius: '50%', background: 'var(--success)' }} />
            </div>
          ))}
        </nav>

        <div style={{ padding: '1rem', borderTop: '1px solid var(--glass-border)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <div style={{ display: 'flex', flexDirection: 'column' }}>
            <span style={{ color: 'var(--text-primary)', fontSize: '0.85rem', fontWeight: 600 }}>v0.96.0</span>
            <span style={{ color: 'var(--text-secondary)', fontSize: '0.75rem' }}>Stable Build</span>
          </div>
          <button className="btn-text danger" onClick={logout} style={{ fontSize: '0.85rem', padding: '0.5rem', display: 'flex', gap: '0.5rem', alignItems: 'center' }}>
            <FaSignOutAlt />
          </button>
        </div>
      </aside>

      <main className="main-content">
        <header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '1rem', flexWrap: 'wrap' }}>
            <button className="mobile-menu-btn" onClick={() => setMobileMenuOpen(true)}>
              <FaBars />
            </button>
            <h2 style={{ fontSize: '1.75rem', fontWeight: 700, margin: 0 }}>{getHeaderTitle()}</h2>
            <HeaderNodeSwitcher onOpenClusterTab={() => setActiveTab('cluster')} />
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
            {/* RBAC Session Badge */}
            {authUser && (
              <div className="glass-panel" style={{
                padding: '0.45rem 0.85rem',
                fontSize: '0.85rem',
                display: 'flex',
                alignItems: 'center',
                gap: '0.5rem',
                borderRadius: '10px'
              }}>
                <span style={{ color: 'var(--text-secondary)' }}>Session:</span>
                <span style={{ fontWeight: 600, color: 'var(--text-primary)' }}>{authUser.username}</span>
                <span style={{
                  padding: '1px 6px',
                  borderRadius: '4px',
                  fontSize: '0.7rem',
                  fontWeight: 700,
                  textTransform: 'uppercase',
                  background: role === 'admin' ? 'rgba(239, 68, 68, 0.2)' : role === 'operator' ? 'rgba(56, 189, 248, 0.2)' : 'rgba(168, 85, 247, 0.2)',
                  color: role === 'admin' ? '#f87171' : role === 'operator' ? '#38bdf8' : '#c084fc',
                  border: `1px solid ${role === 'admin' ? '#f87171' : role === 'operator' ? '#38bdf8' : '#c084fc'}40`
                }}>
                  {role}
                </span>
              </div>
            )}

            {/* Host System User */}
            <div className={`glass-panel ${systemInfo && !systemInfo.is_root ? 'glow-warning' : ''}`} style={{ 
              padding: '0.45rem 0.85rem', 
              fontSize: '0.85rem', 
              display: 'flex', 
              alignItems: 'center',
              transition: 'all 0.3s ease',
              borderRadius: '10px'
            }}>
              {systemInfo ? (
                <span style={{ display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                  {!systemInfo.is_root && (
                    <span title="No Root Access" style={{ color: 'var(--warning)', display: 'flex', alignItems: 'center' }}>
                      <FaExclamationTriangle />
                    </span>
                  )}
                  <span style={{ color: 'var(--text-secondary)' }}>Host:</span>
                  <span style={{ 
                    fontWeight: 600, 
                    color: systemInfo.is_root ? 'var(--text-primary)' : '#000',
                    background: systemInfo.is_root ? 'transparent' : 'var(--warning)',
                    padding: systemInfo.is_root ? '0' : '0.2rem 0.5rem',
                    borderRadius: systemInfo.is_root ? '4px' : '4px',
                    marginLeft: systemInfo.is_root ? '0' : '0.2rem'
                  }}>{systemInfo.username}</span>
                </span>
              ) : 'Loading...'}
            </div>
          </div>
        </header>

        <div style={{ flex: 1, minHeight: 0 }}>
          {renderContent()}
        </div>

      </main>
    </div>
  );
}

function App() {
  return (
    <ToastProvider>
      <ModalProvider>
        <AuthProvider>
          <ServerStatusProvider>
            <DependencyProvider>
              <ClusterProvider>
                <SystemProvider>
                  <StatsProvider>
                    <TerminalProvider>
                      <JobProvider>
                        <ServerStatusOverlay />
                        <MainContent />
                      </JobProvider>
                    </TerminalProvider>
                  </StatsProvider>
                </SystemProvider>
              </ClusterProvider>
            </DependencyProvider>
          </ServerStatusProvider>
        </AuthProvider>
      </ModalProvider>
    </ToastProvider>
  );
}

export default App;
