import { useState, useEffect, useCallback } from 'react';
import { useToast } from '../context/ToastContext';
import { useSystem } from '../context/SystemContext';
import { useModal } from '../context/ModalContext';
import { 
    FaDatabase, FaTable, FaCode, FaPlay, FaChevronRight, 
    FaChevronDown, FaSync, FaHistory, FaSearch, FaDatabase as FaDbIcon,
    FaDocker, FaExclamationTriangle, FaTrash, FaUndo, FaSave,
    FaFileExport, FaPlus, FaUpload, FaEdit, FaTimes
} from 'react-icons/fa';

interface Database {
    name: string;
    engine: string;
    size: string;
    container_id?: string;
}

interface Table {
    name: string;
}

interface QueryResult {
    columns: string[];
    rows: string[][];
}

interface Backup {
    filename: string;
    size: number;
    created_at: string;
}

export default function Database() {
    const { addToast } = useToast();
    const { confirm } = useModal();
    const { systemInfo } = useSystem();
    const canManage = systemInfo?.is_root || systemInfo?.has_sudo;
    
    // State
    const [dbs, setDbs] = useState<Database[]>([]);
    const [expandedDbs, setExpandedDbs] = useState<Record<string, boolean>>({});
    const [tablesMap, setTablesMap] = useState<Record<string, Table[]>>({});
    const [loadingDbs, setLoadingDbs] = useState(true);
    const [loadingTables, setLoadingTables] = useState<Record<string, boolean>>({});
    
    // Active Workspace
    const [activeView, setActiveView] = useState<'data' | 'sql' | 'backups'>('data');
    const [selectedEntity, setSelectedEntity] = useState<{ engine: string, db: string, table?: string, container_id?: string } | null>(null);
    const [dataResult, setDataResult] = useState<QueryResult | null>(null);
    const [loadingData, setLoadingData] = useState(false);
    
    // SQL Editor
    const [sqlQuery, setSqlQuery] = useState('');
    const [queryResult, setQueryResult] = useState<QueryResult | null>(null);
    const [queryError, setQueryError] = useState<string | null>(null);
    const [executing, setExecuting] = useState(false);

    // Backups
    const [backups, setBackups] = useState<Backup[]>([]);
    const [loadingBackups, setLoadingBackups] = useState(false);
    const [creatingBackup, setCreatingBackup] = useState(false);
    const [uploading, setUploading] = useState(false);
    
    // Row Editor
    const [editingRow, setEditingRow] = useState<{ index: number, values: string[] } | null>(null);
    const [savingRow, setSavingRow] = useState(false);

    const fetchDbs = useCallback(async () => {
        setLoadingDbs(true);
        try {
            const res = await fetch('/api/db');
            if (res.ok) {
                const data = await res.json();
                setDbs(data);
            }
        } catch (err) {
            console.error(err);
            addToast("Failed to fetch databases", "error");
        } finally {
            setLoadingDbs(false);
        }
    }, [addToast]);

    useEffect(() => {
        fetchDbs();
    }, [fetchDbs]);

    const toggleDb = async (db: Database) => {
        const id = `${db.engine}:${db.name}:${db.container_id || 'native'}`;
        const isExpanded = !!expandedDbs[id];
        
        setExpandedDbs(prev => ({ ...prev, [id]: !isExpanded }));
        
        if (!isExpanded && !tablesMap[id]) {
            setLoadingTables(prev => ({ ...prev, [id]: true }));
            try {
                const url = `/api/db/${db.engine}/${db.name}/tables${db.container_id ? `?container_id=${db.container_id}` : ''}`;
                const res = await fetch(url);
                if (res.ok) {
                    const tables = await res.json();
                    setTablesMap(prev => ({ ...prev, [id]: tables }));
                }
            } catch (err) {
                addToast(`Failed to load tables for ${db.name}`, "error");
            } finally {
                setLoadingTables(prev => ({ ...prev, [id]: false }));
            }
        }
        
        setSelectedEntity({ engine: db.engine, db: db.name, container_id: db.container_id });
        if (activeView === 'backups') fetchBackups(db.engine, db.name);
    };

    const selectTable = async (db: Database, table: Table) => {
        setSelectedEntity({ engine: db.engine, db: db.name, table: table.name, container_id: db.container_id });
        setActiveView('data');
        fetchTableData(db.engine, db.name, table.name, db.container_id);
    };

    const fetchTableData = async (engine: string, db: string, table: string, container_id?: string) => {
        setLoadingData(true);
        setDataResult(null);
        try {
            const url = `/api/db/${engine}/${db}/${table}/data${container_id ? `?container_id=${container_id}` : ''}`;
            const res = await fetch(url);
            if (res.ok) {
                const data = await res.json();
                setDataResult(data);
            } else {
                const err = await res.json();
                addToast(`Error: ${err}`, "error");
            }
        } catch {
            addToast("Failed to fetch table data", "error");
        } finally {
            setLoadingData(false);
        }
    };

    const runQuery = async (queryOverride?: string) => {
        const queryToRun = queryOverride || sqlQuery;
        if (!selectedEntity || !queryToRun.trim()) return;
        
        setExecuting(true);
        if (!queryOverride) {
            setQueryResult(null);
            setQueryError(null);
        }
        
        try {
            const url = `/api/db/${selectedEntity.engine}/${selectedEntity.db}/query${selectedEntity.container_id ? `?container_id=${selectedEntity.container_id}` : ''}`;
            const res = await fetch(url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ query: queryToRun })
            });
            const data = await res.json();
            if (res.ok) {
                if (queryOverride) {
                    addToast("Action successful", "success");
                    if (selectedEntity.table) fetchTableData(selectedEntity.engine, selectedEntity.db, selectedEntity.table, selectedEntity.container_id);
                } else {
                    setQueryResult(data);
                }
            } else {
                if (queryOverride) addToast(`Failed: ${data}`, "error");
                else setQueryError(data);
            }
        } catch {
            addToast("Failed to execute query", "error");
        } finally {
            setExecuting(false);
        }
    };

    // BACKUP LOGIC
    const fetchBackups = async (engine: string, db: string) => {
        setLoadingBackups(true);
        try {
            const res = await fetch(`/api/db/${engine}/${db}/backups`);
            if (res.ok) {
                setBackups(await res.json());
            }
        } catch {
            addToast("Failed to fetch backups", "error");
        } finally {
            setLoadingBackups(false);
        }
    };

    const handleCreateBackup = async () => {
        if (!selectedEntity) return;
        setCreatingBackup(true);
        try {
            const url = `/api/db/${selectedEntity.engine}/${selectedEntity.db}/backup${selectedEntity.container_id ? `?container_id=${selectedEntity.container_id}` : ''}`;
            const res = await fetch(url, { method: 'POST' });
            if (res.ok) {
                addToast("Backup created successfully", "success");
                fetchBackups(selectedEntity.engine, selectedEntity.db);
            } else {
                addToast("Failed to create backup", "error");
            }
        } catch {
            addToast("Error creating backup", "error");
        } finally {
            setCreatingBackup(false);
        }
    };

    const handleRestoreBackup = async (filename: string) => {
        if (!selectedEntity) return;
        const confirmed = await confirm({
            title: "Restore Database",
            message: `Are you sure you want to restore ${selectedEntity.db} from ${filename}? This will overwrite current data!`,
            confirmText: "Restore Now",
            type: "warning"
        });

        if (!confirmed) return;

        try {
            const url = `/api/db/${selectedEntity.engine}/${selectedEntity.db}/backups/${filename}${selectedEntity.container_id ? `?container_id=${selectedEntity.container_id}` : ''}`;
            const res = await fetch(url, { method: 'POST' });
            if (res.ok) {
                addToast("Database restored successfully", "success");
                if (selectedEntity.table) fetchTableData(selectedEntity.engine, selectedEntity.db, selectedEntity.table, selectedEntity.container_id);
            } else {
                addToast("Restore failed", "error");
            }
        } catch {
            addToast("Error during restore", "error");
        }
    };

    const handleDownload = async (filename: string) => {
        if (!selectedEntity) return;
        try {
            const url = `/api/db/${selectedEntity.engine}/${selectedEntity.db}/backups/${filename}/download`;
            const res = await fetch(url);
            if (!res.ok) {
                addToast("Download failed", "error");
                return;
            }
            const blob = await res.blob();
            const downloadUrl = window.URL.createObjectURL(blob);
            const link = document.createElement('a');
            link.href = downloadUrl;
            link.download = filename;
            document.body.appendChild(link);
            link.click();
            link.remove();
            window.URL.revokeObjectURL(downloadUrl);
        } catch {
            addToast("Error during download", "error");
        }
    };

    const handleUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (!file || !selectedEntity) return;

        setUploading(true);
        const formData = new FormData();
        formData.append('file', file);

        try {
            const res = await fetch(`/api/db/${selectedEntity.engine}/${selectedEntity.db}/upload`, {
                method: 'POST',
                body: formData
            });
            if (res.ok) {
                addToast("SQL file imported successfully", "success");
                fetchBackups(selectedEntity.engine, selectedEntity.db);
            } else {
                addToast("Failed to upload SQL file", "error");
            }
        } catch {
            addToast("Error during upload", "error");
        } finally {
            setUploading(false);
            e.target.value = ''; // Reset input
        }
    };

    const handleDeleteBackup = async (filename: string) => {
        if (!selectedEntity) return;
        const confirmed = await confirm({
            title: "Delete Backup",
            message: `Are you sure you want to delete ${filename}?`,
            confirmText: "Delete",
            type: "danger"
        });

        if (!confirmed) return;

        try {
            const res = await fetch(`/api/db/${selectedEntity.engine}/${selectedEntity.db}/backups/${filename}`, { method: 'DELETE' });
            if (res.ok) {
                addToast("Backup deleted", "success");
                fetchBackups(selectedEntity.engine, selectedEntity.db);
            }
        } catch {
            addToast("Failed to delete backup", "error");
        }
    };

    const deleteRow = async (rowIndex: number) => {
        if (!selectedEntity?.table || !dataResult) return;
        
        const row = dataResult.rows[rowIndex];
        const primaryKeyCol = dataResult.columns[0]; 
        const pkValue = row[0];

        const confirmed = await confirm({
            title: "Delete Row",
            message: `Are you sure you want to delete row where ${primaryKeyCol} = '${pkValue}'?`,
            confirmText: "Delete Row",
            type: "danger"
        });

        if (!confirmed) return;

        const query = `DELETE FROM ${selectedEntity.table} WHERE ${primaryKeyCol} = '${pkValue}'`;
        runQuery(query);
    };

    const saveEditedRow = async () => {
        if (!selectedEntity?.table || !dataResult || !editingRow) return;
        
        setSavingRow(true);
        try {
            const columns = dataResult.columns;
            const primaryKeyCol = columns[0];
            const pkValue = dataResult.rows[editingRow.index][0];
            
            // Build SET clause
            const setClause = columns.map((col, i) => {
                const val = editingRow.values[i];
                const escapedVal = val.replace(/'/g, "''");
                return `${col} = '${escapedVal}'`;
            }).join(', ');

            const query = `UPDATE ${selectedEntity.table} SET ${setClause} WHERE ${primaryKeyCol} = '${pkValue}'`;
            
            const url = `/api/db/${selectedEntity.engine}/${selectedEntity.db}/query${selectedEntity.container_id ? `?container_id=${selectedEntity.container_id}` : ''}`;
            const res = await fetch(url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ query })
            });

            if (res.ok) {
                addToast("Row updated successfully", "success");
                setEditingRow(null);
                fetchTableData(selectedEntity.engine, selectedEntity.db, selectedEntity.table, selectedEntity.container_id);
            } else {
                const err = await res.json();
                addToast(`Failed to update row: ${err}`, "error");
            }
        } catch {
            addToast("Error saving row", "error");
        } finally {
            setSavingRow(false);
        }
    };

    const applyTemplate = (type: string) => {
        if (!selectedEntity?.table) return;
        let tpl = '';
        const table = selectedEntity.table;
        switch(type) {
            case 'select': tpl = `SELECT * FROM ${table} WHERE id = 1;`; break;
            case 'insert': tpl = `INSERT INTO ${table} (column1, column2) VALUES ('value1', 'value2');`; break;
            case 'update': tpl = `UPDATE ${table} SET column1 = 'value' WHERE id = 1;`; break;
            case 'delete': tpl = `DELETE FROM ${table} WHERE id = 1;`; break;
        }
        setSqlQuery(tpl);
        setActiveView('sql');
    };

    if (!canManage) {
        return (
            <div className="glass-panel" style={{ padding: '3rem', textAlign: 'center' }}>
                <FaExclamationTriangle style={{ fontSize: '3rem', color: 'var(--warning)', marginBottom: '1rem' }} />
                <h3>Access Restricted</h3>
                <p style={{ color: 'var(--text-secondary)' }}>You need Root or Sudo privileges to manage databases.</p>
            </div>
        );
    }

    return (
        <div style={{ display: 'grid', gridTemplateColumns: '300px 1fr', gap: '1.5rem', height: 'calc(100vh - 120px)', minWidth: 0 }}>
            
            {/* SIDEBAR: Navigator */}
            <div className="glass-panel" style={{ display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
                <header style={{ padding: '1rem', borderBottom: '1px solid var(--glass-border)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontWeight: 600 }}>
                        <FaDbIcon style={{ color: 'var(--accent-color)' }} /> Navigator
                    </div>
                    <button className="btn-text" onClick={fetchDbs}><FaSync /></button>
                </header>
                
                <div style={{ flex: 1, overflowY: 'auto', padding: '0.5rem' }} className="no-scrollbar">
                    {loadingDbs ? (
                        <div style={{ padding: '1rem', color: 'var(--text-secondary)' }}>Loading schema...</div>
                    ) : dbs.length === 0 ? (
                        <div style={{ padding: '1rem', color: 'var(--text-secondary)', fontSize: '0.9rem' }}>No databases found.</div>
                    ) : (
                        dbs.map(db => {
                            const id = `${db.engine}:${db.name}:${db.container_id || 'native'}`;
                            const isExpanded = expandedDbs[id];
                            const tables = tablesMap[id] || [];
                            const isLoading = loadingTables[id];
                            
                            return (
                                <div key={id} style={{ marginBottom: '0.25rem' }}>
                                    <div 
                                        onClick={() => toggleDb(db)}
                                        style={{ 
                                            display: 'flex', alignItems: 'center', gap: '0.5rem', 
                                            padding: '0.5rem', borderRadius: '6px', cursor: 'pointer',
                                            background: selectedEntity?.db === db.name && selectedEntity?.container_id === db.container_id && !selectedEntity.table ? 'rgba(56, 189, 248, 0.1)' : 'transparent',
                                            transition: 'background 0.2s'
                                        }}
                                        className="item-hover"
                                    >
                                        {isExpanded ? <FaChevronDown style={{ fontSize: '0.7rem' }} /> : <FaChevronRight style={{ fontSize: '0.7rem' }} />}
                                        <FaDatabase style={{ color: db.engine === 'postgres' ? '#336791' : '#f29111', fontSize: '0.9rem' }} />
                                        <span style={{ fontSize: '0.9rem', fontWeight: 500 }}>{db.name}</span>
                                        <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                                            {db.container_id && (
                                                <span title="Docker Container" style={{ 
                                                    background: 'rgba(56, 189, 248, 0.2)', 
                                                    color: 'var(--accent-color)', 
                                                    fontSize: '0.65rem', 
                                                    padding: '0.1rem 0.3rem', 
                                                    borderRadius: '4px',
                                                    display: 'flex',
                                                    alignItems: 'center',
                                                    gap: '0.2rem'
                                                }}>
                                                    <FaDocker /> DOCKER
                                                </span>
                                            )}
                                            <span style={{ fontSize: '0.7rem', color: 'var(--text-secondary)' }}>{db.engine}</span>
                                        </div>
                                    </div>
                                    
                                    {isExpanded && (
                                        <div style={{ marginLeft: '1.5rem', borderLeft: '1px solid var(--glass-border)', paddingLeft: '0.5rem' }}>
                                            {isLoading ? (
                                                <div style={{ padding: '0.4rem', fontSize: '0.8rem', color: 'var(--text-secondary)' }}>Loading tables...</div>
                                            ) : tables.length === 0 ? (
                                                <div style={{ padding: '0.4rem', fontSize: '0.8rem', color: 'var(--text-secondary)' }}>No tables found</div>
                                            ) : (
                                                tables.map(t => (
                                                    <div 
                                                        key={t.name}
                                                        onClick={() => selectTable(db, t)}
                                                        style={{ 
                                                            display: 'flex', alignItems: 'center', gap: '0.5rem', 
                                                            padding: '0.4rem', borderRadius: '4px', cursor: 'pointer',
                                                            background: selectedEntity?.table === t.name && selectedEntity?.db === db.name && selectedEntity?.container_id === db.container_id ? 'rgba(56, 189, 248, 0.1)' : 'transparent',
                                                            fontSize: '0.85rem'
                                                        }}
                                                        className="item-hover"
                                                    >
                                                        <FaTable style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }} />
                                                        {t.name}
                                                    </div>
                                                ))
                                            )}
                                        </div>
                                    )}
                                </div>
                            );
                        })
                    )}
                </div>
            </div>

            {/* MAIN AREA: Workspace */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: '1rem', minWidth: 0 }}>
                
                {/* Workspace Header/Tabs */}
                <div className="glass-panel" style={{ padding: '0.5rem 1rem', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                    <div style={{ display: 'flex', gap: '1rem' }}>
                        <button 
                            className={`btn-text ${activeView === 'data' ? 'active' : ''}`}
                            onClick={() => setActiveView('data')}
                            style={{ borderBottom: activeView === 'data' ? '2px solid var(--accent-color)' : '2px solid transparent', padding: '0.5rem' }}
                        >
                            <FaSearch style={{ marginRight: '0.5rem' }} /> Data Browser
                        </button>
                        <button 
                            className={`btn-text ${activeView === 'sql' ? 'active' : ''}`}
                            onClick={() => setActiveView('sql')}
                            style={{ borderBottom: activeView === 'sql' ? '2px solid var(--accent-color)' : '2px solid transparent', padding: '0.5rem' }}
                        >
                            <FaCode style={{ marginRight: '0.5rem' }} /> SQL Editor
                        </button>
                        <button 
                            className={`btn-text ${activeView === 'backups' ? 'active' : ''}`}
                            onClick={() => {
                                setActiveView('backups');
                                if (selectedEntity) fetchBackups(selectedEntity.engine, selectedEntity.db);
                            }}
                            style={{ borderBottom: activeView === 'backups' ? '2px solid var(--accent-color)' : '2px solid transparent', padding: '0.5rem' }}
                        >
                            <FaHistory style={{ marginRight: '0.5rem' }} /> Backups
                        </button>
                    </div>
                    {selectedEntity && (
                        <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)', display: 'flex', alignItems: 'center' }}>
                            {selectedEntity.container_id && <FaDocker style={{ color: 'var(--accent-color)', marginRight: '0.5rem' }} />}
                            <span style={{ fontWeight: 600, color: 'var(--accent-color)' }}>{selectedEntity.engine}</span>
                            <FaChevronRight style={{ margin: '0 0.5rem', fontSize: '0.6rem' }} />
                            <span>{selectedEntity.db}</span>
                            {selectedEntity.table && (
                                <>
                                    <FaChevronRight style={{ margin: '0 0.5rem', fontSize: '0.6rem' }} />
                                    <span style={{ color: 'var(--text-primary)' }}>{selectedEntity.table}</span>
                                </>
                            )}
                        </div>
                    )}
                </div>

                {/* Content Area */}
                <div className="glass-panel" style={{ flex: 1, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
                    
                    {activeView === 'data' ? (
                        <div style={{ flex: 1, display: 'flex', flexDirection: 'column' }}>
                            {!selectedEntity?.table ? (
                                <div style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', color: 'var(--text-secondary)' }}>
                                    <FaTable style={{ fontSize: '4rem', marginBottom: '1rem', opacity: 0.2 }} />
                                    <p>Select a table from the navigator to browse data</p>
                                </div>
                            ) : loadingData ? (
                                <div style={{ padding: '2rem', textAlign: 'center' }}>Loading records...</div>
                            ) : dataResult ? (
                                <div style={{ overflow: 'auto', flex: 1 }} className="custom-scroll">
                                    <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.85rem' }}>
                                        <thead style={{ background: 'rgba(0,0,0,0.2)', position: 'sticky', top: 0, zIndex: 10 }}>
                                            <tr>
                                                <th style={{ padding: '0.75rem', width: '80px' }}>Actions</th>
                                                {dataResult.columns.map((col, i) => (
                                                    <th key={i} style={{ padding: '0.75rem', textAlign: 'left', borderBottom: '1px solid var(--glass-border)', whiteSpace: 'nowrap' }}>{col}</th>
                                                ))}
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {dataResult.rows.map((row, ri) => (
                                                <tr key={ri} className="table-row-hover">
                                                    <td style={{ padding: '0.5rem', textAlign: 'center' }}>
                                                        <div style={{ display: 'flex', gap: '0.4rem', justifyContent: 'center' }}>
                                                            <button 
                                                                className="btn-text" 
                                                                title="Edit Row"
                                                                onClick={() => setEditingRow({ index: ri, values: [...row] })}
                                                                style={{ padding: '0.2rem', fontSize: '0.75rem', color: 'var(--accent-color)' }}
                                                            >
                                                                <FaEdit />
                                                            </button>
                                                            <button 
                                                                className="btn-text danger" 
                                                                title="Delete Row"
                                                                onClick={() => deleteRow(ri)}
                                                                style={{ padding: '0.2rem', fontSize: '0.75rem' }}
                                                            >
                                                                <FaTrash />
                                                            </button>
                                                        </div>
                                                    </td>
                                                    {row.map((val, ci) => (
                                                        <td key={ci} style={{ padding: '0.5rem 0.75rem', borderBottom: '1px solid rgba(255,255,255,0.05)', whiteSpace: 'nowrap', maxWidth: '300px', overflow: 'hidden', textOverflow: 'ellipsis' }}>
                                                            {val === null || val === 'NULL' ? <span style={{ color: 'rgba(255,255,255,0.2)', fontStyle: 'italic' }}>NULL</span> : val}
                                                        </td>
                                                    ))}
                                                </tr>
                                            ))}
                                        </tbody>
                                    </table>
                                </div>
                            ) : (
                                <div style={{ padding: '2rem', textAlign: 'center', color: 'var(--text-secondary)' }}>Failed to load data</div>
                            )}
                        </div>
                    ) : activeView === 'sql' ? (
                        /* SQL Editor View */
                        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', padding: '1rem' }}>
                            <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '1rem' }}>
                                <button className="btn-sm" onClick={() => applyTemplate('select')}><FaSearch /> SELECT</button>
                                <button className="btn-sm" onClick={() => applyTemplate('insert')}><FaPlus /> INSERT</button>
                                <button className="btn-sm" onClick={() => applyTemplate('update')}><FaSave /> UPDATE</button>
                                <button className="btn-sm danger" onClick={() => applyTemplate('delete')}><FaTrash /> DELETE</button>
                            </div>
                            <div style={{ position: 'relative', flex: '0 0 200px', marginBottom: '1rem' }}>
                                <textarea
                                    className="input-field"
                                    placeholder="Enter SQL query here..."
                                    value={sqlQuery}
                                    onChange={(e) => setSqlQuery(e.target.value)}
                                    style={{ 
                                        width: '100%', height: '100%', fontFamily: 'JetBrains Mono, monospace', 
                                        background: 'rgba(0,0,0,0.3)', resize: 'none', padding: '1rem'
                                    }}
                                />
                                <button 
                                    className="btn-primary" 
                                    onClick={() => runQuery()}
                                    disabled={executing || !sqlQuery.trim() || !selectedEntity}
                                    style={{ position: 'absolute', bottom: '1rem', right: '1rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}
                                >
                                    {executing ? <span className="spinner"></span> : <FaPlay />}
                                    Run Query
                                </button>
                            </div>
                            
                            <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
                                <header style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', marginBottom: '0.5rem', fontSize: '0.9rem', color: 'var(--text-secondary)' }}>
                                    <FaHistory /> Results
                                </header>
                                <div className="custom-scroll" style={{ flex: 1, background: 'rgba(0,0,0,0.2)', borderRadius: '8px', overflow: 'auto' }}>
                                    {queryError ? (
                                        <div style={{ padding: '1rem', color: '#f87171', fontFamily: 'monospace', fontSize: '0.85rem' }}>{queryError}</div>
                                    ) : queryResult ? (
                                        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.85rem' }}>
                                            <thead style={{ background: 'rgba(255,255,255,0.05)', position: 'sticky', top: 0 }}>
                                                <tr>
                                                    {queryResult.columns.map((col, i) => (
                                                        <th key={i} style={{ padding: '0.75rem', textAlign: 'left', borderBottom: '1px solid var(--glass-border)', whiteSpace: 'nowrap' }}>{col}</th>
                                                    ))}
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {queryResult.rows.map((row, ri) => (
                                                    <tr key={ri}>
                                                        {row.map((val, ci) => (
                                                            <td key={ci} style={{ padding: '0.5rem 0.75rem', borderBottom: '1px solid rgba(255,255,255,0.03)', whiteSpace: 'nowrap' }}>{val}</td>
                                                        ))}
                                                    </tr>
                                                ))}
                                            </tbody>
                                        </table>
                                    ) : (
                                        <div style={{ padding: '2rem', textAlign: 'center', color: 'rgba(255,255,255,0.2)' }}>No results to display</div>
                                    )}
                                </div>
                            </div>
                        </div>
                    ) : (
                        /* BACKUPS VIEW */
                        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', padding: '1.5rem' }}>
                            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.5rem' }}>
                                <h3>Database Backups</h3>
                                <div style={{ display: 'flex', gap: '0.8rem' }}>
                                    <label className={`btn-primary ${uploading ? 'disabled' : ''}`} style={{ cursor: 'pointer', display: 'flex', alignItems: 'center', margin: 0 }}>
                                        {uploading ? <span className="spinner"></span> : <FaUpload style={{ marginRight: '0.5rem' }} />}
                                        Import SQL File
                                        <input type="file" accept=".sql" onChange={handleUpload} style={{ display: 'none' }} disabled={uploading || !selectedEntity} />
                                    </label>
                                    <button className="btn-primary" onClick={handleCreateBackup} disabled={creatingBackup || !selectedEntity}>
                                        {creatingBackup ? <span className="spinner"></span> : <FaSave style={{ marginRight: '0.5rem' }} />}
                                        Take Instant Backup
                                    </button>
                                </div>
                            </div>

                            <div className="custom-scroll" style={{ flex: 1, background: 'rgba(0,0,0,0.2)', borderRadius: '12px', overflow: 'auto' }}>
                                {loadingBackups ? (
                                    <div style={{ padding: '2rem', textAlign: 'center' }}>Scanning backups...</div>
                                ) : backups.length === 0 ? (
                                    <div style={{ padding: '4rem', textAlign: 'center', color: 'var(--text-secondary)' }}>
                                        <FaHistory style={{ fontSize: '3rem', marginBottom: '1rem', opacity: 0.1 }} />
                                        <p>No backups found for this database.</p>
                                    </div>
                                ) : (
                                    <table style={{ width: '100%', borderCollapse: 'collapse' }}>
                                        <thead>
                                            <tr style={{ background: 'rgba(255,255,255,0.05)' }}>
                                                <th style={{ padding: '1rem', textAlign: 'left' }}>Filename</th>
                                                <th style={{ padding: '1rem', textAlign: 'left' }}>Size</th>
                                                <th style={{ padding: '1rem', textAlign: 'left' }}>Created At</th>
                                                <th style={{ padding: '1rem', textAlign: 'right' }}>Actions</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {backups.map(b => (
                                                <tr key={b.filename} style={{ borderBottom: '1px solid var(--glass-border)' }}>
                                                    <td style={{ padding: '1rem', fontWeight: 500 }}>{b.filename}</td>
                                                    <td style={{ padding: '1rem', color: 'var(--text-secondary)' }}>{(b.size / 1024).toFixed(2)} KB</td>
                                                    <td style={{ padding: '1rem', color: 'var(--text-secondary)' }}>{b.created_at}</td>
                                                    <td style={{ padding: '1rem', textAlign: 'right' }}>
                                                        <div style={{ display: 'flex', gap: '0.5rem', justifyContent: 'flex-end' }}>
                                                            <button 
                                                                className="btn-sm success" 
                                                                title="Restore" 
                                                                onClick={() => handleRestoreBackup(b.filename)}
                                                            >
                                                                <FaUndo /> Restore
                                                            </button>
                                                            <button 
                                                                className="btn-sm" 
                                                                title="Export SQL" 
                                                                onClick={() => handleDownload(b.filename)}
                                                                style={{ display: 'inline-flex', alignItems: 'center' }}
                                                            >
                                                                <FaFileExport style={{ marginRight: '0.4rem' }} /> Export
                                                            </button>
                                                            <button 
                                                                className="btn-sm danger" 
                                                                title="Delete" 
                                                                onClick={() => handleDeleteBackup(b.filename)}
                                                            >
                                                                <FaTrash />
                                                            </button>
                                                        </div>
                                                    </td>
                                                </tr>
                                            ))}
                                        </tbody>
                                    </table>
                                )}
                            </div>
                        </div>
                    )}
                </div>
                
                {/* Footer Status */}
                <footer className="glass-panel" style={{ padding: '0.5rem 1rem', fontSize: '0.8rem', display: 'flex', justifyContent: 'space-between', color: 'var(--text-secondary)' }}>
                    <div>
                        {executing ? 'Executing query...' : 'Ready'}
                    </div>
                    {queryResult && <div>{queryResult.rows.length} rows returned</div>}
                </footer>
            </div>

            {/* ROW EDITOR MODAL */}
            {editingRow && dataResult && (
                <div style={{ 
                    position: 'fixed', top: 0, left: 0, right: 0, bottom: 0, 
                    background: 'rgba(0,0,0,0.7)', display: 'flex', alignItems: 'center', 
                    justifyContent: 'center', zIndex: 2000, padding: '2rem'
                }}>
                    <div className="glass-panel" style={{ width: '100%', maxWidth: '600px', display: 'flex', flexDirection: 'column', maxHeight: '90vh' }}>
                        <header style={{ padding: '1.25rem', borderBottom: '1px solid var(--glass-border)', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                            <h3 style={{ margin: 0, display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                                <FaEdit style={{ color: 'var(--accent-color)' }} /> Edit Row
                            </h3>
                            <button className="btn-text" onClick={() => setEditingRow(null)}><FaTimes /></button>
                        </header>
                        
                        <div className="custom-scroll" style={{ flex: 1, overflowY: 'auto', padding: '1.5rem' }}>
                            {dataResult.columns.map((col, i) => (
                                <div key={col} style={{ marginBottom: '1rem' }}>
                                    <label style={{ display: 'block', fontSize: '0.8rem', color: 'var(--text-secondary)', marginBottom: '0.4rem', fontWeight: 600 }}>
                                        {col} {i === 0 && <span style={{ color: 'var(--accent-color)', fontSize: '0.7rem' }}>(PRIMARY KEY)</span>}
                                    </label>
                                    <input 
                                        type="text"
                                        className="input-field"
                                        value={editingRow.values[i] || ''}
                                        onChange={(e) => {
                                            const newValues = [...editingRow.values];
                                            newValues[i] = e.target.value;
                                            setEditingRow({ ...editingRow, values: newValues });
                                        }}
                                        style={{ width: '100%' }}
                                        disabled={i === 0} // Disable PK editing for safety
                                    />
                                </div>
                            ))}
                        </div>

                        <footer style={{ padding: '1.25rem', borderTop: '1px solid var(--glass-border)', display: 'flex', justifyContent: 'flex-end', gap: '1rem' }}>
                            <button className="btn-text" onClick={() => setEditingRow(null)}>Cancel</button>
                            <button className="btn-primary" onClick={saveEditedRow} disabled={savingRow}>
                                {savingRow ? <span className="spinner"></span> : <FaSave style={{ marginRight: '0.5rem' }} />}
                                Save Changes
                            </button>
                        </footer>
                    </div>
                </div>
            )}

            <style>{`
                .item-hover:hover {
                    background: rgba(255, 255, 255, 0.05) !important;
                }
                .table-row-hover:hover {
                    background: rgba(56, 189, 248, 0.05) !important;
                }
                .custom-scroll::-webkit-scrollbar {
                    width: 6px;
                    height: 6px;
                }
                .custom-scroll::-webkit-scrollbar-thumb {
                    background: rgba(255, 255, 255, 0.1);
                    border-radius: 10px;
                }
                .custom-scroll::-webkit-scrollbar-track {
                    background: transparent;
                }
            `}</style>
        </div>
    );
}
