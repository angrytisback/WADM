# WADM (Web Administration for Linux) - Features & Roadmap Specification

> **Version Reference:** v0.96.0 | **Architecture Status:** Stable (Production-Ready) | **Last Updated:** October 2026

This document is the primary technical specification detailing all current capabilities, security controls, architectural subsystems, and forward roadmap targets for the WADM server control plane.

---

## 1. Production Architecture & Implemented Capabilities

WADM is engineered as a zero-external-runtime Linux server control plane. It combines a compiled **Rust (Actix-Web 4 & Tokio)** backend with an embedded reactive **React 19 / TypeScript (Vite 7)** single-page application (SPA).

### A. Real-Time Telemetry & Hardware Monitoring
* **Core & Package Telemetry:**
  * Per-core and total CPU utilization, clock frequencies, and package temperatures sourced directly from `sysinfo` and `/sys/class/hwmon`.
  * Physical RAM and Swap memory utilization (Total, Used, Free, Cached, Available).
  * Disk partition mapping, filesystem type, mount points, capacity, and usage tracking with Recharts visualization.
* **Network Telemetry:**
  * Live RX/TX throughput rates (KB/s and MB/s) with 60-point historical telemetry charts.
  * Automatic active network interface detection and link speed negotiation queries.
* **Multi-Vendor GPU Support:**
  * **NVIDIA:** `nvidia-smi` CSV queries for VRAM usage, GPU core load, and thermal sensors.
  * **AMD:** Direct `/sys/class/drm` and sysfs counters (`gpu_busy_percent`, VRAM usage, and temperatures).
  * **Intel:** `intel_gpu_top` JSON output and sysfs frequency ratio fallback (`gt_act_freq_mhz / gt_max_freq_mhz`).
  * Hardware PCI enumeration via `/sys/bus/pci/devices` for unconfigured or headless adapters.
* **Process Management:**
  * System process table inspection sorted by CPU and memory consumption.
  * Process lifecycle signals (`SIGTERM` and `SIGKILL`).
  * **Kernel & Host Guardrails:** Explicit immunity protections for PID 1 (system init) and the WADM server's own process ID.

### B. Package & Dependency Management
* **Multi-Distribution Package Drivers:**
  * APT (Debian, Ubuntu, Linux Mint, Pop!_OS)
  * DNF (Fedora, RHEL, AlmaLinux, Rocky Linux)
  * Pacman (Arch Linux, Manjaro, EndeavourOS)
* **Lifecycle Operations:**
  * Package indexing, searching, and filtering.
  * Single package upgrades and batch system updates (`update-all`).
  * Dependency-previewed safe uninstallation (`remove-dry-run` and `remove`).
  * Asynchronous upgradable count caching (`UPGRADABLE_CACHE`) with a 5-minute TTL to eliminate fork-bomb load on regular telemetry polls.
* **System Dependency Health Checker:**
  * Verification of host CLI utilities (`smartctl`, `ufw`, `fstrim`, `sensors`, `speedtest-cli`, `docker`, `mysql`, `psql`).
  * Single-click automated installation of missing prerequisites via the detected package manager driver.

### C. Systemd Service Control Plane
* **Unit Lifecycle:**
  * Real-time query of active, inactive, failed, and dead systemd services via `systemctl`.
  * State management operations: `start`, `stop`, `restart`, `enable`, and `disable`.
  * Strict service name sanitization rejecting shell meta-characters and argument injection attacks.
* **Live Journal Logs:**
  * Direct extraction of unit logs via `journalctl -u <service> -n 100 --no-pager`.

### D. Docker Container Orchestration & Zero-Port-Exposure App Store
* **Bollard Async Client Integration:**
  * Native non-blocking communication with the Docker Engine over `/var/run/docker.sock`.
  * Container lifecycle control (`start`, `stop`, `restart`, `remove`) and real-time CPU/memory consumption.
* **Zero Port Exposure Architecture:**
  * Installed application containers (Nextcloud, Jellyfin, Vaultwarden, Pi-hole, WordPress, Nginx Proxy Manager, Portainer) bind strictly to `127.0.0.1:<internal_port>` or isolated Docker bridge networks. Zero external public port binding (`0.0.0.0:PORT`) is allowed.
* **Internal Reverse Proxy Routing:**
  * All container web interfaces are served through WADM's primary HTTP/HTTPS entrypoint via:
    * **Subpath Routing:** `https://panel.example.com/apps/<app-id>/`
    * **Subdomain Routing:** `https://<app-id>.panel.example.com`
  * Bidirectional HTTP header rewriting (`Host`, `X-Forwarded-For`, `X-Forwarded-Proto`, `X-Forwarded-Prefix`) and WebSocket proxying.
* **Firewall Explicit Consent Modal:**
  * If an application defines raw non-HTTP network protocols (e.g. DNS UDP/53 for Pi-hole, WireGuard UDP/51820), WADM presents an interactive modal detailing the exact ports, protocols, and security implications, requiring explicit user authorization before updating UFW rules.
* **Cryptographic Credential Generation:**
  * Automated generation of cryptographically secure credentials (`rand::thread_rng`), persisted to `credentials.txt` with strict `0o600` permissions.

### E. Asynchronous Job Runner & Live SSE Streaming
* **JobManager Core:**
  * Long-running, I/O-intensive operations (package installations, app deployments, backups, plugin downloads) execute as asynchronous background jobs.
  * Immediate HTTP 202 Accepted response with job ID token.
* **SQLite WAL State Persistence:**
  * Job status, progress percentages, exit codes, and stdout/stderr ring buffers are persisted in `wadm.db` under write-ahead logging (WAL).
* **Server-Sent Events (SSE):**
  * Real-time log streaming at `GET /api/jobs/{id}/stream` delivering live terminal outputs directly to the UI.

### F. Out-of-Process Plugin Engine & Plugin Store
* **Unix Domain Socket & JSON-RPC 2.0:**
  * Plugins run as isolated child processes communicating with WADM exclusively over Unix Domain Sockets using strict JSON-RPC 2.0 messages.
  * Fault isolation: A plugin crash or memory leak cannot affect the stability or uptime of the main WADM process.
* **Plugin Store & Archive Security:**
  * Automated installation from remote catalogs or GitHub releases.
  * Cryptographic SHA-256 hash verification against catalog manifests prior to archive extraction.
  * Zip-Slip / Path Traversal protection: Strict rejection of archive entries containing `..`, absolute paths, or special device nodes.
* **Role-Based Access Control:**
  * Only users with the `Admin` role can install, upgrade, or uninstall plugins (`RequireAdmin` middleware).

### G. Cluster Federation & Headless Node Agents
* **Outbound Reverse WebSocket Tunneling:**
  * Worker nodes (Agents) do not expose any inbound open ports to the network or the Internet.
  * Agents establish an outbound secure WebSocket tunnel to the Central Hub at `/api/cluster/tunnel`.
  * Node-to-Hub handshake verifies UUID, hostname, secret token, and kernel metadata.
* **Transparent JSON-RPC Routing:**
  * Central Hub routes telemetry requests, container controls, and commands directly through the active tunnel to the target node.
  * Heartbeat liveness checks: Active ping/pong every 15 seconds; stale connections are flagged `offline` automatically.
* **Agent Headless Mode:**
  * Single binary runs in dedicated agent mode: `wadm --agent --hub-url <url> --token <token> --name <name>`.

### H. Native SSL / TLS & ACME Engine
* **Rustls Dual-Stack Engine:**
  * Single binary natively terminates TLS without requiring an external reverse proxy like Nginx or Caddy.
  * Dual-stack listeners: Plaintext HTTP on port 80/8168 and TLS HTTPS on port 443/8168 with automatic HTTP-to-HTTPS redirect.
* **Automated ACME Let's Encrypt:**
  * Integrated `instant-acme` client executing automated HTTP-01 challenge verification (`/.well-known/acme-challenge/*`).
  * Automatic certificate renewal when within 30 days of expiration.
* **Self-Signed Fallback:**
  * Instant generation of ECDSA P-256 self-signed certificates via `rcgen` for air-gapped or private network deployments.

### I. Multi-User RBAC & Audit Logging
* **Role Hierarchy:**
  * `Viewer`: Read-only access to system telemetry, container status, and service health.
  * `Operator`: Operational control over services, containers, memory flushing, and database backups.
  * `Admin`: Full supervisory control including user management, terminal access, plugin installation, and security configurations.
* **Immutable Audit Trail:**
  * SQLite WAL `audit_logs` table recording timestamp, username, client IP, action performed, resource targeted, and status.
  * Dedicated UI audit log explorer with filtering and CSV/JSON export.

### J. Interactive Web Terminal (Web TTY)
* **High-Performance Emulation:**
  * `@xterm/xterm` v6.x and `@xterm/addon-fit` for seamless browser rendering.
  * Backend PTY allocation via `portable-pty` bridged over Actix WebSocket (`actix-ws`).
* **Lifecycle & Resource Protection:**
  * Dynamic terminal resizing protocol (`RESIZE:colsxrows`).
  * Lazy allocation: PTY is created only upon UI connection and destroyed upon disconnect.
  * Child process reaper: Dedicated cleanup handler executes `child.kill()` and `child.wait()` to prevent zombie processes and file descriptor leaks.
  * Developer Mode authorization check: Terminal access requires explicit administrative enablement.

### K. Filesystem Explorer & Configuration Editor
* **Directory Navigation:**
  * Hierarchical tree navigation with POSIX permissions (`drwxr-xr-x`), file sizes, and modification timestamps.
* **Integrated Editor:**
  * In-browser editing and syntax saving for configuration files (.conf, .yaml, .json, .sh, .txt) up to 10 MB.
* **File Transfers:**
  * Multipart upload with filename sanitization.
  * Two-phase authenticated blob download via `Authorization: Bearer <token>`.
* **Kernel Security Guardrails:**
  * Path traversal (`..`) and null-byte (`\0`) rejection.
  * Absolute read protection on sensitive host credentials (`/etc/shadow`, `/etc/gshadow`, `/etc/sudoers`, `.wadm_jwt_secret`, `wadm-auth.json`).
  * Absolute write protection on virtual filesystems and critical system trees (`/proc`, `/sys`, `/dev`, `/boot`, `/etc/sudoers.d`).

### L. Storage Health & S.M.A.R.T. Monitoring
* **Disk Diagnostics:**
  * Physical drive inspection via `smartctl --json=c`.
  * Overall drive health assessment (Passed / Failed / Degraded).
  * Operating metrics: Power-on hours, operational temperature, reallocated sector counts.
  * NVMe wear leveling percentages and critical hardware alerts.

### M. System Maintenance & Memory Hygiene
* **RAM Flush:** Atomic invocation of `sync` followed by `/proc/sys/vm/drop_caches` (mode 3) to release PageCache, dentries, and inodes.
* **OOM-Protected Swap Evacuation:** Safe memory transfer (`swapoff -a && swapon -a`) verified against available physical RAM thresholds (`avail_kb >= swap_used + 150MB`).
* **Cache Cleanup:** Automated package manager cache purging (`apt clean`, `pacman -Sc`, `dnf clean all`) and systemd journal vacuuming (`journalctl --vacuum-time=3d`).
* **Storage Optimization:** SSD block trimming via `fstrim -av`.
* **DNS Cache Flush:** Instant DNS resolver cache clearing via `resolvectl flush-caches`.

### N. Single-Binary Distribution
* **Asset Embedding:**
  * Frontend assets (`web/dist/*`) are compiled directly into the release binary using `rust-embed`.
  * Zero external runtime dependencies: no Node.js, Python, or external web server required on production hosts.

---

## 2. System Compatibility Matrix

| Component / Feature | Debian / Ubuntu | Fedora / RHEL | Arch Linux | Support Status |
| :--- | :---: | :---: | :---: | :--- |
| **Package Manager** | APT (`apt-get`) | DNF (`dnf`) | Pacman (`pacman`) | Fully Supported |
| **Service Manager** | Systemd (`systemctl`) | Systemd (`systemctl`) | Systemd (`systemctl`) | Fully Supported |
| **Container Engine** | Docker Engine | Docker Engine / Podman* | Docker Engine | Fully Supported (*Docker socket) |
| **Databases** | MySQL / PostgreSQL | MySQL / PostgreSQL | MySQL / PostgreSQL | Native and Docker |
| **Web Terminal** | Portable-PTY (bash) | Portable-PTY (bash) | Portable-PTY (bash) | Fully Supported |
| **Storage Diagnostics**| `smartmontools` | `smartmontools` | `smartmontools` | Fully Supported |
| **Firewall Engine** | UFW | UFW / Firewalld* | UFW | Fully Supported (*UFW backend) |
| **Hardware Detection** | NVIDIA / AMD / Intel | NVIDIA / AMD / Intel | NVIDIA / AMD / Intel | Hybrid sysfs + CLI |
| **Cluster Federation** | WebSocket Tunnel | WebSocket Tunnel | WebSocket Tunnel | Fully Supported |
| **Native TLS / ACME** | Rustls + Let's Encrypt | Rustls + Let's Encrypt | Rustls + Let's Encrypt | Fully Supported |
| **Power Management** | `systemd-logind` | `systemd-logind` | `systemd-logind` | Fully Supported |