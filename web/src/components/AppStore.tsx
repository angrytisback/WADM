import React, { useEffect, useState } from 'react';
import { FaPlay, FaSpinner } from 'react-icons/fa';
import { useToast } from '../context/ToastContext';

interface AppTemplate {
  id: string;
  name: string;
  description: string;
  icon: string;
}

export const AppStore: React.FC = () => {
  const [apps, setApps] = useState<AppTemplate[]>([]);
  const [loading, setLoading] = useState(true);
  const [installing, setInstalling] = useState<Record<string, boolean>>({});
  const { addToast } = useToast();

  useEffect(() => {
    fetchApps();
  }, []);

  const fetchApps = async () => {
    try {
      const res = await fetch('/api/apps');
      if (!res.ok) throw new Error('Failed to fetch apps');
      const data = await res.json();
      setApps(data);
    } catch {
      addToast('Failed to fetch apps', 'error');
    } finally {
      setLoading(false);
    }
  };

  const installApp = async (id: string, name: string) => {
    if (!window.confirm(`Are you sure you want to install ${name}?`)) return;
    
    setInstalling((prev) => ({ ...prev, [id]: true }));
    try {
      const res = await fetch('/api/apps/install', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ id })
      });
      const data = await res.json();
      if (res.ok) {
        addToast(typeof data === 'string' ? data : `${name} installation started`, 'success');
      } else {
        addToast(`Failed to install ${name}`, 'error');
      }
    } catch {
      addToast(`Failed to install ${name}`, 'error');
    } finally {
      setInstalling((prev) => ({ ...prev, [id]: false }));
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem', padding: '0 1rem' }} className="fade-in">
      <div>
        <h2 style={{ fontSize: '1.75rem', fontWeight: 700, margin: 0, color: 'var(--text-primary)' }}>App Store</h2>
        <p style={{ color: 'var(--text-secondary)', fontSize: '0.95rem', marginTop: '0.25rem' }}>
          One-click install popular Dockerized applications on your server.
        </p>
      </div>

      {loading ? (
        <div style={{ display: 'flex', justifyContent: 'center', padding: '3rem' }}>
          <FaSpinner className="animate-spin" style={{ fontSize: '2rem', color: 'var(--accent-color)' }} />
        </div>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(280px, 1fr))', gap: '1.5rem' }}>
          {apps.map((app) => (
            <div key={app.id} className="glass-panel" style={{ padding: '1.5rem', display: 'flex', flexDirection: 'column', alignItems: 'center', textAlign: 'center' }}>
              <img src={app.icon} alt={app.name} style={{ width: '64px', height: '64px', marginBottom: '1rem', objectFit: 'contain' }} />
              <h3 style={{ fontSize: '1.2rem', fontWeight: 700, color: 'var(--text-primary)', marginBottom: '0.5rem' }}>{app.name}</h3>
              <p style={{ fontSize: '0.9rem', color: 'var(--text-secondary)', marginBottom: '1.5rem', flexGrow: 1 }}>{app.description}</p>
              <button
                onClick={() => installApp(app.id, app.name)}
                disabled={installing[app.id]}
                className="btn primary"
                style={{
                  width: '100%',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: '0.5rem',
                  padding: '0.6rem 1rem'
                }}
              >
                {installing[app.id] ? (
                  <>
                    <FaSpinner className="animate-spin" />
                    <span>Installing...</span>
                  </>
                ) : (
                  <>
                    <FaPlay style={{ fontSize: '0.8rem' }} />
                    <span>Install</span>
                  </>
                )}
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
