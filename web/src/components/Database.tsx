import { useState, useEffect } from 'react';

interface Database {
    name: string;
    engine: string;
    size: string;
}

export default function Database() {
    const [dbs, setDbs] = useState<Database[]>([]);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        fetchDbs();
    }, []);

    const fetchDbs = async () => {
        try {
            const res = await fetch('/api/db');
            const data = await res.json();
            setDbs(data);
        } catch (err) {
            console.error(err);
        } finally {
            setLoading(false);
        }
    };

    if (loading) return <div className="glass-panel" style={{ padding: '2rem' }}>Loading databases...</div>;

    return (
        <div className="glass-panel" style={{ padding: '2rem' }}>
            <header style={{ marginBottom: '2rem', display: 'flex', justifyContent: 'space-between' }}>
                <h3>Databases</h3>
                <button className="btn-primary" onClick={fetchDbs}>Refresh</button>
            </header>

            {dbs.length === 0 ? (
                <p style={{ color: 'var(--text-secondary)' }}>No databases found (MySQL/PostgreSQL).</p>
            ) : (
                <table style={{ width: '100%', borderCollapse: 'collapse' }}>
                    <thead>
                        <tr style={{ textAlign: 'left', borderBottom: '1px solid var(--glass-border)' }}>
                            <th style={{ padding: '1rem' }}>Name</th>
                            <th style={{ padding: '1rem' }}>Engine</th>
                            <th style={{ padding: '1rem' }}>Size</th>
                        </tr>
                    </thead>
                    <tbody>
                        {dbs.map((db, idx) => (
                            <tr key={idx} style={{ borderBottom: '1px solid var(--glass-border)' }}>
                                <td style={{ padding: '1rem', fontWeight: 500 }}>{db.name}</td>
                                <td style={{ padding: '1rem' }}>
                                    <span className="badge neutral" style={{ textTransform: 'capitalize' }}>{db.engine}</span>
                                </td>
                                <td style={{ padding: '1rem', color: 'var(--text-secondary)' }}>{db.size}</td>
                            </tr>
                        ))}
                    </tbody>
                </table>
            )}
        </div>
    );
}
