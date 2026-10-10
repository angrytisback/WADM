# WADM (Web Administration for Linux) - Architecture & Context Reference

> **Version Reference:** v0.96.0 | **Architecture Standard:** Layered Domain Driver & Reverse WebSocket Cluster | **Last Updated:** October 2026  
> **Audience:** Senior Systems Architects, Security Auditors, Systems Programmers, and SREs

---

## 1. System Overview

### 1.1. Purpose & Architectural Vision
**WADM (Web Administration for Linux)** is a lightweight, high-performance, zero-external-runtime server control plane engineered for Linux servers, virtual machines, and cloud environments. It provides real-time hardware telemetry, system-level memory and storage maintenance, multi-distribution package management, systemd service control, Docker container orchestration, an out-of-process plugin host, native TLS/ACME termination, and multi-node cluster federation.

Unlike legacy server administration tools that rely on interpreted runtimes, complex dependency chains, or high idle resource consumption, WADM is compiled directly to native machine code using **Rust (Actix-Web 4 & Tokio)**. The user interface is an embedded, reactive Single Page Application (SPA) built with **React 19, TypeScript, and Vite 7**.

### 1.2. Deployment & Execution Model
- **Single Statically-Linked Binary (`wadm`):** Interacts directly with Linux kernel virtual filesystems (`/proc`, `/sys`), systemd control interfaces, the Docker Unix socket (`/var/run/docker.sock`), and POSIX pseudo-terminals (PTYs).
- **Embedded Frontend Assets:** Compiled directly into the `wadm` binary using `rust-embed`. The binary serves static assets, API endpoints, WebSocket bridges, and reverse proxy routes from a single process.
- **Multi-Architecture Support:** Builds for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`.
- **Zero Port Exposure Policy:** Managed container applications bind exclusively to localhost or internal bridge networks. All traffic flows through WADM's integrated reverse proxy over HTTP/HTTPS.

### 1.3. Functional Capabilities
1. **Real-Time Telemetry & Hardware Inspection:**
   - **CPU:** Per-core and total utilization, operating frequency, and package thermal metrics via `sysinfo` and `/sys/class/hwmon`.
   - **Memory & Swap:** Physical RAM allocation (Used, Free, Cached, Available) and swap utilization.
   - **Storage:** Partition mapping, mount points, capacity, and usage tracking.
   - **Network I/O:** Per-second RX/TX throughput rates (KB/s, MB/s) with a rolling 60-point historical telemetry series.
   - **Multi-Vendor GPU Support:** Hardware-specific telemetry for NVIDIA (`nvidia-smi`), AMD (`sysfs/drm` counters), and Intel (`intel_gpu_top` / sysfs frequency ratio).
   - **S.M.A.R.T. Diagnostics:** Storage device health queries via `smartctl --scan --json`.

2. **System Maintenance & Memory Hygiene:**
   - **RAM Flush:** Atomic synchronization and cache dropping via `/proc/sys/vm/drop_caches` (mode 3).
   - **Package & Journal Cleanup:** Purging local package manager caches (`apt clean`, `pacman -Sc`, `dnf clean all`) and vacuuming systemd journal logs (`journalctl --vacuum-time=3d`).
   - **OOM-Guarded Swap Evacuation:** Safe swap evacuation (`swapoff -a && swapon -a`) with strict memory threshold verification (`avail_kb >= swap_used + 150MB`).
   - **SSD TRIM:** Storage wear-leveling optimization via `fstrim -av`.
   - **DNS Cache Management:** Resolver cache flushing via `resolvectl flush-caches`.

3. **Package & Service Management:**
   - **Multi-Manager Support:** Automatic driver selection for APT (Debian/Ubuntu), DNF (Fedora/RHEL), and Pacman (Arch Linux).
   - **Package Lifecycle:** Upgradable list detection, single-package upgrades, full system updates, and dependency-dry-run uninstallation.
   - **System Dependency Health:** Automated detection and one-click remediation of missing CLI tools.
   - **Systemd Control Plane:** State queries, unit lifecycle control (`start`, `stop`, `restart`, `enable`, `disable`), and real-time journal extraction.

4. **Container Orchestration & App Store:**
   - **Docker Socket Integration:** Non-blocking async communication via `bollard` over `/var/run/docker.sock`.
   - **App Store Templates:** Automated deployment of curated server templates (Nextcloud, Jellyfin, Vaultwarden, Pi-hole, WordPress, Nginx Proxy Manager, Portainer).
   - **Secure Credentials:** Cryptographic password generation saved to `credentials.txt` with mode `0o600`.

5. **Relational Database Administration:**
   - **Multi-Engine Discovery:** Automated detection of MySQL/MariaDB and PostgreSQL instances running natively or inside Docker containers.
   - **Console & Backups:** Table inspection, paginated previews, SQL console, and streaming dump/restore workflows.

6. **Filesystem Explorer & Integrated Editor:**
   - POSIX file tree navigation with file sizes, permissions, and timestamps.
   - In-browser editor for text and configuration files up to 10 MB.
   - Multipart file upload and authenticated blob downloads.
   - Strict path traversal (`..`) and sensitive directory read/write security guardrails.

7. **Interactive Web Terminal (Web TTY):**
   - VT100/Xterm terminal emulator (`@xterm/xterm` v6.x) with responsive resizing.
   - Subprotocol-authenticated WebSocket bridge (`actix-ws`) backed by `portable-pty`.
   - Child process reaper ensuring child shells are terminated on connection drop.

8. **Cluster Federation & Node Agent Engine:**
   - Outbound reverse WebSocket tunnel (`/api/cluster/tunnel`) connecting worker nodes to the central panel without inbound port exposure.
   - Central Hub request proxying and JSON-RPC dispatching.
   - Node liveness heartbeats (15s ping/pong) and automated offline status reconciliation.

9. **Out-of-Process Plugin Engine:**
   - Isolated plugin execution communicating via Unix Domain Sockets using JSON-RPC 2.0.
   - Dynamic Plugin Store with SHA-256 archive validation and Zip-Slip protection.

10. **Native TLS & ACME Let's Encrypt:**
    - Integrated Rustls dual-stack HTTP/HTTPS server with automatic HTTP-to-HTTPS redirect.
    - Automated ACME HTTP-01 challenge completion and certificate renewal via `instant-acme`.
    - Self-signed certificate fallback via `rcgen`.

---

## 2. Architecture & Subsystem Layout

### 2.1. Directory Structure

```
wadm/
├── Cargo.toml                      # Backend dependencies and build configuration
├── Dockerfile                      # Multi-stage production container build
├── build.sh                        # Multi-architecture release compilation script
├── scripts/
│   ├── install.sh                  # Production single-binary & cluster agent installer
│   └── uninstall.sh                # Clean uninstaller with optional --purge
├── systemd/
│   └── wadm.service                # Hardened systemd service definition
├── src/
│   ├── main.rs                     # Entry point, CLI arguments, dual-stack HTTP/TLS server
│   ├── middleware.rs               # Authentication, CORS, and RBAC middleware
│   ├── api/                        # REST and WebSocket route controllers
│   │   ├── apps.rs                 # App Store lifecycle and credentials
│   │   ├── audit.rs                # Audit log search and export
│   │   ├── auth.rs                 # Authentication, TOTP, and JWT signing
│   │   ├── cluster.rs              # Cluster node management and tunnel routing
│   │   ├── config.rs               # Actix-Web service configuration and route binding
│   │   ├── db.rs                   # Database discovery, query console, and backups
│   │   ├── dependencies.rs         # System CLI dependency detection and installation
│   │   ├── docker.rs               # Container inspection, stats, and lifecycle
│   │   ├── files.rs                # Filesystem explorer, uploads, and downloads
│   │   ├── firewall.rs             # UFW status and rule management
│   │   ├── jobs.rs                 # Asynchronous background job status and SSE stream
│   │   ├── logs.rs                 # System log buffer queries
│   │   ├── monitor.rs              # Real-time hardware and resource telemetry
│   │   ├── pkgmgr.rs               # Package manager operations and upgradable cache
│   │   ├── plugins.rs              # Plugin lifecycle and store endpoints
│   │   ├── services.rs             # Systemd unit management and journalctl logs
│   │   ├── ssl.rs                  # TLS certificate and ACME management
│   │   ├── system.rs               # Power sequencing, memory flush, and maintenance
│   │   ├── terminal.rs             # PTY allocation and WebSocket bridge
│   │   └── users.rs                # User management and RBAC administration
│   ├── apps/                       # App Store manifests and deployment engine
│   │   ├── manifest.rs             # App template schemas and port configurations
│   │   └── mod.rs                  # Docker Compose deployment and credential handling
│   ├── audit/                      # SQLite-backed persistent audit logging
│   │   ├── db.rs                   # Audit schema migrations and query engine
│   │   └── mod.rs                  # Audit recording dispatcher
│   ├── auth/                       # User authentication and RBAC domain logic
│   │   ├── db.rs                   # User credentials, Argon2id verification, and roles
│   │   └── mod.rs                  # Authentication tokens and permission assertions
│   ├── cluster/                    # Multi-node federation subsystem
│   │   ├── db.rs                   # Node registry and status persistence
│   │   ├── mod.rs                  # Node connection state and message dispatch
│   │   └── tunnel.rs               # Reverse WebSocket tunnel server and agent client
│   ├── drivers/                    # Linux OS abstraction trait implementations
│   │   ├── mod.rs                  # SystemDriver, PackageManagerDriver, FirewallDriver
│   │   ├── memory.rs               # Memory flushing and swap management
│   │   ├── network.rs              # Interface and throughput inspection
│   │   ├── package_manager.rs      # APT, DNF, and Pacman implementations
│   │   └── service_manager.rs      # Systemd unit management
│   ├── plugins/                    # Out-of-process plugin host
│   │   ├── client.rs               # UDS JSON-RPC 2.0 communication client
│   │   ├── db.rs                   # Installed plugin registry persistence
│   │   ├── manager.rs              # Process supervisor, hot-reloading, and store downloader
│   │   └── mod.rs                  # Plugin interfaces and event definitions
│   ├── proxy/                      # Internal reverse proxy and asset server
│   │   ├── mod.rs                  # Single-binary embedded asset server (rust-embed)
│   │   └── reverse.rs              # Dynamic HTTP/WebSocket reverse proxy router
│   └── ssl/                        # Native TLS termination and certificate management
│       ├── acme.rs                 # Instant-ACME Let's Encrypt automated client
│       ├── config.rs               # TLS configuration persistence (wadm-ssl.json)
│       ├── generator.rs            # Self-signed certificate generation (rcgen)
│       ├── mod.rs                  # Dual-stack server supervisor
│       └── tls.rs                  # Rustls ServerConfig builder and certificate loader
└── web/                            # React 19 / TypeScript / Vite 7 SPA
    ├── src/
    │   ├── App.tsx                 # Root component, routing, and navigation layout
    │   ├── components/             # UI module views and management panels
    │   ├── context/                # React state providers (Auth, Stats, System, Jobs, Cluster)
    │   └── types/                  # TypeScript interface definitions
    ├── vite.config.ts              # Vite build and development proxy configuration
    └── package.json                # Frontend dependencies and scripts
```

---

## 3. Communication Pipelines & Data Flows

### 3.1. Telemetry Data Flow
1. The frontend initiates periodic polling or consumes telemetry streams.
2. The Actix-Web controller (`src/api/monitor.rs`) reads CPU, memory, and disk counters directly from `/proc` and `/sys`.
3. GPU metrics are polled asynchronously across vendor drivers (NVIDIA SMI, AMD sysfs, Intel DRM).
4. Upgradable package counts are resolved from the in-memory cache (`UPGRADABLE_CACHE`) with a 5-minute TTL, avoiding synchronous process spawning.
5. The serialized telemetry snapshot is returned as JSON to the UI for visualization.

### 3.2. Asynchronous Job & SSE Streaming Flow
1. A client initiates a long-running action (e.g., app deployment or system update).
2. The controller registers a new job in the `JobManager` (`src/apps/mod.rs` or `src/plugins/manager.rs`).
3. An HTTP 202 Accepted response is returned immediately containing the unique `job_id`.
4. A background Tokio task executes the operation, streaming stdout/stderr into the job's ring buffer and updating progress percentages in SQLite.
5. The UI connects to `GET /api/jobs/{id}/stream` via Server-Sent Events, receiving live terminal log events in real time.

### 3.3. Reverse WebSocket Cluster Federation
1. A cluster worker node runs in agent mode: `wadm --agent --hub-url <url> --token <token> --name <name>`.
2. The agent initiates an outbound WebSocket connection to the central panel at `/api/cluster/tunnel`.
3. The panel validates the join token and registers the node in the active node registry.
4. When an administrator inspects or manages the remote node from the UI, the panel forwards JSON-RPC requests across the established tunnel.
5. Periodic ping/pong frames maintain connection liveness, marking nodes offline if disconnected.

### 3.4. Out-of-Process Plugin Host Flow
1. Plugins are installed into `/var/lib/wadm/plugins/<plugin-id>/`.
2. The plugin supervisor spawns the plugin binary as an isolated child process.
3. The plugin binds to a Unix Domain Socket at `/var/lib/wadm/plugins/<plugin-id>/plugin.sock`.
4. The WADM backend establishes a client connection to the socket and exchanges JSON-RPC 2.0 requests for health checks, capabilities, and UI manifest registration.

---

## 4. Security Architecture & Hardening

### 4.1. Authentication & Session Management
- **Password Hashing:** Passwords are encrypted using Argon2id with random salts.
- **Two-Factor Authentication:** TOTP (RFC 6238) verified via HMAC-SHA256 and HMAC-SHA1.
- **Session Tokens:** Signed JWTs using a dynamically generated 256-bit secret saved to `.wadm_jwt_secret` with mode `0o600`.
- **Brute-Force Protection:** IP-based rate limiting enforcing a maximum of 5 failed attempts within 60 seconds.

### 4.2. Role-Based Access Control (RBAC)
- Three discrete tiers:
  - `Viewer`: Read-only access to monitoring, containers, and services.
  - `Operator`: Administrative actions on services, containers, databases, and maintenance tasks.
  - `Admin`: Full supervisory control including user creation, terminal access, and plugin management.
- Enforced at the route middleware level using `RequireRole`, `RequireOperator`, and `RequireAdmin`.

### 4.3. Filesystem & Execution Guardrails
- **Path Traversal Shield:** Resolves canonical paths and rejects any sequences containing `..` or null bytes.
- **Sensitive Path Guard:** Enforces strict read prohibitions on `/etc/shadow`, `/etc/gshadow`, `/etc/sudoers`, private keys (`id_*`, `.pem`), and `.wadm_jwt_secret`.
- **Protected Filesystem Hierarchy:** Prohibits write operations targeting `/proc`, `/sys`, `/dev`, `/boot`, `/etc/sudoers.d`, and `/usr`.
- **Command Sanitization:** Strict whitelisting of input parameters for package managers, systemd units, and UFW firewall rules.
- **Child Process Lifecycle:** Terminal PTY sessions are monitored; when a WebSocket closes, `SIGKILL` and process reaping prevent lingering processes.

---

## 5. Verification & Test Suite

The codebase enforces full automated verification across all layers:
- **Rust Unit & Integration Tests:** Comprehensive test suite covering JWT secret generation, rate limiting, SQL injection defense, path sanitization, firewall validation, package manager input filtering, and self-signed TLS generation.
- **Static Analysis & Linting:** Strict zero-warning enforcement via `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --all -- --check`.
- **Frontend Verification:** TypeScript compiler validation (`tsc -b`), ESLint verification (`npm run lint`), and production bundle compilation (`npm run build`).
