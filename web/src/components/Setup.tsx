import { useState, useEffect } from 'react';
import { useAuth } from '../context/AuthContext';
import { useToast } from '../context/ToastContext';

export default function Setup() {
    const [step, setStep] = useState(1);
    const [password, setPassword] = useState('');
    const [code, setCode] = useState('');
    const [qr, setQr] = useState('');
    const [secret, setSecret] = useState('');
    const { login } = useAuth();
    const { addToast } = useToast();
    const [loading, setLoading] = useState(false);

    useEffect(() => {

        fetch('/api/auth/setup/init', { method: 'POST' })
            .then(res => res.json())
            .then(data => {
                setQr(data.qr);
                setSecret(data.secret);
            })
            .catch(err => console.error(err));
    }, []);

    const handleConfirm = async (e: React.FormEvent) => {
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
                login(data.token);
                addToast("Setup complete! Logged in.", "success");
            } else {

                addToast("Setup initiated! Please scan the QR code.", "success");



            }
        } catch {
            addToast("Setup initiation failed", "error");
        } finally {
            setLoading(false);
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
            <div className="glass-panel" style={{ width: '500px', padding: '3rem' }}>
                <h2 style={{ textAlign: 'center', marginBottom: '1rem' }}>🎉 Welcome to WADM</h2>
                <p style={{ textAlign: 'center', color: 'var(--text-secondary)', marginBottom: '2rem' }}>
                    Let's set up your Admin account.
                </p>

                {step === 1 && (
                    <form onSubmit={(e) => { e.preventDefault(); if (password) setStep(2); }} style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                        <div>
                            <label style={{ display: 'block', marginBottom: '0.5rem', color: 'var(--text-secondary)' }}>Set Admin Password</label>
                            <input
                                type="password"
                                className="input-field"
                                value={password}
                                onChange={e => setPassword(e.target.value)}
                                required
                                placeholder="Enter strong password"
                            />
                        </div>
                        <button type="submit" className="btn-primary" style={{ width: '100%' }}>Next</button>
                    </form>
                )}

                {step === 2 && (
                    <form onSubmit={handleConfirm} style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                        <div style={{ textAlign: 'center' }}>
                            <p style={{ marginBottom: '1rem' }}>Scan this QR Code with your Authenticator App</p>
                            {qr ? (
                                <>
                                    <img src={`data:image/png;base64,${qr}`} alt="QR Code" style={{ borderRadius: '12px', marginBottom: '1rem', border: '5px solid white' }} />
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
                                {loading ? 'Finalizing...' : 'Complete Setup'}
                            </button>
                        </div>
                    </form>
                )}
            </div>
        </div >
    );
}
