# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.96.0] - 2026-09-23

### Added
- **App Store Templates:** Added official templates for **Nginx Proxy Manager** (ports 8084, 8085, 8443) and **Portainer CE** (ports 9000, 9443).
- **Port Conflict Detection:** Pre-flight TCP port collision checks (`TcpListener::bind`) returning `409 Conflict` before launching containers.
- **App Management Endpoints:**
  - `GET /api/apps/{id}/credentials`: Secure retrieval of generated app passwords stored under `0o600` permissions.
  - `POST /api/apps/{id}/uninstall`: Automated container and volume cleanup (`docker-compose down -v`).
- **Telemetry Settings:** User-configurable metric polling interval (1s High Precision, 2s Default, 5s Resource Saver) in Settings panel.
- **Community Standards:** Added `SECURITY.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, PR template, and GitHub Issue forms.
- **Automated Dependency Updates:** Integrated `.github/dependabot.yml` for Cargo, npm, and GitHub Actions.

### Changed
- **Direct Kernel Routing:** Replaced subshell `ip route` invocation with zero-cost Linux `/proc/net/route` parser, saving 4 spawned subshells every 2 seconds.
- **Log Store Ring Buffer:** Migrated `LogStore` from `Vec<LogEntry>` to `VecDeque<LogEntry>`, eliminating $O(N)$ shift operations in favor of $O(1)$ amortized eviction.
- **Service & Container Caching:** Added 30-second TTL cache for systemd service counts and 10-second TTL cache for container metrics.
- **Package Manager Caching:** Cached package manager detection (`which apt/dnf/pacman`) via `Lazy<ManagerType>`.
- **System Information Sharing:** Reused `sysinfo::System` state from `AppState` rather than re-instantiating on each query; parsed CPU temperature via `/sys/class/thermal/`.
- **Lazy Web Terminal:** Terminal WebSocket and pseudo-terminal (PTY) now connect only when opened in the UI, freeing system resources when idle.
- **CI/CD Quality Gates:** Upgraded GitHub Actions workflow to run `cargo test` and `npm run lint`; modernized release workflow with `softprops/action-gh-release@v2`.

### Fixed
- **Authentication Resilience:** Removed fatal `panic!` on corrupted authentication store; gracefully falls back to setup/recovery mode.
- **Rate Limiter Memory Leak:** Implemented periodic eviction of expired IP entries in `LoginRateLimiter`.
- **Terminal Crash Prevention:** Removed `.expect(...)` panics from PTY allocation, child process spawn, and I/O streams in `terminal.rs`.
- **Formatting & Lints:** Resolved all Clippy warnings and rustfmt alignment issues.
