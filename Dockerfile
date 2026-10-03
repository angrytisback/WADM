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
FROM rust:1.80-alpine AS backend-builder

# Install musl toolchain and required system libraries
RUN apk add --no-cache musl-dev libc-dev pkgconfig openssl-dev

# Pre-cache dependency compilation by building an empty manifest first
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Build actual source
COPY src/ src/
# Touch main.rs to force Cargo to rebuild the application binary
RUN touch src/main.rs && cargo build --release

# =============================================================================
# Stage 3: Runtime Image
# =============================================================================
FROM alpine:3.20 AS runtime

# Install runtime dependencies:
# - ca-certificates: required for TLS connections (HTTPS endpoints, Docker Hub)
# - smartmontools: required for S.M.A.R.T. disk health queries
# - bash: interactive shell for web terminal
RUN apk add --no-cache ca-certificates smartmontools bash

# Create a dedicated non-root service user
RUN addgroup -S wadm && adduser -S -G wadm wadm

WORKDIR /opt/wadm

# Copy compiled binary
COPY --from=backend-builder /app/target/release/wadm ./wadm

# Copy frontend static assets alongside binary
COPY --from=frontend-builder /app/web/dist ./web/dist

# Set ownership
RUN chown -R wadm:wadm /opt/wadm

USER wadm

EXPOSE 8080

ENTRYPOINT ["./wadm"]
