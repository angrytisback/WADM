import { useState, useEffect, useRef, useCallback } from 'react';
import { useJobs } from '../context/JobContext';
import { useAuth } from '../context/AuthContext';
import type { JobStatus, JobLogEvent } from '../types';
import {
    FaTerminal,
    FaCheckCircle,
    FaTimesCircle,
    FaWindowMinimize,
    FaTimes,
    FaCopy,
    FaCheck,
    FaSyncAlt,
    FaChevronUp
} from 'react-icons/fa';

export function JobProgressModal() {
    const {
        activeJobId,
        activeJobTitle,
        isModalOpen,
        openJobModal,
        closeJobModal,
        clearActiveJob,
        refreshJobs
    } = useJobs();
    const { token } = useAuth();

    const [logs, setLogs] = useState<string[]>([]);
    const [status, setStatus] = useState<JobStatus>('pending');
    const [progress, setProgress] = useState<number>(0);
    const [copied, setCopied] = useState(false);
    const [autoScroll, setAutoScroll] = useState(true);

    const logContainerRef = useRef<HTMLDivElement>(null);

    // Reset logs and state when activeJobId changes
    useEffect(() => {
        setLogs([]);
        setStatus('pending');
        setProgress(0);
    }, [activeJobId]);

    // Stream SSE events for active job
    useEffect(() => {
        if (!activeJobId) return;

        let isCancelled = false;
        const abortController = new AbortController();

        const connectStream = async () => {
            try {
                const response = await fetch(`/api/jobs/${activeJobId}/stream`, {
                    headers: {
                        'Accept': 'text/event-stream',
                        ...(token ? { 'Authorization': `Bearer ${token}` } : {})
                    },
                    credentials: 'include',
                    signal: abortController.signal
                });

                if (!response.ok) {
                    throw new Error(`SSE stream failed with HTTP ${response.status}`);
                }

                if (!response.body) {
                    throw new Error('Response body is null');
                }

                const reader = response.body.getReader();
                const decoder = new TextDecoder('utf-8');
                let buffer = '';

                while (!isCancelled) {
                    const { done, value } = await reader.read();
                    if (done) break;

                    buffer += decoder.decode(value, { stream: true });
                    const events = buffer.split('\n\n');
                    buffer = events.pop() || '';

                    for (const event of events) {
                        const lines = event.split('\n');
                        for (const line of lines) {
                            if (line.startsWith('data:')) {
                                const jsonStr = line.replace(/^data:\s*/, '').trim();
                                if (jsonStr) {
                                    try {
                                        const payload: JobLogEvent = JSON.parse(jsonStr);
                                        if (payload.status) {
                                            setStatus(payload.status);
                                        }
                                        if (payload.progress !== undefined && payload.progress >= 0) {
                                            setProgress(payload.progress);
                                        }

                                        if (payload.line) {
                                            setLogs(prev => [...prev, payload.line]);
                                        }

                                        if (payload.event_type === 'complete' || payload.event_type === 'error') {
                                            refreshJobs();
                                        }
                                    } catch (err) {
                                        console.error('Failed to parse JobLogEvent JSON:', err);
                                    }
                                }
                            }
                        }
                    }
                }
            } catch (err: unknown) {
                if (isCancelled) return;
                if (err instanceof DOMException && err.name === 'AbortError') return;

                console.warn('Job stream connection closed or error:', err);
            }
        };

        connectStream();

        return () => {
            isCancelled = true;
            abortController.abort();
        };
    }, [activeJobId, token, refreshJobs]);

    // Handle auto-scroll
    useEffect(() => {
        if (autoScroll && logContainerRef.current) {
            logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
        }
    }, [logs, autoScroll]);

    const handleCopyLogs = useCallback(() => {
        const text = logs.join('\n');
        navigator.clipboard.writeText(text).then(() => {
            setCopied(true);
            setTimeout(() => setCopied(false), 2000);
        });
    }, [logs]);

    if (!activeJobId) {
        return null;
    }

    // Minimized Floating Widget
    if (!isModalOpen) {
        const isRunning = status === 'pending' || status === 'running';
        const isSuccess = status === 'completed';
        const isFailed = status === 'failed';

        return (
            <div
                style={{
                    position: 'fixed',
                    bottom: '24px',
                    right: '24px',
                    zIndex: 2500,
                    cursor: 'pointer',
                    animation: 'fadeIn 0.3s ease-out'
                }}
            >
                <div
                    className="glass-panel"
                    onClick={() => openJobModal(activeJobId)}
                    style={{
                        padding: '0.75rem 1.25rem',
                        display: 'flex',
                        alignItems: 'center',
                        gap: '0.85rem',
                        boxShadow: '0 8px 30px rgba(0,0,0,0.5)',
                        border: isRunning
                            ? '1px solid var(--accent-glow)'
                            : isSuccess
                                ? '1px solid rgba(52, 211, 153, 0.4)'
                                : '1px solid rgba(248, 113, 113, 0.4)',
                        background: 'rgba(15, 23, 42, 0.85)',
                        borderRadius: '30px',
                        backdropFilter: 'blur(12px)'
                    }}
                >
                    {isRunning && (
                        <FaSyncAlt
                            className="spin"
                            style={{ color: 'var(--accent-color)', fontSize: '1rem', flexShrink: 0 }}
                        />
                    )}
                    {isSuccess && (
                        <FaCheckCircle
                            style={{ color: 'var(--success)', fontSize: '1.1rem', flexShrink: 0 }}
                        />
                    )}
                    {isFailed && (
                        <FaTimesCircle
                            style={{ color: 'var(--danger)', fontSize: '1.1rem', flexShrink: 0 }}
                        />
                    )}

                    <div style={{ display: 'flex', flexDirection: 'column' }}>
                        <span style={{ fontSize: '0.85rem', fontWeight: 600, color: 'var(--text-primary)' }}>
                            {activeJobTitle}
                        </span>
                        <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>
                            {isRunning ? `In progress (${progress}%)` : isSuccess ? 'Completed successfully' : 'Failed'}
                        </span>
                    </div>

                    <button
                        className="btn-text"
                        onClick={(e) => {
                            e.stopPropagation();
                            openJobModal(activeJobId);
                        }}
                        title="Expand console"
                        style={{
                            padding: '0.2rem',
                            color: 'var(--text-secondary)',
                            display: 'flex',
                            alignItems: 'center'
                        }}
                    >
                        <FaChevronUp />
                    </button>

                    {!isRunning && (
                        <button
                            className="btn-text"
                            onClick={(e) => {
                                e.stopPropagation();
                                clearActiveJob();
                            }}
                            title="Dismiss"
                            style={{
                                padding: '0.2rem',
                                color: 'var(--text-secondary)',
                                display: 'flex',
                                alignItems: 'center'
                            }}
                        >
                            <FaTimes />
                        </button>
                    )}
                </div>
            </div>
        );
    }

    // Full Modal
    const isRunning = status === 'pending' || status === 'running';

    const getStatusBadge = () => {
        switch (status) {
            case 'pending':
                return (
                    <span className="badge warning" style={{ display: 'inline-flex', alignItems: 'center', gap: '0.35rem' }}>
                        <span className="spinner" style={{ width: '0.65rem', height: '0.65rem' }}></span>
                        Queued
                    </span>
                );
            case 'running':
                return (
                    <span
                        className="badge"
                        style={{
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: '0.35rem',
                            background: 'rgba(56, 189, 248, 0.2)',
                            color: 'var(--accent-color)',
                            border: '1px solid rgba(56, 189, 248, 0.3)'
                        }}
                    >
                        <span className="spinner" style={{ width: '0.65rem', height: '0.65rem' }}></span>
                        Running ({progress}%)
                    </span>
                );
            case 'completed':
                return (
                    <span className="badge success" style={{ display: 'inline-flex', alignItems: 'center', gap: '0.35rem' }}>
                        <FaCheckCircle style={{ fontSize: '0.75rem' }} />
                        Completed
                    </span>
                );
            case 'failed':
                return (
                    <span
                        className="badge"
                        style={{
                            display: 'inline-flex',
                            alignItems: 'center',
                            gap: '0.35rem',
                            background: 'rgba(248, 113, 113, 0.2)',
                            color: 'var(--danger)',
                            border: '1px solid rgba(248, 113, 113, 0.3)'
                        }}
                    >
                        <FaTimesCircle style={{ fontSize: '0.75rem' }} />
                        Failed
                    </span>
                );
        }
    };

    return (
        <div
            style={{
                position: 'fixed',
                top: 0,
                left: 0,
                right: 0,
                bottom: 0,
                background: 'rgba(0, 0, 0, 0.75)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                zIndex: 2500,
                backdropFilter: 'blur(8px)',
                padding: '1.5rem'
            }}
        >
            <div
                className="glass-panel"
                style={{
                    width: '780px',
                    maxWidth: '100%',
                    height: '580px',
                    maxHeight: '90vh',
                    display: 'flex',
                    flexDirection: 'column',
                    padding: '1.5rem',
                    position: 'relative',
                    boxShadow: '0 20px 50px rgba(0, 0, 0, 0.6)'
                }}
            >
                {/* Modal Header */}
                <div
                    style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        paddingBottom: '1rem',
                        borderBottom: '1px solid var(--glass-border)'
                    }}
                >
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                        <div
                            style={{
                                padding: '0.5rem',
                                background: 'rgba(56, 189, 248, 0.15)',
                                borderRadius: '8px',
                                color: 'var(--accent-color)',
                                display: 'flex',
                                alignItems: 'center'
                            }}
                        >
                            <FaTerminal style={{ fontSize: '1.1rem' }} />
                        </div>
                        <div>
                            <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                                <h3 style={{ margin: 0, fontSize: '1.2rem', color: 'var(--text-primary)' }}>
                                    {activeJobTitle}
                                </h3>
                                {getStatusBadge()}
                            </div>
                            <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>
                                Job ID: <code style={{ color: 'var(--accent-color)' }}>{activeJobId}</code>
                            </span>
                        </div>
                    </div>

                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <button
                            className="btn-text"
                            onClick={closeJobModal}
                            title="Minimize to background"
                            style={{
                                padding: '0.5rem',
                                color: 'var(--text-secondary)',
                                display: 'flex',
                                alignItems: 'center',
                                gap: '0.35rem',
                                fontSize: '0.85rem'
                            }}
                        >
                            <FaWindowMinimize style={{ fontSize: '0.85rem' }} />
                            <span>Minimize</span>
                        </button>
                        <button
                            className="btn-text"
                            onClick={clearActiveJob}
                            title={isRunning ? "Cancel tracking" : "Close"}
                            style={{
                                padding: '0.5rem',
                                color: 'var(--text-secondary)',
                                display: 'flex',
                                alignItems: 'center'
                            }}
                        >
                            <FaTimes style={{ fontSize: '1.1rem' }} />
                        </button>
                    </div>
                </div>

                {/* Progress Bar */}
                <div
                    style={{
                        marginTop: '1rem',
                        marginBottom: '0.75rem',
                        background: 'rgba(255, 255, 255, 0.05)',
                        borderRadius: '4px',
                        height: '6px',
                        overflow: 'hidden',
                        position: 'relative'
                    }}
                >
                    <div
                        style={{
                            height: '100%',
                            width: `${Math.max(0, Math.min(progress, 100))}%`,
                            background: status === 'failed'
                                ? 'var(--danger)'
                                : status === 'completed'
                                    ? 'var(--success)'
                                    : 'linear-gradient(90deg, var(--accent-color), #818cf8)',
                            transition: 'width 0.4s ease-in-out'
                        }}
                    />
                </div>

                {/* Terminal Console View */}
                <div
                    ref={logContainerRef}
                    onScroll={(e) => {
                        const target = e.currentTarget;
                        const isAtBottom = target.scrollHeight - target.scrollTop <= target.clientHeight + 40;
                        setAutoScroll(isAtBottom);
                    }}
                    style={{
                        flex: 1,
                        background: '#070b14',
                        borderRadius: '8px',
                        border: '1px solid rgba(255, 255, 255, 0.08)',
                        padding: '1rem',
                        overflowY: 'auto',
                        fontFamily: 'monospace, "Fira Code", Courier, sans-serif',
                        fontSize: '0.82rem',
                        lineHeight: '1.5',
                        color: '#e2e8f0',
                        whiteSpace: 'pre-wrap',
                        wordBreak: 'break-word',
                        boxShadow: 'inset 0 2px 10px rgba(0,0,0,0.5)'
                    }}
                >
                    {logs.length === 0 ? (
                        <div style={{ color: 'var(--text-secondary)', fontStyle: 'italic', padding: '0.5rem 0' }}>
                            Waiting for output stream...
                        </div>
                    ) : (
                        logs.map((line, idx) => (
                            <div
                                key={idx}
                                style={{
                                    display: 'flex',
                                    gap: '0.75rem',
                                    color: line.startsWith('---')
                                        ? 'var(--accent-color)'
                                        : line.startsWith('$')
                                            ? '#38bdf8'
                                            : line.toLowerCase().includes('error') || line.toLowerCase().includes('failed')
                                                ? '#f87171'
                                                : line.toLowerCase().includes('warning')
                                                    ? '#fbbf24'
                                                    : '#e2e8f0'
                                }}
                            >
                                <span style={{ color: '#475569', userSelect: 'none', minWidth: '2.5rem', textAlign: 'right' }}>
                                    {idx + 1}
                                </span>
                                <span style={{ flex: 1 }}>{line}</span>
                            </div>
                        ))
                    )}
                </div>

                {/* Footer Controls */}
                <div
                    style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                        marginTop: '1rem',
                        paddingTop: '0.5rem'
                    }}
                >
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                        <button
                            className="btn-sm"
                            onClick={handleCopyLogs}
                            disabled={logs.length === 0}
                            style={{
                                display: 'inline-flex',
                                alignItems: 'center',
                                gap: '0.4rem',
                                fontSize: '0.8rem',
                                padding: '0.4rem 0.8rem'
                            }}
                        >
                            {copied ? <FaCheck style={{ color: 'var(--success)' }} /> : <FaCopy />}
                            <span>{copied ? 'Copied' : 'Copy Logs'}</span>
                        </button>

                        <button
                            className="btn-sm"
                            onClick={() => {
                                setAutoScroll(true);
                                if (logContainerRef.current) {
                                    logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
                                }
                            }}
                            style={{
                                display: 'inline-flex',
                                alignItems: 'center',
                                gap: '0.4rem',
                                fontSize: '0.8rem',
                                padding: '0.4rem 0.8rem',
                                opacity: autoScroll ? 1 : 0.6
                            }}
                        >
                            <span>{autoScroll ? 'Auto-scroll On' : 'Scroll to Bottom'}</span>
                        </button>
                    </div>

                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.75rem' }}>
                        <button
                            className="btn-text"
                            onClick={closeJobModal}
                            style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}
                        >
                            Run in Background
                        </button>

                        <button
                            className="btn-primary"
                            onClick={clearActiveJob}
                            disabled={isRunning}
                            style={{
                                padding: '0.45rem 1.25rem',
                                fontSize: '0.85rem',
                                opacity: isRunning ? 0.5 : 1,
                                cursor: isRunning ? 'not-allowed' : 'pointer'
                            }}
                        >
                            {isRunning ? 'Running...' : 'Done'}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}
