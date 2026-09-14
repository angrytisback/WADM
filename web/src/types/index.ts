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
