# =============================================================================
# Stage 1: Frontend Build
# =============================================================================
FROM node:20-alpine AS frontend-builder

WORKDIR /app/web

COPY web/package.json web/package-lock.json ./
RUN npm ci --prefer-offline

COPY web/ ./
RUN npm run build

# =============================================================================
# Stage 2: Backend Build
# =============================================================================
FROM rust:alpine AS backend-builder

RUN apk add --no-cache build-base pkgconfig openssl-dev

WORKDIR /app

# Copy embedded frontend assets first (required by rust-embed at compile time)
COPY --from=frontend-builder /app/web/dist ./web/dist

# Pre-cache dependency compilation by building an empty manifest first
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Build actual source code
COPY src/ src/
RUN touch src/main.rs && cargo build --release

# =============================================================================
# Stage 3: Runtime Image
# =============================================================================
FROM alpine:3.20 AS runtime

# Install runtime dependencies:
# - ca-certificates: TLS verification for HTTPS and ACME
# - smartmontools: S.M.A.R.T. disk telemetry
# - bash: Interactive shell for web terminal
# - curl: Health check and script utilities
RUN apk add --no-cache ca-certificates smartmontools bash curl

# Create dedicated non-root service user
RUN addgroup -S wadm && adduser -S -G wadm -h /var/lib/wadm wadm

WORKDIR /opt/wadm

# Copy compiled binary (contains embedded frontend assets)
COPY --from=backend-builder /app/target/release/wadm ./wadm

# Configure runtime directories and permissions
RUN mkdir -p /var/lib/wadm /run/wadm && \
    chown -R wadm:wadm /opt/wadm /var/lib/wadm /run/wadm

VOLUME ["/var/lib/wadm"]

USER wadm

EXPOSE 8168

ENTRYPOINT ["/opt/wadm/wadm"]
