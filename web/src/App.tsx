import { useState } from 'react';
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
import { DependencyModal } from './components/DependencyModal';
import { DependencyWarning } from './components/DependencyWarning';
import { RootWarningModal } from './components/RootWarningModal';
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
import {
  FaTachometerAlt, FaChartPie, FaInfoCircle, FaBoxOpen,
  FaCogs, FaShieldAlt, FaDocker, FaDatabase, FaTerminal,
  FaCog, FaSignOutAlt, FaBars, FaTimes, FaExclamationTriangle,
  FaListUl, FaLayerGroup, FaStore, FaFolderOpen
} from 'react-icons/fa';

function MainContent() {
  const { isAuthenticated, setupRequired, logout } = useAuth();
  const { stats } = useStats();
  const { systemInfo } = useSystem();
  const [activeTab, setActiveTab] = useState('dashboard');
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const { addToast } = useToast();

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
            switch (activeTab) {
              case 'management': return <SystemManagement />;
              case 'info': return <SystemInfo />;
              case 'usage': return <SystemUsage />;
              case 'packages': return <Packages />;
              case 'services': return <Services />;
              case 'firewall': return <Firewall />;
              case 'docker': return <Docker />;
              case 'database': return <Database />;
              case 'logs': return <Logs />;
              case 'settings': return <Settings />;
              case 'appstore': return <AppStore />;
              case 'files': return <FileExplorer />;
              default: return <Dashboard stats={stats} onNavigate={setActiveTab} />;
            }
          })()}
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

  const renderNavItem = (item: any) => {
    const isDisabled = item.privileged && !canManage;
    return (
      <div
        key={item.id}
        className={`nav-link ${activeTab === item.id ? 'active' : ''} ${isDisabled ? 'disabled' : ''}`}
        onClick={() => { 
          if (isDisabled) {
            addToast("This section requires Root or Sudo privileges", "warning");
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
        title={isDisabled ? "Root or Sudo privileges required" : ""}
      >
        <item.icon style={{ fontSize: '1.1rem', minWidth: '20px' }} />
        <span style={{ fontSize: '0.9rem' }}>{item.label}</span>
        {isDisabled && <FaExclamationTriangle style={{ marginLeft: 'auto', fontSize: '0.8rem', color: 'var(--warning)' }} />}
      </div>
    );
  };

  const NAV_ITEMS = [
    { id: 'dashboard', label: 'Dashboard', icon: FaTachometerAlt },
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
    { id: 'terminal', label: 'Terminal', icon: FaTerminal },
    { id: 'logs', label: 'Logs', icon: FaListUl },
    { id: 'settings', label: 'Settings', icon: FaCog },
  ];

  const getHeaderTitle = () => {
    const item = NAV_ITEMS.find(i => i.id === activeTab);
    if (item) return item.label;
    return activeTab.charAt(0).toUpperCase() + activeTab.slice(1);
  }

  return (
    <div style={{ display: 'flex', minHeight: '100vh', width: '100%', position: 'relative' }}>
      <DependencyModal />
      <DependencyWarning />
      {systemInfo && <RootWarningModal isRoot={systemInfo.is_root} hasSudo={systemInfo.has_sudo} />}
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
          {NAV_ITEMS.filter(i => ['dashboard', 'usage', 'info'].includes(i.id)).map(item => renderNavItem(item))}
          
          <div style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', fontWeight: 700, padding: '0.5rem 1rem', marginTop: '0.8rem', letterSpacing: '0.05em' }}>RESOURCES</div>
          {NAV_ITEMS.filter(i => ['management', 'packages', 'services', 'firewall', 'docker', 'database', 'appstore', 'files'].includes(i.id)).map(item => renderNavItem(item))}
          
          <div style={{ fontSize: '0.7rem', color: 'var(--text-secondary)', fontWeight: 700, padding: '0.5rem 1rem', marginTop: '0.8rem', letterSpacing: '0.05em' }}>TOOLS</div>
          {NAV_ITEMS.filter(i => ['terminal', 'logs', 'settings'].includes(i.id)).map(item => renderNavItem(item))}
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
          <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
            <button className="mobile-menu-btn" onClick={() => setMobileMenuOpen(true)}>
              <FaBars />
            </button>
            <h2 style={{ fontSize: '1.75rem', fontWeight: 700 }}>{getHeaderTitle()}</h2>
          </div>
          <div className={`glass-panel ${systemInfo && !systemInfo.is_root ? 'glow-warning' : ''}`} style={{ 
            padding: '0.5rem 1rem', 
            fontSize: '0.9rem', 
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
                <span style={{ color: 'var(--text-secondary)' }}>User:</span>
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
        </header>

        <div style={{ flex: 1, minHeight: 0 }}>
          {renderContent()}
        </div>

      </main>
    </div>
  );
}

import { ModalProvider } from './context/ModalContext';

function App() {
  return (
    <ToastProvider>
      <ModalProvider>
        <AuthProvider>
          <ServerStatusProvider>
            <DependencyProvider>
              <SystemProvider>
                <StatsProvider>
                  <TerminalProvider>
                    <ServerStatusOverlay />
                    <MainContent />
                  </TerminalProvider>
                </StatsProvider>
              </SystemProvider>
            </DependencyProvider>
          </ServerStatusProvider>
        </AuthProvider>
      </ModalProvider>
    </ToastProvider>
  );
}

export default App;
