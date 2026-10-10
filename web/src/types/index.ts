export interface GpuStats {
    load: number;
    vram_used: number;
    vram_total: number;
    temp: number;
    name: string;
    vendor: string;
    pci_id: string;
    error?: string;
}

export interface SystemStats {
    cpu_usage: number;
    ram_total: number;
    ram_used: number;
    swap_total: number;
    swap_used: number;
    disk_total: number;
    disk_used: number;
    network_rx: number;
    network_tx: number;
    active_services: number;
    failed_services: number;
    active_containers: number;
    upgradable_packages: number;
    network_interface: string;
    network_max_speed: number;
    gpus: GpuStats[];
    cpu_temp: number;
}

export interface SystemInfo {
    os_name: string;
    os_version: string;
    kernel_version: string;
    host_name: string;
    uptime: number;
    cpu_arch: string;
    cpu_count: number;
    total_memory: number;
    used_memory: number;
    total_swap: number;
    used_swap: number;
    username: string;
    has_sudo: boolean;
    is_root: boolean;
    cpu_temp?: number;
    gpu_temp?: number;
    gpus: GpuStats[];
    smart?: {
        name: string;
        model: string;
        serial: string;
        health: string;
        temperature: number | null;
        power_on_hours: number | null;
    }[];
}

export interface Dependency {
    name: string;
    command: string;
    installed: boolean;
    optional: boolean;
    install_hint?: string;
}

export interface DependencyReport {
    dependencies: Dependency[];
    critical_missing: boolean;
}

export type JobStatus = 'pending' | 'running' | 'completed' | 'failed';

export interface Job {
    id: string;
    job_type: string;
    status: JobStatus;
    progress: number;
    logs: string;
    error_message?: string | null;
    created_at: string;
    updated_at: string;
}

export interface JobSummary {
    id: string;
    job_type: string;
    status: JobStatus;
    progress: number;
    error_message?: string | null;
    created_at: string;
    updated_at: string;
}

export interface JobActionResponse {
    job_id: string;
    status: string;
    message: string;
    app_id?: string;
    password?: string;
}

export interface JobLogEvent {
    job_id: string;
    event_type: 'log' | 'status' | 'complete' | 'error';
    line: string;
    status: JobStatus;
    progress: number;
    timestamp: string;
}

export type PluginEntryType = 'declarative_schema' | 'iframe' | 'custom_view';

export interface PluginUiConfig {
    tab_id: string;
    title: string;
    icon: string;
    entry_type: PluginEntryType;
}

export interface PluginManifest {
    id: string;
    name: string;
    version: string;
    author: string;
    description: string;
    executable: string;
    ui: PluginUiConfig;
    capabilities: string[];
}

export type PluginState = 'installed' | 'running' | 'stopped' | 'crashed';

export interface PluginRuntimeInfo {
    manifest: PluginManifest;
    state: PluginState;
    pid?: number | null;
    socket_path?: string | null;
    started_at?: string | null;
    error?: string | null;
}

export interface PluginRpcResponse<T = unknown> {
    jsonrpc: string;
    id: number | string | null;
    result?: T;
    error?: {
        code: number;
        message: string;
        data?: unknown;
    };
}

export interface PluginDownloadAsset {
    url: string;
    sha256: string;
}

export interface PluginStoreCatalogItem {
    id: string;
    name: string;
    description: string;
    version: string;
    author: string;
    category: string;
    icon: string;
    homepage?: string | null;
    repository: string;
    capabilities: string[];
    downloads: Record<string, PluginDownloadAsset>;
}

export interface PluginStoreItemView {
    id: string;
    name: string;
    description: string;
    version: string;
    author: string;
    category: string;
    icon: string;
    homepage?: string | null;
    repository: string;
    capabilities: string[];
    downloads: Record<string, PluginDownloadAsset>;
    is_installed: boolean;
    installed_version?: string | null;
    has_update: boolean;
}

export type UserRole = 'viewer' | 'operator' | 'admin';

export interface User {
    id: number;
    username: string;
    role: UserRole;
    created_at: string;
}

export interface AuditLog {
    id: number;
    timestamp: string;
    username: string;
    role: string;
    action: string;
    resource?: string | null;
    ip_address: string;
    status: 'SUCCESS' | 'FAILED' | 'DENIED' | string;
    details?: string | null;
}

export interface AuditLogPage {
    logs: AuditLog[];
    total: number;
    page: number;
    limit: number;
    total_pages: number;
}

export type SslMode = 'self_signed' | 'custom_cert' | 'lets_encrypt';

export interface SslConfig {
    enabled: boolean;
    mode: SslMode;
    domain?: string | null;
    email?: string | null;
    cert_path?: string | null;
    key_path?: string | null;
    auto_renew: boolean;
    force_https: boolean;
    https_port: number;
    http_port: number;
    expires_at?: number | null;
    issuer?: string | null;
    updated_at: string;
}

export interface SslStatusResponse {
    config: SslConfig;
    cert_exists: boolean;
    days_left?: number | null;
    is_expired: boolean;
}

export interface NodeSpecsData {
    cpu_cores: number;
    total_memory: number;
    os_name: string;
    os_version: string;
    kernel_version: string;
    architecture: string;
}

export interface ClusterNode {
    id: string;
    name: string;
    hostname: string;
    ip_address: string;
    status: 'online' | 'offline' | 'pending';
    version: string;
    specs: string;
    last_heartbeat: string | null;
    created_at: string;
}

export interface JoinTokenResponse {
    token: string;
    command: string;
    expires_at: string;
}

export interface DirectPortRequirement {
    port: number;
    protocol: string;
    reason: string;
}

export interface AppPortConfig {
    internal_web_port?: number | null;
    exposed_network_ports: DirectPortRequirement[];
}

export interface AppTemplate {
    id: string;
    name: string;
    description: string;
    icon: string;
    ports: AppPortConfig;
    default_access_mode: 'path' | 'subdomain';
}

export interface AppInstallRequest {
    id: string;
    access_mode: 'path' | 'subdomain';
    domain?: string;
    allow_exposed_ports: boolean;
    approved_ports: number[];
}

export interface AppCredentialsResponse {
    id: string;
    credentials?: string | null;
    installed: boolean;
    access_url?: string | null;
    access_mode?: string | null;
    internal_port?: number | null;
}

export interface ProxyRoute {
    id: string;
    app_id: string;
    domain?: string | null;
    path_prefix?: string | null;
    target_url: string;
    websocket_support: boolean;
    created_at: string;
}


