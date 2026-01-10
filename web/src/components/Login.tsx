import { useState, useEffect } from 'react';
import { useAuth } from '../context/AuthContext';
import { useToast } from '../context/ToastContext';

export default function Login() {
    const [password, setPassword] = useState('');
    const [code, setCode] = useState('');
    const { login } = useAuth();
    const { addToast } = useToast();
    const [loading, setLoading] = useState(false);



    useEffect(() => {
        
        
    }, []);

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setLoading(true);

        try {
            const res = await fetch('/api/auth/login', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ password, code })
            });

            if (res.ok) {
                const data = await res.json();
                login(data.token);
                addToast("Login successful", "success");
            } else {
                const err = await res.json();
                addToast(typeof err === 'string' ? err : 'Login failed', "error");
            }
        } catch {
            addToast("Login error", "error"); 
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
            background: 'var(--bg-dark)'
        }}>
            <div className="glass-panel" style={{ width: '100%', maxWidth: '400px', padding: '3rem' }}>
                <h2 style={{ textAlign: 'center', marginBottom: '2rem', fontSize: '1.8rem' }}>Welcome Back</h2>
                <form onSubmit={handleSubmit} style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
                    <div>
                        <label style={{ display: 'block', marginBottom: '0.5rem', color: 'var(--text-secondary)', fontSize: '0.9rem' }}>Password</label>
                        <input
                            type="password"
                            className="input-field"
                            value={password}
                            onChange={e => setPassword(e.target.value)}
                            required
                            placeholder="••••••••"
                        />
                    </div>
                    <div>
                        <label style={{ display: 'block', marginBottom: '0.5rem', color: 'var(--text-secondary)', fontSize: '0.9rem' }}>Authenticator Code</label>
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

                    <button type="submit" className="btn-primary" disabled={loading} style={{ marginTop: '1rem', width: '100%' }}>
                        {loading ? 'Verifying...' : 'Login'}
                    </button>
                </form>
            </div>
        </div>
    );
}
