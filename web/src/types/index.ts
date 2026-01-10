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
}
