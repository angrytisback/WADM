# WADM Docker Deployment & Multi-Architecture Publishing Guide

This guide details building, publishing, and operating WADM multi-architecture container images (`linux/amd64` and `linux/arm64`) for Docker Hub and GitHub Container Registry (GHCR).

---

## 1. Local Image Build

To build the WADM container image locally on your machine:

```bash
docker build -t wadm:latest .
```

The multi-stage build will:
1. Compile the React 19 frontend into static assets (`web/dist`).
2. Compile the Rust backend in release mode, embedding the frontend assets directly into the binary via `rust-embed`.
3. Package the statically-compiled binary into an Alpine Linux runtime container with non-root user `wadm`.

---

## 2. Multi-Architecture Build with Docker Buildx

WADM supports `linux/amd64` and `linux/arm64` container architectures. To build and push multi-arch images directly to your container registry:

```bash
# Initialize a buildx builder instance if not already active
docker buildx create --use --name wadm-builder

# Build and push to Docker Hub
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t <your_dockerhub_username>/wadm:latest \
  -t <your_dockerhub_username>/wadm:0.96.0 \
  --push .
```

To build and push to GitHub Container Registry (GHCR):

```bash
# Authenticate with GHCR
echo $CR_PAT | docker login ghcr.io -u <your_github_username> --password-stdin

# Build and push
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t ghcr.io/<your_github_username>/wadm:latest \
  -t ghcr.io/<your_github_username>/wadm:0.96.0 \
  --push .
```

---

## 3. Production Container Execution

WADM is a Linux server administration and telemetry platform. Running it in an isolated container restricts its view to the container namespace. To allow WADM to monitor host hardware, Docker containers, and systemd services, mount the required host interfaces:

```bash
docker run -d \
  --name wadm \
  --restart unless-stopped \
  --privileged \
  --pid=host \
  --network=host \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v /var/lib/wadm:/var/lib/wadm \
  -v /run/systemd:/run/systemd \
  ghcr.io/<your_github_username>/wadm:latest
```

### Mount Configuration Overview

| Host Path | Purpose |
| :--- | :--- |
| `/var/run/docker.sock` | Enables WADM Docker manager to inspect and orchestrate host containers via Bollard. |
| `/var/lib/wadm` | Persistent data directory for SQLite database (`wadm.db`), plugin storage, and SSL certificates. |
| `/run/systemd` | Allows reading host service units and systemd journal events. |
| `--pid=host` | Grants access to the host `/proc` filesystem for process table inspection and memory metrics. |
| `--network=host` | Binds WADM directly to host network interfaces for bandwidth monitoring and reverse proxy routing. |

---

## 4. First-Time Setup

1. Open your browser to `http://<server-ip>:8168`.
2. Follow the setup wizard to configure the master administrator username, secure password, and RFC 6238 TOTP 2FA secret.
3. Once enrolled, log in using your credentials and 6-digit TOTP token.
