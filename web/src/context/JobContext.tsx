/* eslint-disable react-refresh/only-export-components */
import React, { createContext, useContext, useState, useEffect, useCallback } from 'react';
import type { Job, JobSummary } from '../types';

interface JobContextType {
    activeJobId: string | null;
    activeJobTitle: string;
    isModalOpen: boolean;
    trackJob: (jobId: string, title?: string) => void;
    openJobModal: (jobId: string, title?: string) => void;
    closeJobModal: () => void;
    clearActiveJob: () => void;
    recentJobs: JobSummary[];
    refreshJobs: () => Promise<void>;
}

const JobContext = createContext<JobContextType | undefined>(undefined);

export function JobProvider({ children }: { children: React.ReactNode }) {
    const [activeJobId, setActiveJobId] = useState<string | null>(() => {
        return localStorage.getItem('wadm_active_job_id');
    });
    const [activeJobTitle, setActiveJobTitle] = useState<string>(() => {
        return localStorage.getItem('wadm_active_job_title') || 'Background Operation';
    });
    const [isModalOpen, setIsModalOpen] = useState<boolean>(false);
    const [recentJobs, setRecentJobs] = useState<JobSummary[]>([]);

    const refreshJobs = useCallback(async () => {
        try {
            const res = await fetch('/api/jobs');
            if (res.ok) {
                const data: JobSummary[] = await res.json();
                setRecentJobs(data);
            }
        } catch (err) {
            console.error('Failed to fetch jobs list:', err);
        }
    }, []);

    useEffect(() => {
        let isCancelled = false;
        const loadInitial = async () => {
            try {
                const res = await fetch('/api/jobs');
                if (res.ok && !isCancelled) {
                    const data: JobSummary[] = await res.json();
                    setRecentJobs(data);
                }
            } catch (err) {
                console.error('Failed to fetch initial jobs list:', err);
            }
        };

        loadInitial();
        return () => {
            isCancelled = true;
        };
    }, []);

    const trackJob = useCallback((jobId: string, title?: string) => {
        const resolvedTitle = title || 'Background Operation';
        setActiveJobId(jobId);
        setActiveJobTitle(resolvedTitle);
        setIsModalOpen(true);
        localStorage.setItem('wadm_active_job_id', jobId);
        localStorage.setItem('wadm_active_job_title', resolvedTitle);
        refreshJobs();
    }, [refreshJobs]);

    const openJobModal = useCallback((jobId: string, title?: string) => {
        setActiveJobId(jobId);
        if (title) {
            setActiveJobTitle(title);
        }
        setIsModalOpen(true);
    }, []);

    const closeJobModal = useCallback(() => {
        setIsModalOpen(false);
    }, []);

    const clearActiveJob = useCallback(() => {
        setActiveJobId(null);
        setIsModalOpen(false);
        localStorage.removeItem('wadm_active_job_id');
        localStorage.removeItem('wadm_active_job_title');
    }, []);

    // Check status of restored active job on mount
    useEffect(() => {
        if (!activeJobId) return;

        fetch(`/api/jobs/${activeJobId}`)
            .then(res => res.ok ? res.json() : null)
            .then((job: Job | null) => {
                if (job) {
                    if (job.status === 'completed' || job.status === 'failed') {
                        // Job already finished, but keep for user review until cleared
                    }
                } else {
                    // Not found, clear it
                    clearActiveJob();
                }
            })
            .catch(() => {
                // Ignore initial network fetch errors
            });
    }, [activeJobId, clearActiveJob]);

    return (
        <JobContext.Provider
            value={{
                activeJobId,
                activeJobTitle,
                isModalOpen,
                trackJob,
                openJobModal,
                closeJobModal,
                clearActiveJob,
                recentJobs,
                refreshJobs,
            }}
        >
            {children}
        </JobContext.Provider>
    );
}

export function useJobs() {
    const context = useContext(JobContext);
    if (!context) {
        throw new Error('useJobs must be used within a JobProvider');
    }
    return context;
}

export function getReadableJobTitle(jobType: string): string {
    switch (jobType) {
        case 'package_upgrade':
            return 'System Packages Upgrade';
        case 'db_backup':
            return 'Database Backup';
        case 'app_install':
            return 'App Store Deployment';
        default:
            return jobType.replace(/_/g, ' ').toUpperCase();
    }
}
