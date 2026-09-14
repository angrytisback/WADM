import { useState, useEffect, useRef } from 'react';
import { useAuth } from '../context/AuthContext';
import { useToast } from '../context/ToastContext';
import { FaRocket, FaCheckCircle, FaLock, FaShieldAlt, FaTools, FaCheck } from 'react-icons/fa';

interface Dependency {
    name: string;
    command: string;
    installed: boolean;
    optional: boolean;
    install_hint?: string;
}

interface DependencyReport {
    dependencies: Dependency[];
    critical_missing: boolean;
}

export default function Setup() {
    const [step, setStep] = useState(1);
    const [password, setPassword] = useState('');
    const [code, setCode] = useState('');
    const [qr, setQr] = useState('');
    const [secret, setSecret] = useState('');
    const { login } = useAuth();
    const { addToast } = useToast();
    const [loading, setLoading] = useState(false);
    const [tempToken, setTempToken] = useState<string | null>(null);

    // Step 3 States
    const [dependencies, setDependencies] = useState<Dependency[]>([]);
    const [criticalMissing, setCriticalMissing] = useState(false);
    const [installing, setInstalling] = useState<string | null>(null);
    const [logs, setLogs] = useState<string[]>([]);
    const logsEndRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        fetch('/api/auth/setup/init', { method: 'POST' })
            .then(res => res.json())
            .then(data => {
                setQr(data.qr);
                setSecret(data.secret);
            })
            .catch(err => console.error(err));
    }, []);

    useEffect(() => {
        if (logsEndRef.current) {
            logsEndRef.current.scrollIntoView({ behavior: 'smooth' });
        }
    }, [logs]);

    // Fetch dependencies when entering step 3
    useEffect(() => {
        if (step === 3 && tempToken) {
            fetchDependencies();
        }
    }, [step, tempToken]);

    const fetchDependencies = async () => {
        if (!tempToken) return;
        try {
            const res = await fetch('/api/system/dependencies', {
                headers: { 'Authorization': `Bearer ${tempToken}` }
            });
            if (res.ok) {
                const data: DependencyReport = await res.json();
                setDependencies(data.dependencies);
                setCriticalMissing(data.critical_missing);
            }
        } catch (e) {
            console.error(e);
            addToast("Failed to fetch dependencies", "error");
        }
    };

    const handleConfirm2FA = async (e: React.FormEvent) => {
        e.preventDefault();
        setLoading(true);

        try {
            const res = await fetch('/api/auth/setup/confirm', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ password, code, secret })
            });

            if (res.ok) {
                const data = await res.json();
                setTempToken(data.token);
                addToast("2FA Verified! checking dependencies...", "success");
                setStep(3);
            } else {
                addToast("Invalid 2FA Code", "error");
            }
        } catch {
            addToast("Setup initiation failed", "error");
        } finally {
            setLoading(false);
        }
    };

    const handleInstall = async (depName: string) => {
        if (!tempToken) return;
        setInstalling(depName);
        setLogs(prev => [...prev, `> Installing ${depName}...`]);

        try {
            const res = await fetch('/api/system/dependencies/install', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                    'Authorization': `Bearer ${tempToken}`
                },
                body: JSON.stringify({ name: depName })
            });

            const text = await res.json();
            setLogs(prev => [...prev, text]);

            if (res.ok) {
                addToast(`${depName} installed successfully`, "success");
                await fetchDependencies(); // Refresh list
            } else {
                addToast(`Failed to install ${depName}`, "error");
            }
        } catch (e) {
            setLogs(prev => [...prev, `Error: ${e}`]);
        } finally {
            setInstalling(null);
        }
    };

    const finishSetup = () => {
        if (tempToken) {
            login(tempToken);
            addToast("Setup complete! Logged in.", "success");
        }
    };

    return (
        <div style={{
            display: 'flex',
            justifyContent: 'center',
            alignItems: 'center',
            height: '100vh',
            width: '100%',
            flex: 1,
            background: 'var(--bg-color)'
        }}>
            <div className="glass-panel" style={{ width: '600px', padding: '3rem', maxHeight: '90vh', overflowY: 'auto' }}>
                <div style={{ textAlign: 'center', marginBottom: '2rem' }}>
                    <h2 style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '0.5rem', marginBottom: '0.5rem' }}>
                        <FaRocket style={{ color: 'var(--accent-color)' }} />
                        Welcome to WADM
                    </h2>
                    <p style={{ color: 'var(--text-secondary)' }}>
                        Follow the steps below to initialize your admin panel.
                    </p>
                </div>

                {/* Stepper Header */}
                <div style={{ display: 'flex', alignItems: 'flex-start', marginBottom: '2.5rem', padding: '0 0.5rem' }}>
                    {[
                        { num: 1, label: 'Password', icon: FaLock },
                        { num: 2, label: '2FA', icon: FaShieldAlt },
                        { num: 3, label: 'Dependencies', icon: FaTools }
                    ].map((s, index, arr) => (
                        <div key={s.num} style={{ display: 'flex', flex: index === arr.length - 1 ? '0 0 auto' : '1 1 auto', alignItems: 'flex-start' }}>
                            <div style={{
                                display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '0.5rem',
                                minWidth: '80px', // Ensure consistent width for labels
                                opacity: step >= s.num ? 1 : 0.5,
                                color: step >= s.num ? 'var(--text-primary)' : 'var(--text-secondary)'
                            }}>
                                <div style={{
                                    width: '40px', height: '40px', borderRadius: '50%',
                                    background: step >= s.num ? 'var(--accent-color)' : 'var(--glass-bg)',
                                    border: `2px solid ${step >= s.num ? 'var(--accent-color)' : 'var(--glass-border)'}`,
                                    color: step >= s.num ? '#fff' : 'var(--text-secondary)',
                                    display: 'flex', alignItems: 'center', justifyContent: 'center',
                                    fontSize: '1rem', transition: 'all 0.3s ease'
                                }}>
                                    {step > s.num ? <FaCheck /> : <s.icon />}
                                </div>
                                <span style={{ fontSize: '0.85rem', fontWeight: 500 }}>{s.label}</span>
                            </div>

                            {/* Spacer Line */}
                            {index < arr.length - 1 && (
                                <div style={{
                                    flex: 1,
                                    height: '2px',
                                    background: 'var(--glass-border)',
                                    marginTop: '19px', // Center with 40px circle (20px - 1px)
                                    marginLeft: '0.5rem',
                                    marginRight: '0.5rem'
                                }} />
                            )}
                        </div>
                    ))}
                </div>

                {step === 1 && (
                    <form onSubmit={(e) => { e.preventDefault(); if (password) setStep(2); }} style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                        <p style={{ textAlign: 'center', color: 'var(--text-secondary)' }}>
                            Secure your admin account with a strong password.
                        </p>
                        <div>
                            <label style={{ display: 'block', marginBottom: '0.5rem', color: 'var(--text-secondary)' }}>Set Admin Password</label>
                            <div style={{ position: 'relative' }}>
                                <input
                                    type="password"
                                    className="input-field"
                                    value={password}
                                    onChange={e => setPassword(e.target.value)}
                                    required
                                    placeholder="Enter strong password"
                                    style={{ paddingRight: '2.5rem' }}
                                />
                                {/* Optional: Add our own lock icon if desired, or just keep padding for browser icon */}
                            </div>
                        </div>
                        <button type="submit" className="btn-primary" style={{ width: '100%' }}>Next: Setup 2FA</button>
                    </form>
                )}

                {step === 2 && (
                    <form onSubmit={handleConfirm2FA} style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                        <div style={{ textAlign: 'center' }}>
                            <p style={{ marginBottom: '1rem' }}>Scan this QR Code with your Authenticator App</p>
                            {qr ? (
                                <>
                                    <div style={{ background: 'white', padding: '10px', borderRadius: '12px', display: 'inline-block', marginBottom: '1rem' }}>
                                        <img src={`data:image/png;base64,${qr}`} alt="QR Code" style={{ display: 'block' }} />
                                    </div>
                                    <div style={{ fontFamily: 'monospace', background: 'rgba(0,0,0,0.3)', padding: '0.5rem', borderRadius: '4px', fontSize: '0.9rem', color: '#fbbf24' }}>
                                        {secret}
                                    </div>
                                </>
                            ) : <div>Loading QR...</div>}
                        </div>

                        <div>
                            <label style={{ display: 'block', marginBottom: '0.5rem', color: 'var(--text-secondary)' }}>Enter 2FA Code</label>
                            <input
                                type="text"
                                className="input-field"
                                value={code}
                                onChange={e => {
                                    const val = e.target.value.replace(/[^0-9]/g, '');
                                    if (val.length <= 6) setCode(val);
                                }}
                                placeholder="000 000"
                                required
                                maxLength={6}
                                style={{ textAlign: 'center', letterSpacing: '0.2em', fontSize: '1.2rem' }}
                            />
                        </div>
                        <div style={{ display: 'flex', gap: '1rem' }}>
                            <button type="button" className="btn-text" onClick={() => setStep(1)} disabled={loading}>Back</button>
                            <button type="submit" className="btn-primary" disabled={loading} style={{ flex: 1 }}>
                                {loading ? 'Verifying...' : 'Verify & Continue'}
                            </button>
                        </div>
                    </form>
                )}

                {step === 3 && (
                    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                        <p style={{ textAlign: 'center', color: 'var(--text-secondary)' }}>
                            Installing necessary system dependencies.
                        </p>

                        <div style={{ display: 'flex', flexDirection: 'column', gap: '0.8rem' }}>
                            {dependencies.map((dep, i) => (
                                <div key={i} style={{
                                    display: 'flex', justifyContent: 'space-between', alignItems: 'center',
                                    padding: '0.8rem', background: 'var(--glass-bg)', borderRadius: '8px',
                                    border: '1px solid var(--glass-border)'
                                }}>
                                    <div style={{ display: 'flex', flexDirection: 'column' }}>
                                        <div style={{ fontWeight: 'bold' }}>{dep.name}</div>
                                        <div style={{ fontSize: '0.8rem', color: dep.name === "Package Manager" ? 'var(--danger)' : 'var(--text-secondary)' }}>
                                            {dep.name === "Package Manager" ? "Critical System Component" : "Optional Component"}
                                        </div>
                                    </div>
                                    {dep.installed ? (
                                        <span style={{ color: 'var(--success)', fontSize: '0.9rem', display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                                            <FaCheckCircle /> Installed
                                        </span>
                                    ) : (
                                        <button
                                            className="btn-primary"
                                            style={{ fontSize: '0.8rem', padding: '0.4rem 1rem', minWidth: '90px' }}
                                            onClick={() => handleInstall(dep.name)}
                                            disabled={installing !== null}
                                        >
                                            {installing === dep.name ? '...' : 'Install'}
                                        </button>
                                    )}
                                </div>
                            ))}
                        </div>

                        {logs.length > 0 && (
                            <div style={{
                                background: '#1e1e1e', color: '#d4d4d4', padding: '1rem', borderRadius: '8px',
                                fontFamily: 'monospace', fontSize: '0.85rem', height: '150px', overflowY: 'auto',
                                border: '1px solid var(--glass-border)'
                            }}>
                                {logs.map((log, i) => (
                                    <div key={i} style={{ whiteSpace: 'pre-wrap', marginBottom: '0.5rem' }}>{log}</div>
                                ))}
                                <div ref={logsEndRef} />
                            </div>
                        )}

                        <button
                            className="btn-primary"
                            onClick={finishSetup}
                            disabled={criticalMissing || installing !== null}
                            style={{ width: '100%', marginTop: '1rem' }}
                        >
                            {criticalMissing ? 'Please install critical dependencies first' : 'Complete Setup'}
                        </button>
                    </div>
                )}
            </div>
        </div >
    );
}
