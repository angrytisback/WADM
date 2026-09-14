# Stage 1: Build Frontend
FROM node:20-slim AS frontend-builder
WORKDIR /app
COPY web/package.json web/package-lock.json* ./
RUN npm install
COPY web/ ./
RUN npm run build

# Stage 2: Build Backend
FROM rust:1.80-slim AS backend-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src/ ./src/
# Copy frontend dist to serve (assuming wadm serves it from ./web/dist)
COPY --from=frontend-builder /app/dist ./web/dist
RUN cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies required by wadm
RUN apt-get update && apt-get install -y \
    sudo \
    smartmontools \
    nvme-cli \
    curl \
    iproute2 \
    procps \
    && rm -rf /var/lib/apt/lists/*

COPY --from=backend-builder /app/target/release/wadm ./wadm
COPY --from=frontend-builder /app/dist ./web/dist

# WADM needs a .wadm_jwt_secret file or will create it on the fly in the working directory
EXPOSE 8080

CMD ["./wadm"]
