# WADM - Web Admin

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)](#)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-1.80+-orange)](#)
[![React](https://img.shields.io/badge/React-18.x-blue)](#)

WADM (Web Admin) is a highly secure, lightweight, and performant web-based administration panel designed for Linux server management. Built with a Rust backend (Actix-Web) and a React/TypeScript frontend (Vite), WADM provides a responsive, real-time dashboard for comprehensive server monitoring, package management, container orchestration, and power sequencing.

## Table of Contents
- [Architecture Overview](#architecture-overview)
- [Key Features](#key-features)
- [Security Protocol](#security-protocol)
- [Installation and Setup](#installation-and-setup)
- [Building from Source](#building-from-source)
- [Docker Deployment](#docker-deployment)
- [CI/CD and Releases](#cicd-and-releases)
- [License](#license)

## Architecture Overview

WADM operates on a monolithic client-server architecture engineered for minimal footprint and maximum security:
- **Backend**: Rust (Actix-Web) handles system-level API requests, strictly executing authorized Linux commands via standard libraries, `bollard` for Docker IPC, and direct D-Bus/sysfs integrations.
- **Frontend**: React 18 with TypeScript, bundled by Vite. State is managed via Context API, ensuring low-latency real-time updates for telemetry graphs.
- **Communication**: Secure RESTful API validated via hardened JSON Web Tokens (JWT).

## Key Features

- **System Telemetry & Monitoring**: Real-time visualization of CPU, RAM, Swap, Network, and GPU usage (via NVML/sysfs).
- **Advanced Maintenance Automation**: Deep system cleaning protocols including RAM cache flushing, Swap cycle resetting, SSD TRIM (`fstrim`), and package manager cache sweeping.
- **App Store & Docker Orchestration**: One-click deployment of containerized services. Complete container lifecycle management (Start, Stop, Restart, Delete).
- **Advanced File Explorer**: Full read/write/execute capabilities over the server filesystem, including in-browser file editing and cross-network transfers.
- **Package Management**: Unified interface for APT, DNF, and Pacman. Update, upgrade, and resolve dependencies directly from the dashboard.
- **Scheduled Power Sequencing**: Hardware power control supporting instant reboots, immediate shutdowns, and minute-precision scheduled power cycles integrated directly with `systemd-logind`.
- **Integrated Terminal**: Fully functional WebSocket-based TTY interface for raw command-line access.

## Security Protocol

WADM is designed for production server environments where security is paramount:
- **Zero-Trust Authentication**: Requires valid Linux PAM authentication (`/etc/shadow`) mapped to authorized users.
- **Privilege Boundaries**: Requires `sudo` group membership or `root` user context. 
- **Dynamic Key Generation**: Secures sessions via a rotating, auto-generated 256-bit cryptographic JWT secret.
- **Sanitization Engine**: Strict regex-based input sanitization prevents arbitrary shell injection across all system command interfaces.

## Installation and Setup

### Prerequisites
- A Linux-based operating system (Debian, Ubuntu, RHEL, Arch).
- `sudo` privileges.
- Rust toolchain (if compiling from source).
- Node.js >= 20.x (if compiling from source).

### Downloading Pre-built Binaries
Navigate to the [Releases](https://github.com/yourusername/wadm/releases) page to download the latest binary for your architecture (`linux-x64`, `linux-arm64`, `linux-riscv64`).

```bash
chmod +x wadm
sudo ./wadm
```
The server will initialize on port `8080` by default.

## Building from Source

WADM uses a dual-build process. You can utilize the provided build script for cross-compilation.

```bash
# Clone the repository
git clone https://github.com/yourusername/wadm.git
cd wadm

# Execute the automated build script (requires 'cross' cargo plugin for multi-arch)
chmod +x build.sh
./build.sh
```

## Docker Deployment

To deploy WADM via Docker, host system bindings are required to allow the management interface to interact with the underlying hardware and services. 

```bash
docker run -d \
  --name wadm \
  --privileged \
  --pid=host \
  --network=host \
  -v /:/host_root \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v /run/systemd:/run/systemd \
  angryt/wadm:latest
```

## CI/CD and Releases

This repository is structured for continuous integration. To push a new release:
1. Commit your changes to the `main` branch.
2. Draft a new Release on GitHub.
3. Attach the compiled artifacts from the `build/` directory directly to the Release assets.

## License

Distributed under the MIT License. See `LICENSE` for more information.
