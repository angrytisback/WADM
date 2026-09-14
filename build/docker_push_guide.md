# WADM Docker Image Guide

This guide explains how to build and upload the `wadm` Docker image to Docker Hub, and how to run it so it can manage your host system properly.

## 1. Build the Docker Image
To build the Docker image, run the following command in the root of the project (where the `Dockerfile` is located).
Replace `<your_dockerhub_username>` with your actual Docker Hub username.

```bash
docker build -t <your_dockerhub_username>/wadm:latest .
```

If you want to build a multi-architecture image (for x86_64, ARM64, and RISC-V) and push it directly, you can use Docker Buildx:

```bash
# Set up a new builder instance if you haven't already
docker buildx create --use

# Build and push for multiple architectures
docker buildx build --platform linux/amd64,linux/arm64,linux/riscv64 -t <your_dockerhub_username>/wadm:latest --push .
```

## 2. Push to Docker Hub
If you built the image using the standard `docker build` command (not `buildx`), you can push it with:

```bash
# Login to Docker Hub (if you haven't already)
docker login

# Push the image
docker push <your_dockerhub_username>/wadm:latest
```

## 3. How to Run WADM via Docker
WADM is a system administration tool. If you run it inside a standard, isolated Docker container, it will only see the container's isolated filesystem, processes, and network, **not** the host Linux server.

To allow WADM to manage the host system (check disks, flush RAM, reboot host, manage docker apps), you must run it with elevated privileges and host mounts:

```bash
docker run -d \
  --name wadm \
  --privileged \
  --pid=host \
  --network=host \
  -v /:/host_root \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v /run/systemd:/run/systemd \
  <your_dockerhub_username>/wadm:latest
```

> **Note:** WADM scripts execute commands like `fstrim` and `apt`. When running inside Docker, some OS-level commands will only affect the Debian container WADM is running inside unless the backend is modified to explicitly `chroot /host_root` before executing system maintenance commands. For the best, most unrestricted experience on bare-metal servers, running the compiled binary directly as `root` (or via a systemd service) is recommended.
