import { useState } from 'react';
import { useCluster } from '../context/ClusterContext';
import { useToast } from '../context/ToastContext';
import type { ClusterNode, JoinTokenResponse, NodeSpecsData } from '../types';
import {
    FaServer,
    FaPlus,
    FaTrash,
    FaSync,
    FaCopy,
    FaCheck,
    FaDesktop,
    FaMicrochip,
    FaMemory,
    FaHeartbeat,
    FaNetworkWired,
    FaShieldAlt,
} from 'react-icons/fa';

interface ClusterOverviewProps {
    onSelectNodeAndNavigate?: (node: ClusterNode | null) => void;
}

export function ClusterOverview({ onSelectNodeAndNavigate }: ClusterOverviewProps) {
    const { activeNode, setActiveNode, nodes, isLoading, refreshNodes, generateJoinToken, deleteNode } =
        useCluster();
    const { addToast } = useToast();

    const [isAddModalOpen, setIsAddModalOpen] = useState(false);
    const [joinData, setJoinData] = useState<JoinTokenResponse | null>(null);
    const [isGenerating, setIsGenerating] = useState(false);
    const [copiedToken, setCopiedToken] = useState(false);
    const [copiedCommand, setCopiedCommand] = useState(false);

    const [nodeToDelete, setNodeToDelete] = useState<ClusterNode | null>(null);
    const [isDeleting, setIsDeleting] = useState(false);

    const totalNodes = nodes.length + 1; // including Local Master
    const onlineNodes = nodes.filter((n) => n.status === 'online').length + 1;
    const offlineNodes = nodes.filter((n) => n.status === 'offline').length;

    const handleOpenAddModal = async () => {
        setIsAddModalOpen(true);
        setIsGenerating(true);
        const data = await generateJoinToken();
        setJoinData(data);
        setIsGenerating(false);
    };

    const handleCopy = (text: string, type: 'token' | 'command') => {
        navigator.clipboard.writeText(text);
        if (type === 'token') {
            setCopiedToken(true);
            setTimeout(() => setCopiedToken(false), 2000);
        } else {
            setCopiedCommand(true);
            setTimeout(() => setCopiedCommand(false), 2000);
        }
        addToast('Copied to clipboard!', 'success');
    };

    const confirmDelete = async () => {
        if (!nodeToDelete) return;
        setIsDeleting(true);
        await deleteNode(nodeToDelete.id);
        setIsDeleting(false);
        setNodeToDelete(null);
    };

    const parseSpecs = (specsStr: string): NodeSpecsData | null => {
        try {
            return JSON.parse(specsStr);
        } catch {
            return null;
        }
    };

    const formatBytes = (bytes: number) => {
        if (!bytes || bytes === 0) return '0 GB';
        const gb = bytes / (1024 * 1024 * 1024);
        return `${gb.toFixed(1)} GB`;
    };

    const formatHeartbeat = (hbStr: string | null) => {
        if (!hbStr) return 'Never';
        try {
            const date = new Date(hbStr);
            return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
        } catch {
            return hbStr;
        }
    };

    return (
        <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
            {/* Header section */}
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '1rem' }}>
                <div>
                    <h2 style={{ fontSize: '1.6rem', fontWeight: 700, margin: 0, display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                        <FaNetworkWired style={{ color: 'var(--accent-color, #38bdf8)' }} />
                        Cluster Federation
                    </h2>
                    <p style={{ color: 'var(--text-secondary, #94a3b8)', margin: '0.3rem 0 0 0', fontSize: '0.9rem' }}>
                        Outbound reverse WebSocket tunnel architecture. Nodes securely connect without opening firewall ports.
                    </p>
                </div>
                <div style={{ display: 'flex', gap: '0.75rem' }}>
                    <button
                        className="btn-secondary"
                        onClick={() => refreshNodes()}
                        disabled={isLoading}
                        style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.55rem 1rem' }}
                    >
                        <FaSync className={isLoading ? 'spin' : ''} />
                        <span>Refresh</span>
                    </button>
                    <button
                        className="btn-primary"
                        onClick={handleOpenAddModal}
                        style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', padding: '0.55rem 1.2rem', fontWeight: 600 }}
                    >
                        <FaPlus />
                        <span>Add New Node</span>
                    </button>
                </div>
            </div>

            {/* Metrics cards */}
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))', gap: '1rem' }}>
                <div className="glass-panel" style={{ padding: '1.2rem', display: 'flex', alignItems: 'center', gap: '1rem', borderRadius: '12px' }}>
                    <div style={{ padding: '0.8rem', borderRadius: '10px', background: 'rgba(56, 189, 248, 0.15)', color: '#38bdf8', fontSize: '1.4rem' }}>
                        <FaServer />
                    </div>
                    <div>
                        <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary, #94a3b8)', fontWeight: 600 }}>TOTAL NODES</div>
                        <div style={{ fontSize: '1.8rem', fontWeight: 700 }}>{totalNodes}</div>
                    </div>
                </div>

                <div className="glass-panel" style={{ padding: '1.2rem', display: 'flex', alignItems: 'center', gap: '1rem', borderRadius: '12px' }}>
                    <div style={{ padding: '0.8rem', borderRadius: '10px', background: 'rgba(34, 197, 94, 0.15)', color: '#22c55e', fontSize: '1.4rem' }}>
                        <FaHeartbeat />
                    </div>
                    <div>
                        <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary, #94a3b8)', fontWeight: 600 }}>ONLINE NODES</div>
                        <div style={{ fontSize: '1.8rem', fontWeight: 700, color: '#22c55e' }}>{onlineNodes}</div>
                    </div>
                </div>

                <div className="glass-panel" style={{ padding: '1.2rem', display: 'flex', alignItems: 'center', gap: '1rem', borderRadius: '12px' }}>
                    <div style={{ padding: '0.8rem', borderRadius: '10px', background: 'rgba(239, 68, 68, 0.15)', color: '#ef4444', fontSize: '1.4rem' }}>
                        <FaShieldAlt />
                    </div>
                    <div>
                        <div style={{ fontSize: '0.8rem', color: 'var(--text-secondary, #94a3b8)', fontWeight: 600 }}>OFFLINE NODES</div>
                        <div style={{ fontSize: '1.8rem', fontWeight: 700, color: offlineNodes > 0 ? '#ef4444' : 'var(--text-primary)' }}>
                            {offlineNodes}
                        </div>
                    </div>
                </div>
            </div>

            {/* Nodes Grid */}
            <h3 style={{ fontSize: '1.2rem', fontWeight: 600, margin: '0.5rem 0 0 0' }}>Connected Infrastructure</h3>
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(340px, 1fr))', gap: '1.2rem' }}>
                {/* Local Master Server Card */}
                <div
                    className="glass-panel"
                    style={{
                        padding: '1.4rem',
                        borderRadius: '14px',
                        border: activeNode === null ? '2px solid var(--accent-color, #38bdf8)' : '1px solid var(--glass-border)',
                        display: 'flex',
                        flexDirection: 'column',
                        gap: '1rem',
                        position: 'relative',
                    }}
                >
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                        <div>
                            <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                                <span style={{ fontSize: '1.1rem', fontWeight: 700 }}>Local Server (Master)</span>
                                <span style={{ padding: '2px 8px', borderRadius: '12px', fontSize: '0.7rem', fontWeight: 700, background: 'rgba(56, 189, 248, 0.2)', color: '#38bdf8', border: '1px solid #38bdf840' }}>
                                    PRIMARY HUB
                                </span>
                            </div>
                            <span style={{ fontSize: '0.8rem', color: 'var(--text-secondary, #94a3b8)' }}>
                                127.0.0.1 (In-Process Panel)
                            </span>
                        </div>
                        <span style={{ display: 'inline-flex', alignItems: 'center', gap: '0.4rem', padding: '3px 10px', borderRadius: '20px', fontSize: '0.75rem', fontWeight: 600, background: 'rgba(34, 197, 94, 0.15)', color: '#22c55e', border: '1px solid #22c55e40' }}>
                            <span style={{ width: '6px', height: '6px', borderRadius: '50%', background: '#22c55e' }} />
                            Online
                        </span>
                    </div>

                    <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0.6rem', padding: '0.8rem', borderRadius: '8px', background: 'rgba(0,0,0,0.2)', fontSize: '0.8rem' }}>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', color: 'var(--text-secondary)' }}>
                            <FaDesktop /> Host Machine
                        </div>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', color: 'var(--text-secondary)' }}>
                            <FaHeartbeat /> Native Telemetry
                        </div>
                    </div>

                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: 'auto', paddingTop: '0.5rem' }}>
                        <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>Status: Active Master</span>
                        <button
                            className={activeNode === null ? 'btn-primary' : 'btn-secondary'}
                            onClick={() => {
                                setActiveNode(null);
                                if (onSelectNodeAndNavigate) onSelectNodeAndNavigate(null);
                            }}
                            style={{ padding: '0.45rem 1rem', fontSize: '0.85rem' }}
                        >
                            {activeNode === null ? 'Currently Selected' : 'Switch to Master'}
                        </button>
                    </div>
                </div>

                {/* Cluster Worker Nodes */}
                {nodes.map((node) => {
                    const specs = parseSpecs(node.specs);
                    const isActive = activeNode?.id === node.id;
                    const isOnline = node.status === 'online';

                    return (
                        <div
                            key={node.id}
                            className="glass-panel"
                            style={{
                                padding: '1.4rem',
                                borderRadius: '14px',
                                border: isActive ? '2px solid var(--accent-color, #38bdf8)' : '1px solid var(--glass-border)',
                                display: 'flex',
                                flexDirection: 'column',
                                gap: '1rem',
                                position: 'relative',
                            }}
                        >
                            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                                <div>
                                    <div style={{ fontSize: '1.1rem', fontWeight: 700 }}>{node.name}</div>
                                    <span style={{ fontSize: '0.8rem', color: 'var(--text-secondary, #94a3b8)' }}>
                                        {node.hostname} • {node.ip_address}
                                    </span>
                                </div>
                                <span
                                    style={{
                                        display: 'inline-flex',
                                        alignItems: 'center',
                                        gap: '0.4rem',
                                        padding: '3px 10px',
                                        borderRadius: '20px',
                                        fontSize: '0.75rem',
                                        fontWeight: 600,
                                        background: isOnline ? 'rgba(34, 197, 94, 0.15)' : 'rgba(239, 68, 68, 0.15)',
                                        color: isOnline ? '#22c55e' : '#ef4444',
                                        border: `1px solid ${isOnline ? '#22c55e40' : '#ef444440'}`,
                                    }}
                                >
                                    <span
                                        style={{
                                            width: '6px',
                                            height: '6px',
                                            borderRadius: '50%',
                                            background: isOnline ? '#22c55e' : '#ef4444',
                                        }}
                                    />
                                    {isOnline ? 'Online' : 'Offline'}
                                </span>
                            </div>

                            {/* Node Hardware Specifications */}
                            <div
                                style={{
                                    display: 'grid',
                                    gridTemplateColumns: '1fr 1fr',
                                    gap: '0.6rem',
                                    padding: '0.8rem',
                                    borderRadius: '8px',
                                    background: 'rgba(0,0,0,0.2)',
                                    fontSize: '0.8rem',
                                }}
                            >
                                <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', color: 'var(--text-secondary)' }}>
                                    <FaMicrochip /> CPU: {specs ? `${specs.cpu_cores} Cores` : 'N/A'}
                                </div>
                                <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', color: 'var(--text-secondary)' }}>
                                    <FaMemory /> RAM: {specs ? formatBytes(specs.total_memory) : 'N/A'}
                                </div>
                                <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', color: 'var(--text-secondary)' }}>
                                    <FaDesktop /> OS: {specs ? `${specs.os_name} ${specs.os_version}` : 'Linux'}
                                </div>
                                <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem', color: 'var(--text-secondary)' }}>
                                    <FaHeartbeat /> Ping: {formatHeartbeat(node.last_heartbeat)}
                                </div>
                            </div>

                            {/* Actions */}
                            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginTop: 'auto', paddingTop: '0.5rem' }}>
                                <button
                                    className="btn-icon danger"
                                    onClick={() => setNodeToDelete(node)}
                                    title="Remove node from cluster"
                                    style={{ padding: '0.5rem', color: '#ef4444' }}
                                >
                                    <FaTrash />
                                </button>
                                <button
                                    className={isActive ? 'btn-primary' : 'btn-secondary'}
                                    onClick={() => {
                                        setActiveNode(node);
                                        if (onSelectNodeAndNavigate) onSelectNodeAndNavigate(node);
                                    }}
                                    style={{ padding: '0.45rem 1.1rem', fontSize: '0.85rem' }}
                                >
                                    {isActive ? 'Currently Selected' : 'Manage Node'}
                                </button>
                            </div>
                        </div>
                    );
                })}
            </div>

            {/* Modal: Add New Node Wizard */}
            {isAddModalOpen && (
                <div className="modal-overlay">
                    <div className="modal glass-panel" style={{ maxWidth: '640px', width: '90%', padding: '2rem', borderRadius: '16px' }}>
                        <h3 style={{ fontSize: '1.4rem', fontWeight: 700, margin: '0 0 0.5rem 0', display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                            <FaServer style={{ color: 'var(--accent-color)' }} />
                            Add Cluster Node (Agent Mode)
                        </h3>
                        <p style={{ color: 'var(--text-secondary)', fontSize: '0.88rem', margin: '0 0 1.5rem 0', lineHeight: 1.5 }}>
                            Nodes connect outbound to this panel via encrypted WebSocket tunnels. No inbound ports or firewall configurations are required on the worker node.
                        </p>

                        {isGenerating ? (
                            <div style={{ textAlign: 'center', padding: '2rem 0' }}>
                                <div className="spinner" style={{ margin: '0 auto 1rem auto' }} />
                                <span>Generating cryptographic join token...</span>
                            </div>
                        ) : joinData ? (
                            <div style={{ display: 'flex', flexDirection: 'column', gap: '1.2rem' }}>
                                {/* One-line run command */}
                                <div>
                                    <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '0.4rem' }}>
                                        <label style={{ fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-primary)' }}>
                                            Step 1: Run on Remote Node
                                        </label>
                                        <span style={{ fontSize: '0.75rem', color: 'var(--accent-color)' }}>Headless Mode</span>
                                    </div>
                                    <div
                                        style={{
                                            position: 'relative',
                                            background: '#090d16',
                                            padding: '0.8rem 1rem',
                                            borderRadius: '8px',
                                            fontFamily: 'monospace',
                                            fontSize: '0.82rem',
                                            color: '#38bdf8',
                                            border: '1px solid rgba(255,255,255,0.1)',
                                            wordBreak: 'break-all',
                                        }}
                                    >
                                        <code>{joinData.command}</code>
                                        <button
                                            className="btn-icon"
                                            onClick={() => handleCopy(joinData.command, 'command')}
                                            title="Copy command"
                                            style={{
                                                position: 'absolute',
                                                right: '8px',
                                                top: '8px',
                                                background: 'rgba(255,255,255,0.1)',
                                                border: 'none',
                                                padding: '6px',
                                                borderRadius: '6px',
                                                cursor: 'pointer',
                                                color: '#fff',
                                            }}
                                        >
                                            {copiedCommand ? <FaCheck style={{ color: '#22c55e' }} /> : <FaCopy />}
                                        </button>
                                    </div>
                                </div>

                                {/* Join Token string */}
                                <div>
                                    <label style={{ fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-primary)', display: 'block', marginBottom: '0.4rem' }}>
                                        Join Token
                                    </label>
                                    <div
                                        style={{
                                            display: 'flex',
                                            alignItems: 'center',
                                            gap: '0.5rem',
                                            background: '#090d16',
                                            padding: '0.6rem 0.8rem',
                                            borderRadius: '8px',
                                            border: '1px solid rgba(255,255,255,0.1)',
                                        }}
                                    >
                                        <code style={{ fontSize: '0.85rem', color: '#f8fafc', flex: 1, overflow: 'hidden', textOverflow: 'ellipsis' }}>
                                            {joinData.token}
                                        </code>
                                        <button
                                            className="btn-icon"
                                            onClick={() => handleCopy(joinData.token, 'token')}
                                            style={{ padding: '4px', background: 'transparent', border: 'none', color: '#fff', cursor: 'pointer' }}
                                        >
                                            {copiedToken ? <FaCheck style={{ color: '#22c55e' }} /> : <FaCopy />}
                                        </button>
                                    </div>
                                    <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', display: 'block', marginTop: '0.3rem' }}>
                                        Valid for 1 hour. Token will be consumed on first connection.
                                    </span>
                                </div>
                            </div>
                        ) : null}

                        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.8rem', marginTop: '1.8rem' }}>
                            <button className="btn-primary" onClick={() => setIsAddModalOpen(false)}>
                                Done
                            </button>
                        </div>
                    </div>
                </div>
            )}

            {/* Modal: Delete Node Confirmation */}
            {nodeToDelete && (
                <div className="modal-overlay">
                    <div className="modal glass-panel" style={{ maxWidth: '440px', width: '90%', padding: '1.8rem', borderRadius: '16px' }}>
                        <h3 style={{ fontSize: '1.2rem', fontWeight: 700, margin: '0 0 0.8rem 0', color: '#ef4444' }}>
                            Remove Node from Cluster?
                        </h3>
                        <p style={{ color: 'var(--text-secondary)', fontSize: '0.9rem', lineHeight: 1.5, margin: '0 0 1.5rem 0' }}>
                            Are you sure you want to remove <strong>{nodeToDelete.name}</strong> ({nodeToDelete.hostname})? Its active WebSocket tunnel will be terminated and it will no longer be managed by this Hub.
                        </p>
                        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '0.8rem' }}>
                            <button className="btn-secondary" onClick={() => setNodeToDelete(null)} disabled={isDeleting}>
                                Cancel
                            </button>
                            <button className="btn-primary danger" onClick={confirmDelete} disabled={isDeleting}>
                                {isDeleting ? 'Removing...' : 'Remove Node'}
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}
