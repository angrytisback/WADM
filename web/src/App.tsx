import { useState, useEffect } from 'react';
import Packages from './components/Packages';
import Services from './components/Services';
import Docker from './components/Docker';
import Database from './components/Database';
import SystemInfo from './components/SystemInfo';
import Firewall from './components/Firewall';
import Login from './components/Login';
import Setup from './components/Setup';
import Dashboard from './components/Dashboard';
import SystemUsage from './components/SystemUsage';
import Settings from './components/Settings';
import Terminal from './components/Terminal';
import { ToastProvider, useToast } from './context/ToastContext';
import { AuthProvider, useAuth } from './context/AuthContext';
import type { SystemStats } from './types';



function MainApp() {
  const { isAuthenticated, setupRequired, logout } = useAuth();
  const [activeTab, setActiveTab] = useState('dashboard');
  const [stats, setStats] = useState<SystemStats | null>(null);
  const [systemInfo, setSystemInfo] = useState<any>(null);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const { addToast } = useToast();

  useEffect(() => {
    // Polling only when on dashboard and authenticated
    if (activeTab !== 'dashboard' || !isAuthenticated) return;

    const fetchStats = async () => {
      try {
        const token = localStorage.getItem('wadm_token');
        const headers: HeadersInit = {};
        if (token) {
          headers['Authorization'] = `Bearer ${token}`;
        }

        const response = await fetch('/api/stats', { headers });
        if (response.ok) {
          const data = await response.json();
          setStats(data);
        } else {
          console.error("Failed to fetch stats status:", response.status);
          if (response.status === 401) logout();
        }
      } catch (error) {
        console.error("Failed to fetch stats:", error);
      }
    };

    // Also fetch system info once to get user/sudo status
    if (!systemInfo) {
      const token = localStorage.getItem('wadm_token');
      const headers: HeadersInit = {};
      if (token) headers['Authorization'] = `Bearer ${token}`;

      fetch('/api/system', { headers })
        .then(res => {
          if (res.ok) return res.json();
          throw new Error('Failed to fetch system info');
        })
        .then(data => {
          setSystemInfo(data);
          if (data.has_sudo === false) {
            // Short delay to ensure toast provider is ready if needed
            setTimeout(() => {
              addToast("Warning: WADM does not have sudo privileges. Service management will fail.", "warning");
            }, 500);
          }
        })
        .catch(err => console.error("Failed to fetch system info:", err));
    }

    fetchStats();
    const interval = setInterval(fetchStats, 2000);
    return () => clearInterval(interval);
  }, [activeTab, isAuthenticated, logout, systemInfo, addToast]);



  const renderContent = () => {
    switch (activeTab) {
      case 'system': return <SystemInfo />;
      case 'usage': return <SystemUsage />;
      case 'packages': return <Packages />;
      case 'services': return <Services />;
      case 'firewall': return <Firewall />;
      case 'docker': return <Docker />;

      case 'database': return <Database />;
      case 'terminal': return <Terminal />;
      case 'settings': return <Settings />;

      default: return <Dashboard stats={stats} onNavigate={setActiveTab} />;
    }
  }

  if (setupRequired) {
    return <Setup />;
  }

  if (!isAuthenticated) {
    return <Login />;
  }

  return (
    <div style={{ display: 'flex', minHeight: '100vh', width: '100%', position: 'relative' }}>
      {/* Mobile Backdrop */}
      <div
        className={`mobile-overlay ${mobileMenuOpen ? 'open' : ''}`}
        onClick={() => setMobileMenuOpen(false)}
      />

      <aside className={`sidebar ${mobileMenuOpen ? 'open' : ''}`}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <h1>WADM</h1>
          <button className="mobile-menu-btn" onClick={() => setMobileMenuOpen(false)} style={{ fontSize: '1.2rem' }}>×</button>
        </div>
        <nav style={{ flex: 1 }}>
          <div className={`nav-link ${activeTab === 'dashboard' ? 'active' : ''}`} onClick={() => { setActiveTab('dashboard'); setMobileMenuOpen(false); }}>
            Dashboard
          </div>
          <div className={`nav-link ${activeTab === 'usage' ? 'active' : ''}`} onClick={() => { setActiveTab('usage'); setMobileMenuOpen(false); }}>
            System Usage
          </div>
          <div className={`nav-link ${activeTab === 'system' ? 'active' : ''}`} onClick={() => { setActiveTab('system'); setMobileMenuOpen(false); }}>
            System Info
          </div>
          <div className={`nav-link ${activeTab === 'packages' ? 'active' : ''}`} onClick={() => { setActiveTab('packages'); setMobileMenuOpen(false); }}>
            Packages
          </div>
          <div className={`nav-link ${activeTab === 'services' ? 'active' : ''}`} onClick={() => { setActiveTab('services'); setMobileMenuOpen(false); }}>
            Services
          </div>
          <div className={`nav-link ${activeTab === 'firewall' ? 'active' : ''}`} onClick={() => { setActiveTab('firewall'); setMobileMenuOpen(false); }}>
            Firewall
          </div>
          <div className={`nav-link ${activeTab === 'docker' ? 'active' : ''}`} onClick={() => { setActiveTab('docker'); setMobileMenuOpen(false); }}>
            Docker
          </div>
          <div className={`nav-link ${activeTab === 'database' ? 'active' : ''}`} onClick={() => { setActiveTab('database'); setMobileMenuOpen(false); }}>
            Database
          </div>
          <div className={`nav-link ${activeTab === 'terminal' ? 'active' : ''}`} onClick={() => { setActiveTab('terminal'); setMobileMenuOpen(false); }}>
            Terminal
          </div>
          <div className={`nav-link ${activeTab === 'settings' ? 'active' : ''}`} onClick={() => { setActiveTab('settings'); setMobileMenuOpen(false); }}>
            Settings
          </div>
        </nav>
        <div style={{ padding: '1rem', borderTop: '1px solid var(--glass-border)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
          <span style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>v0.96.0</span>
          <button className="btn-text danger" onClick={logout} style={{ fontSize: '0.8rem' }}>Logout</button>
        </div>
      </aside>

      <main className="main-content">
        <header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
            <button className="mobile-menu-btn" onClick={() => setMobileMenuOpen(true)}>
              ☰
            </button>
            <h2>{activeTab === 'system' ? 'System Information' : (activeTab.charAt(0).toUpperCase() + activeTab.slice(1))}</h2>
          </div>
          <div className="glass-panel" style={{ padding: '0.4rem 1rem', fontSize: '0.9rem' }}>
            {systemInfo ? (
              <span style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                {!systemInfo.has_sudo && <span title="No Sudo Access">⚠️</span>}
                {systemInfo.username}
              </span>
            ) : 'Root User'}
          </div>
        </header>

        {renderContent()}

      </main>
    </div>
  );
}

function App() {
  return (
    <ToastProvider>
      <AuthProvider>
        <MainApp />
      </AuthProvider>
    </ToastProvider>
  );
}

export default App;
