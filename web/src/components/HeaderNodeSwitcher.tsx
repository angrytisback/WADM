import { useState, useRef, useEffect } from 'react';
import { useCluster } from '../context/ClusterContext';
import { FaServer, FaChevronDown, FaCheck, FaPlus, FaCircle } from 'react-icons/fa';

interface HeaderNodeSwitcherProps {
    onOpenClusterTab?: () => void;
}

export function HeaderNodeSwitcher({ onOpenClusterTab }: HeaderNodeSwitcherProps) {
    const { activeNode, setActiveNode, nodes } = useCluster();
    const [isOpen, setIsOpen] = useState(false);
    const dropdownRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
                setIsOpen(false);
            }
        };
        document.addEventListener('mousedown', handleClickOutside);
        return () => document.removeEventListener('mousedown', handleClickOutside);
    }, []);

    const isMaster = activeNode === null;

    const getNodeStatusColor = (status?: string) => {
        if (!status) return 'var(--success, #22c55e)';
        switch (status) {
            case 'online': return 'var(--success, #22c55e)';
            case 'offline': return 'var(--danger, #ef4444)';
            default: return 'var(--warning, #f59e0b)';
        }
    };

    return (
        <div ref={dropdownRef} style={{ position: 'relative' }}>
            <button
                type="button"
                className="glass-panel"
                onClick={() => setIsOpen(!isOpen)}
                style={{
                    padding: '0.45rem 0.85rem',
                    fontSize: '0.85rem',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '0.6rem',
                    borderRadius: '10px',
                    border: '1px solid var(--glass-border, rgba(255,255,255,0.1))',
                    background: 'var(--glass-bg, rgba(15, 23, 42, 0.6))',
                    color: 'var(--text-primary, #f8fafc)',
                    cursor: 'pointer',
                    transition: 'all 0.2s ease',
                }}
                title="Switch active node or cluster server"
            >
                <FaServer style={{ color: 'var(--accent-color, #38bdf8)', fontSize: '0.9rem' }} />
                <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', textAlign: 'left' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                        <span style={{ fontWeight: 600, fontSize: '0.85rem' }}>
                            {isMaster ? 'Local Master' : activeNode.name}
                        </span>
                        <FaCircle
                            style={{
                                fontSize: '0.45rem',
                                color: isMaster ? 'var(--success, #22c55e)' : getNodeStatusColor(activeNode.status),
                            }}
                        />
                    </div>
                    <span style={{ fontSize: '0.7rem', color: 'var(--text-secondary, #94a3b8)', lineHeight: 1 }}>
                        {isMaster ? 'Primary Host' : (activeNode.hostname || activeNode.ip_address)}
                    </span>
                </div>
                <FaChevronDown
                    style={{
                        fontSize: '0.7rem',
                        color: 'var(--text-secondary, #94a3b8)',
                        transform: isOpen ? 'rotate(180deg)' : 'rotate(0deg)',
                        transition: 'transform 0.2s ease',
                        marginLeft: '0.2rem',
                    }}
                />
            </button>

            {isOpen && (
                <div
                    className="glass-panel custom-scroll"
                    style={{
                        position: 'absolute',
                        top: 'calc(100% + 6px)',
                        left: 0,
                        minWidth: '260px',
                        maxWidth: '320px',
                        maxHeight: '380px',
                        overflowY: 'auto',
                        borderRadius: '12px',
                        border: '1px solid var(--glass-border, rgba(255,255,255,0.15))',
                        background: 'rgba(15, 23, 42, 0.95)',
                        backdropFilter: 'blur(16px)',
                        boxShadow: '0 10px 25px -5px rgba(0, 0, 0, 0.5), 0 8px 10px -6px rgba(0, 0, 0, 0.5)',
                        zIndex: 100,
                        padding: '0.4rem',
                    }}
                >
                    <div
                        style={{
                            padding: '0.4rem 0.6rem',
                            fontSize: '0.7rem',
                            fontWeight: 700,
                            letterSpacing: '0.05em',
                            color: 'var(--text-secondary, #94a3b8)',
                        }}
                    >
                        CLUSTER NODES
                    </div>

                    {/* Local Master item */}
                    <div
                        onClick={() => {
                            setActiveNode(null);
                            setIsOpen(false);
                        }}
                        style={{
                            display: 'flex',
                            alignItems: 'center',
                            justifyContent: 'space-between',
                            padding: '0.55rem 0.75rem',
                            borderRadius: '8px',
                            cursor: 'pointer',
                            background: isMaster ? 'rgba(56, 189, 248, 0.15)' : 'transparent',
                            color: isMaster ? 'var(--text-primary, #fff)' : 'var(--text-secondary, #94a3b8)',
                            transition: 'background 0.15s ease',
                            marginBottom: '2px',
                        }}
                        onMouseEnter={(e) => {
                            if (!isMaster) e.currentTarget.style.background = 'rgba(255,255,255,0.05)';
                        }}
                        onMouseLeave={(e) => {
                            if (!isMaster) e.currentTarget.style.background = 'transparent';
                        }}
                    >
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                            <FaCircle style={{ fontSize: '0.5rem', color: 'var(--success, #22c55e)' }} />
                            <div>
                                <div style={{ fontSize: '0.85rem', fontWeight: 600 }}>Local Server (Master)</div>
                                <div style={{ fontSize: '0.7rem', opacity: 0.7 }}>Localhost / In-Process</div>
                            </div>
                        </div>
                        {isMaster && <FaCheck style={{ color: 'var(--accent-color, #38bdf8)', fontSize: '0.8rem' }} />}
                    </div>

                    {/* Remote Nodes list */}
                    {nodes.map((node) => {
                        const isCurrent = activeNode?.id === node.id;
                        return (
                            <div
                                key={node.id}
                                onClick={() => {
                                    setActiveNode(node);
                                    setIsOpen(false);
                                }}
                                style={{
                                    display: 'flex',
                                    alignItems: 'center',
                                    justifyContent: 'space-between',
                                    padding: '0.55rem 0.75rem',
                                    borderRadius: '8px',
                                    cursor: 'pointer',
                                    background: isCurrent ? 'rgba(56, 189, 248, 0.15)' : 'transparent',
                                    color: isCurrent ? 'var(--text-primary, #fff)' : 'var(--text-secondary, #94a3b8)',
                                    transition: 'background 0.15s ease',
                                    marginBottom: '2px',
                                }}
                                onMouseEnter={(e) => {
                                    if (!isCurrent) e.currentTarget.style.background = 'rgba(255,255,255,0.05)';
                                }}
                                onMouseLeave={(e) => {
                                    if (!isCurrent) e.currentTarget.style.background = 'transparent';
                                }}
                            >
                                <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                                    <FaCircle style={{ fontSize: '0.5rem', color: getNodeStatusColor(node.status) }} />
                                    <div>
                                        <div style={{ fontSize: '0.85rem', fontWeight: 600 }}>{node.name}</div>
                                        <div style={{ fontSize: '0.7rem', opacity: 0.7 }}>
                                            {node.ip_address} ({node.status})
                                        </div>
                                    </div>
                                </div>
                                {isCurrent && <FaCheck style={{ color: 'var(--accent-color, #38bdf8)', fontSize: '0.8rem' }} />}
                            </div>
                        );
                    })}

                    <div style={{ height: '1px', background: 'var(--glass-border, rgba(255,255,255,0.1))', margin: '0.4rem 0' }} />

                    {/* Link to Cluster Overview */}
                    <div
                        onClick={() => {
                            setIsOpen(false);
                            if (onOpenClusterTab) onOpenClusterTab();
                        }}
                        style={{
                            display: 'flex',
                            alignItems: 'center',
                            gap: '0.5rem',
                            padding: '0.5rem 0.75rem',
                            borderRadius: '8px',
                            cursor: 'pointer',
                            fontSize: '0.8rem',
                            color: 'var(--accent-color, #38bdf8)',
                            fontWeight: 500,
                            transition: 'background 0.15s ease',
                        }}
                        onMouseEnter={(e) => (e.currentTarget.style.background = 'rgba(56, 189, 248, 0.1)')}
                        onMouseLeave={(e) => (e.currentTarget.style.background = 'transparent')}
                    >
                        <FaPlus style={{ fontSize: '0.75rem' }} />
                        <span>Manage & Add Nodes...</span>
                    </div>
                </div>
            )}
        </div>
    );
}
