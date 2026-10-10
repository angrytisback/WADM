import { useState, useEffect, useCallback } from 'react';
import { useToast } from '../context/ToastContext';
import { useAuth } from '../context/AuthContext';
import { useJobs } from '../context/JobContext';
import type { SslStatusResponse, SslMode } from '../types';
import {
    FaShieldAlt,
    FaLock,
    FaCheckCircle,
    FaExclamationTriangle,
    FaSyncAlt,
    FaGlobe,
    FaKey,
    FaFileAlt,
    FaInfoCircle,
    FaServer
} from 'react-icons/fa';

export default function SSLManagement() {
    const [sslData, setSslData] = useState<SslStatusResponse | null>(null);
    const [loading, setLoading] = useState<boolean>(true);
    const [actionLoading, setActionLoading] = useState<boolean>(false);
    const [activeTab, setActiveTab] = useState<SslMode>('lets_encrypt');

    // Form inputs
    const [leDomain, setLeDomain] = useState<string>('');
    const [leEmail, setLeEmail] = useState<string>('');

    const [customDomain, setCustomDomain] = useState<string>('');
    const [customCertPem, setCustomCertPem] = useState<string>('');
    const [customKeyPem, setCustomKeyPem] = useState<string>('');

    const [selfSignedDomain, setSelfSignedDomain] = useState<string>(
        window.location.hostname || 'localhost'
    );

    const { addToast } = useToast();
    const { isAdmin } = useAuth();
    const { openJobModal } = useJobs();

    const fetchSslStatus = useCallback(async () => {
        setLoading(true);
        try {
            const res = await fetch('/api/ssl/config');
            if (res.ok) {
                const data: SslStatusResponse = await res.json();
                setSslData(data);
                if (data.config.domain) {
                    setLeDomain(data.config.domain);
                    setCustomDomain(data.config.domain);
                }
                if (data.config.email) {
                    setLeEmail(data.config.email);
                }
                if (data.config.mode) {
                    setActiveTab(data.config.mode);
                }
            } else {
                addToast('Failed to load SSL configuration', 'error');
            }
        } catch {
            addToast('Error communicating with SSL engine', 'error');
        } finally {
            setLoading(false);
        }
    }, [addToast]);

    useEffect(() => {
        fetchSslStatus();
    }, [fetchSslStatus]);

    const handleToggleSsl = async (enable: boolean, forceHttps: boolean, autoRenew?: boolean) => {
        if (!isAdmin()) {
            addToast('Admin privilege required to modify SSL configuration', 'error');
            return;
        }

        setActionLoading(true);
        try {
            const res = await fetch('/api/ssl/toggle', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    enabled: enable,
                    force_https: forceHttps,
                    auto_renew: autoRenew ?? sslData?.config.auto_renew ?? true
                })
            });

            if (res.ok) {
                const data = await res.json();
                addToast(data.message || 'SSL configuration updated', 'success');
                fetchSslStatus();
            } else {
                const err = await res.json();
                addToast(err.error || 'Failed to update SSL state', 'error');
            }
        } catch {
            addToast('Network error while updating SSL', 'error');
        } finally {
            setActionLoading(false);
        }
    };

    const handleRequestLetsEncrypt = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!isAdmin()) return;

        if (!leDomain.trim() || !leEmail.trim()) {
            addToast('Please enter both domain and email', 'warning');
            return;
        }

        setActionLoading(true);
        try {
            const res = await fetch('/api/ssl/letsencrypt', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    domain: leDomain.trim(),
                    email: leEmail.trim()
                })
            });

            if (res.ok) {
                const data = await res.json();
                addToast("Let's Encrypt job enqueued!", 'info');
                if (data.job_id) {
                    openJobModal(data.job_id, `Let's Encrypt Issue: ${leDomain.trim()}`);
                }
                fetchSslStatus();
            } else {
                const err = await res.json();
                addToast(err.error || "Failed to start Let's Encrypt job", 'error');
            }
        } catch {
            addToast("Failed to request Let's Encrypt certificate", 'error');
        } finally {
            setActionLoading(false);
        }
    };

    const handleUploadCustomCert = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!isAdmin()) return;

        if (!customCertPem.trim() || !customKeyPem.trim()) {
            addToast('Both Certificate and Private Key PEM contents are required', 'warning');
            return;
        }

        setActionLoading(true);
        try {
            const res = await fetch('/api/ssl/custom', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    cert_pem: customCertPem.trim(),
                    key_pem: customKeyPem.trim(),
                    domain: customDomain.trim() || undefined
                })
            });

            if (res.ok) {
                const data = await res.json();
                addToast(data.message || 'Custom certificate applied', 'success');
                setCustomCertPem('');
                setCustomKeyPem('');
                fetchSslStatus();
            } else {
                const err = await res.json();
                addToast(err.error || 'Failed to save custom certificate', 'error');
            }
        } catch {
            addToast('Error uploading custom certificate', 'error');
        } finally {
            setActionLoading(false);
        }
    };

    const handleGenerateSelfSigned = async () => {
        if (!isAdmin()) return;

        setActionLoading(true);
        try {
            const res = await fetch('/api/ssl/self-signed', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    domain: selfSignedDomain.trim() || undefined
                })
            });

            if (res.ok) {
                const data = await res.json();
                addToast(data.message || 'Self-signed certificate generated', 'success');
                fetchSslStatus();
            } else {
                const err = await res.json();
                addToast(err.error || 'Failed to generate self-signed certificate', 'error');
            }
        } catch {
            addToast('Error generating self-signed certificate', 'error');
        } finally {
            setActionLoading(false);
        }
    };

    if (loading) {
        return (
            <div style={{ padding: '2rem', textAlign: 'center', color: 'var(--text-secondary)' }}>
                <FaSyncAlt className="spin" style={{ fontSize: '1.5rem', marginBottom: '0.5rem' }} />
                <div>Loading SSL / TLS configuration...</div>
            </div>
        );
    }

    const config = sslData?.config;
    const certExists = sslData?.cert_exists ?? false;
    const isSslActive = config?.enabled && certExists;

    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '2rem', maxWidth: '1000px', margin: '0 auto' }}>
            {/* Header */}
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', flexWrap: 'wrap', gap: '1rem' }}>
                <div>
                    <h1 style={{ fontSize: '1.8rem', fontWeight: 700, margin: 0, display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                        <FaShieldAlt style={{ color: 'var(--accent-color, #38bdf8)' }} />
                        SSL / TLS Management
                    </h1>
                    <p style={{ color: 'var(--text-secondary)', margin: '0.4rem 0 0 0', fontSize: '0.9rem' }}>
                        Direct Rustls TLS termination and automated Let&apos;s Encrypt ACME engine without external reverse proxies.
                    </p>
                </div>
                <button
                    onClick={fetchSslStatus}
                    disabled={actionLoading}
                    className="btn btn-secondary"
                    style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.5rem 1rem' }}
                >
                    <FaSyncAlt className={actionLoading ? 'spin' : ''} /> Refresh
                </button>
            </div>

            {/* Live Status Card */}
            <div className="glass-panel" style={{ padding: '1.75rem', borderRadius: '16px', border: '1px solid var(--border-color)' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '1.5rem', borderBottom: '1px solid rgba(255,255,255,0.08)', paddingBottom: '1.5rem' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
                        <div style={{
                            width: '48px', height: '48px', borderRadius: '12px',
                            display: 'flex', alignItems: 'center', justifyContent: 'center',
                            background: isSslActive ? 'rgba(16, 185, 129, 0.15)' : 'rgba(239, 68, 68, 0.15)',
                            color: isSslActive ? '#10b981' : '#ef4444',
                            fontSize: '1.5rem'
                        }}>
                            {isSslActive ? <FaLock /> : <FaExclamationTriangle />}
                        </div>
                        <div>
                            <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                                <span style={{ fontSize: '1.2rem', fontWeight: 700 }}>
                                    {isSslActive ? 'HTTPS Active' : 'HTTPS Inactive'}
                                </span>
                                <span style={{
                                    fontSize: '0.75rem', fontWeight: 600, padding: '0.2rem 0.6rem', borderRadius: '20px',
                                    background: isSslActive ? 'rgba(16, 185, 129, 0.2)' : 'rgba(148, 163, 184, 0.2)',
                                    color: isSslActive ? '#10b981' : '#94a3b8',
                                    border: isSslActive ? '1px solid rgba(16, 185, 129, 0.4)' : '1px solid rgba(148, 163, 184, 0.3)'
                                }}>
                                    {config?.mode === 'lets_encrypt' ? "Let's Encrypt" : config?.mode === 'self_signed' ? 'Self-Signed' : config?.mode === 'custom_cert' ? 'Custom Certificate' : 'Disabled'}
                                </span>
                            </div>
                            <div style={{ color: 'var(--text-secondary)', fontSize: '0.85rem', marginTop: '0.25rem' }}>
                                {config?.domain ? `Domain: ${config.domain}` : 'No specific domain configured'} • Port {config?.https_port || 8168}
                            </div>
                        </div>
                    </div>

                    {/* Expiration and Provider Details */}
                    {certExists && (
                        <div style={{ display: 'flex', gap: '2rem', alignItems: 'center' }}>
                            <div>
                                <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                                    Issuer
                                </div>
                                <div style={{ fontWeight: 600, fontSize: '0.95rem' }}>
                                    {config?.issuer || 'Unknown'}
                                </div>
                            </div>
                            <div>
                                <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                                    Validity
                                </div>
                                <div style={{
                                    fontWeight: 600, fontSize: '0.95rem',
                                    color: sslData?.is_expired ? '#ef4444' : (sslData?.days_left ?? 100) < 30 ? '#f59e0b' : '#10b981'
                                }}>
                                    {sslData?.is_expired
                                        ? 'Expired'
                                        : sslData?.days_left != null
                                        ? `${sslData.days_left} days remaining`
                                        : 'Permanent / Custom'}
                                </div>
                            </div>
                        </div>
                    )}
                </div>

                {/* Quick Toggles */}
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(260px, 1fr))', gap: '1.25rem', marginTop: '1.5rem' }}>
                    {/* Enable SSL */}
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '1rem', background: 'rgba(255,255,255,0.03)', borderRadius: '12px' }}>
                        <div>
                            <div style={{ fontWeight: 600, fontSize: '0.95rem' }}>Enable SSL (HTTPS)</div>
                            <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                                {certExists ? 'Serve traffic over TLS' : 'Requires certificate first'}
                            </div>
                        </div>
                        <button
                            onClick={() => handleToggleSsl(!config?.enabled, config?.force_https || false)}
                            disabled={!certExists || actionLoading || !isAdmin()}
                            style={{
                                background: config?.enabled ? '#10b981' : '#374151',
                                color: 'white', border: 'none', padding: '0.4rem 0.9rem',
                                borderRadius: '8px', cursor: (!certExists || actionLoading || !isAdmin()) ? 'not-allowed' : 'pointer',
                                fontWeight: 600, fontSize: '0.85rem'
                            }}
                        >
                            {config?.enabled ? 'Enabled' : 'Disabled'}
                        </button>
                    </div>

                    {/* Force HTTPS */}
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '1rem', background: 'rgba(255,255,255,0.03)', borderRadius: '12px' }}>
                        <div>
                            <div style={{ fontWeight: 600, fontSize: '0.95rem' }}>Force HTTPS Redirect</div>
                            <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                                Redirect port {config?.http_port || 80} to HTTPS
                            </div>
                        </div>
                        <button
                            onClick={() => handleToggleSsl(config?.enabled || false, !config?.force_https)}
                            disabled={!config?.enabled || actionLoading || !isAdmin()}
                            style={{
                                background: config?.force_https ? '#38bdf8' : '#374151',
                                color: 'white', border: 'none', padding: '0.4rem 0.9rem',
                                borderRadius: '8px', cursor: (!config?.enabled || actionLoading || !isAdmin()) ? 'not-allowed' : 'pointer',
                                fontWeight: 600, fontSize: '0.85rem'
                            }}
                        >
                            {config?.force_https ? 'Forced' : 'Off'}
                        </button>
                    </div>

                    {/* Auto-Renew */}
                    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '1rem', background: 'rgba(255,255,255,0.03)', borderRadius: '12px' }}>
                        <div>
                            <div style={{ fontWeight: 600, fontSize: '0.95rem' }}>Daily Auto-Renewal</div>
                            <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary)' }}>
                                Renew when &lt; 30 days remaining
                            </div>
                        </div>
                        <button
                            onClick={() => handleToggleSsl(config?.enabled || false, config?.force_https || false, !config?.auto_renew)}
                            disabled={actionLoading || !isAdmin()}
                            style={{
                                background: config?.auto_renew ? '#6366f1' : '#374151',
                                color: 'white', border: 'none', padding: '0.4rem 0.9rem',
                                borderRadius: '8px', cursor: (actionLoading || !isAdmin()) ? 'not-allowed' : 'pointer',
                                fontWeight: 600, fontSize: '0.85rem'
                            }}
                        >
                            {config?.auto_renew ? 'Active' : 'Off'}
                        </button>
                    </div>
                </div>
            </div>

            {/* Mode Configuration Tabs */}
            <div className="glass-panel" style={{ padding: '2rem', borderRadius: '16px' }}>
                <div style={{ display: 'flex', gap: '0.75rem', borderBottom: '1px solid rgba(255,255,255,0.1)', paddingBottom: '1rem', marginBottom: '1.5rem', flexWrap: 'wrap' }}>
                    {[
                        { id: 'lets_encrypt', label: "Let's Encrypt (Automated)", icon: FaGlobe },
                        { id: 'custom_cert', label: 'Custom Certificate', icon: FaFileAlt },
                        { id: 'self_signed', label: 'Self-Signed (Local)', icon: FaKey },
                    ].map((tab) => (
                        <button
                            key={tab.id}
                            onClick={() => setActiveTab(tab.id as SslMode)}
                            style={{
                                display: 'flex', alignItems: 'center', gap: '0.5rem',
                                padding: '0.6rem 1.2rem', borderRadius: '10px', border: '1px solid var(--border-color)',
                                background: activeTab === tab.id ? 'var(--accent-color, #38bdf8)' : 'rgba(255,255,255,0.05)',
                                color: activeTab === tab.id ? '#0f172a' : 'white',
                                fontWeight: 600, cursor: 'pointer', transition: 'all 0.2s',
                                fontSize: '0.9rem'
                            }}
                        >
                            <tab.icon /> {tab.label}
                        </button>
                    ))}
                </div>

                {/* Tab 1: Let's Encrypt */}
                {activeTab === 'lets_encrypt' && (
                    <form onSubmit={handleRequestLetsEncrypt} style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
                        <div style={{ background: 'rgba(56, 189, 248, 0.08)', padding: '1rem', borderRadius: '10px', border: '1px solid rgba(56, 189, 248, 0.2)', display: 'flex', gap: '0.75rem', alignItems: 'flex-start' }}>
                            <FaInfoCircle style={{ color: '#38bdf8', marginTop: '0.2rem', flexShrink: 0 }} />
                            <div style={{ fontSize: '0.85rem', lineHeight: 1.5 }}>
                                <strong>HTTP-01 Challenge Requirements:</strong> Let&apos;s Encrypt will verify ownership by sending an HTTP request to <code>http://&lt;domain&gt;/.well-known/acme-challenge/*</code> on port 80. Ensure your domain&apos;s DNS A-record points to this server&apos;s public IP address before issuing.
                            </div>
                        </div>

                        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
                            <div>
                                <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 600, marginBottom: '0.5rem' }}>
                                    Domain / Hostname
                                </label>
                                <input
                                    type="text"
                                    placeholder="panel.example.com"
                                    value={leDomain}
                                    onChange={(e) => setLeDomain(e.target.value)}
                                    required
                                    className="input-field"
                                    style={{ width: '100%', padding: '0.65rem 1rem', borderRadius: '8px', border: '1px solid var(--border-color)', background: 'rgba(0,0,0,0.2)', color: 'white' }}
                                />
                            </div>

                            <div>
                                <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 600, marginBottom: '0.5rem' }}>
                                    Contact Email (Expiry Alerts)
                                </label>
                                <input
                                    type="email"
                                    placeholder="admin@example.com"
                                    value={leEmail}
                                    onChange={(e) => setLeEmail(e.target.value)}
                                    required
                                    className="input-field"
                                    style={{ width: '100%', padding: '0.65rem 1rem', borderRadius: '8px', border: '1px solid var(--border-color)', background: 'rgba(0,0,0,0.2)', color: 'white' }}
                                />
                            </div>
                        </div>

                        <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '0.5rem' }}>
                            <button
                                type="submit"
                                disabled={actionLoading || !isAdmin()}
                                className="btn btn-primary"
                                style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.75rem 1.5rem', fontWeight: 600, borderRadius: '8px' }}
                            >
                                <FaCheckCircle /> Issue Let&apos;s Encrypt Certificate
                            </button>
                        </div>
                    </form>
                )}

                {/* Tab 2: Custom Certificate */}
                {activeTab === 'custom_cert' && (
                    <form onSubmit={handleUploadCustomCert} style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
                        <div>
                            <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 600, marginBottom: '0.5rem' }}>
                                Domain / Common Name (Optional Label)
                            </label>
                            <input
                                type="text"
                                placeholder="my-custom-domain.com"
                                value={customDomain}
                                onChange={(e) => setCustomDomain(e.target.value)}
                                className="input-field"
                                style={{ width: '100%', padding: '0.65rem 1rem', borderRadius: '8px', border: '1px solid var(--border-color)', background: 'rgba(0,0,0,0.2)', color: 'white' }}
                            />
                        </div>

                        <div>
                            <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 600, marginBottom: '0.5rem' }}>
                                Certificate Fullchain (PEM Format)
                            </label>
                            <textarea
                                rows={5}
                                placeholder="-----BEGIN CERTIFICATE-----&#10;...&#10;-----END CERTIFICATE-----"
                                value={customCertPem}
                                onChange={(e) => setCustomCertPem(e.target.value)}
                                required
                                style={{ width: '100%', padding: '0.75rem', borderRadius: '8px', border: '1px solid var(--border-color)', background: 'rgba(0,0,0,0.3)', color: 'white', fontFamily: 'monospace', fontSize: '0.8rem' }}
                            />
                        </div>

                        <div>
                            <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 600, marginBottom: '0.5rem' }}>
                                Private Key (PEM Format)
                            </label>
                            <textarea
                                rows={5}
                                placeholder="-----BEGIN PRIVATE KEY-----&#10;...&#10;-----END PRIVATE KEY-----"
                                value={customKeyPem}
                                onChange={(e) => setCustomKeyPem(e.target.value)}
                                required
                                style={{ width: '100%', padding: '0.75rem', borderRadius: '8px', border: '1px solid var(--border-color)', background: 'rgba(0,0,0,0.3)', color: 'white', fontFamily: 'monospace', fontSize: '0.8rem' }}
                            />
                        </div>

                        <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
                            <button
                                type="submit"
                                disabled={actionLoading || !isAdmin()}
                                className="btn btn-primary"
                                style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.75rem 1.5rem', fontWeight: 600, borderRadius: '8px' }}
                            >
                                <FaCheckCircle /> Validate &amp; Apply Certificate
                            </button>
                        </div>
                    </form>
                )}

                {/* Tab 3: Self-Signed */}
                {activeTab === 'self_signed' && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.25rem' }}>
                        <div style={{ background: 'rgba(245, 158, 11, 0.08)', padding: '1rem', borderRadius: '10px', border: '1px solid rgba(245, 158, 11, 0.2)', display: 'flex', gap: '0.75rem', alignItems: 'flex-start' }}>
                            <FaExclamationTriangle style={{ color: '#f59e0b', marginTop: '0.2rem', flexShrink: 0 }} />
                            <div style={{ fontSize: '0.85rem', lineHeight: 1.5 }}>
                                <strong>Browser Notice:</strong> Self-signed certificates provide end-to-end TLS encryption with Rustls, but web browsers will display an initial security warning (e.g. &quot;Your connection is not private&quot;) until the certificate is trusted in the OS or browser keystore.
                            </div>
                        </div>

                        <div>
                            <label style={{ display: 'block', fontSize: '0.85rem', fontWeight: 600, marginBottom: '0.5rem' }}>
                                Hostname / IP (Included in SANs)
                            </label>
                            <input
                                type="text"
                                placeholder="localhost or server IP"
                                value={selfSignedDomain}
                                onChange={(e) => setSelfSignedDomain(e.target.value)}
                                className="input-field"
                                style={{ width: '100%', padding: '0.65rem 1rem', borderRadius: '8px', border: '1px solid var(--border-color)', background: 'rgba(0,0,0,0.2)', color: 'white' }}
                            />
                        </div>

                        <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '0.5rem' }}>
                            <button
                                onClick={handleGenerateSelfSigned}
                                disabled={actionLoading || !isAdmin()}
                                className="btn btn-primary"
                                style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.75rem 1.5rem', fontWeight: 600, borderRadius: '8px' }}
                            >
                                <FaKey /> Generate 10-Year Self-Signed Certificate
                            </button>
                        </div>
                    </div>
                )}
            </div>

            {/* Network & Port Architecture Info */}
            <div className="glass-panel" style={{ padding: '1.5rem', borderRadius: '16px', display: 'flex', gap: '1rem', alignItems: 'flex-start' }}>
                <FaServer style={{ fontSize: '1.4rem', color: 'var(--accent-color, #38bdf8)', marginTop: '0.2rem', flexShrink: 0 }} />
                <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', lineHeight: 1.6 }}>
                    <div style={{ fontWeight: 600, color: 'white', marginBottom: '0.25rem' }}>Standalone Architecture Notice</div>
                    When SSL is enabled, WADM binds pure Rust TLS directly on <code>0.0.0.0:{config?.https_port || 8168}</code> using <code>rustls 0.20</code>. When <strong>Force HTTPS</strong> is active, an auxiliary HTTP listener runs on port <code>80</code> to answer Let&apos;s Encrypt HTTP-01 challenges and 301-redirect all other web requests to HTTPS. If port 80 is occupied or restricted, panel traffic remains safely accessible on the HTTPS port.
                </div>
            </div>
        </div>
    );
}
