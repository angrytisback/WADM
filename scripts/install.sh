#!/bin/sh
# WADM Installation Script
# https://github.com/angrytisback/WADM
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/angrytisback/WADM/main/scripts/install.sh | sh
#
# Requirements:
#   - Linux x86_64, aarch64, or riscv64
#   - curl, tar, systemctl, sudo
#   - systemd-based init system

set -eu

REPO="angrytisback/WADM"
INSTALL_BIN="/usr/local/bin/wadm"
SERVICE_USER="wadm"
SERVICE_GROUP="wadm"
SYSTEMD_UNIT_DIR="/etc/systemd/system"
SUDOERS_DIR="/etc/sudoers.d"
DATA_DIR="/var/lib/wadm"

# ─── Utility Functions ─────────────────────────────────────────────────────────

log_info()  { printf '[INFO]  %s\n' "$1"; }
log_error() { printf '[ERROR] %s\n' "$1" >&2; }

require_command() {
    command -v "$1" > /dev/null 2>&1 || {
        log_error "Required command not found: $1"
        exit 1
    }
}

require_root() {
    if [ "$(id -u)" -ne 0 ]; then
        log_error "This script must be run as root (use sudo)."
        exit 1
    fi
}

# ─── Architecture Detection ────────────────────────────────────────────────────

detect_arch() {
    MACHINE="$(uname -m)"
    case "$MACHINE" in
        x86_64)          echo "x86_64-unknown-linux-gnu" ;;
        aarch64|arm64)   echo "aarch64-unknown-linux-gnu" ;;
        riscv64)         echo "riscv64gc-unknown-linux-gnu" ;;
        *)
            log_error "Unsupported architecture: $MACHINE"
            exit 1
            ;;
    esac
}

# ─── Latest Release Fetch ──────────────────────────────────────────────────────

fetch_latest_version() {
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' \
        | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/'
}

# ─── Binary Download and Installation ─────────────────────────────────────────

download_binary() {
    TARGET="$1"
    VERSION="$2"
    ARTIFACT="wadm-${VERSION}-${TARGET}.tar.gz"
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARTIFACT}"
    TMPDIR="$(mktemp -d)"

    log_info "Downloading ${ARTIFACT}..."
    curl -fsSL -o "${TMPDIR}/${ARTIFACT}" "$URL"

    log_info "Extracting binary..."
    tar -xzf "${TMPDIR}/${ARTIFACT}" -C "$TMPDIR"

    install -o root -g root -m 0755 "${TMPDIR}/wadm" "$INSTALL_BIN"
    rm -rf "$TMPDIR"

    log_info "Binary installed to ${INSTALL_BIN}."
}

# ─── System User Setup ─────────────────────────────────────────────────────────

create_service_user() {
    if ! id -u "$SERVICE_USER" > /dev/null 2>&1; then
        log_info "Creating system user: ${SERVICE_USER}"
        groupadd --system "$SERVICE_GROUP"
        useradd \
            --system \
            --gid "$SERVICE_GROUP" \
            --home-dir "$DATA_DIR" \
            --no-create-home \
            --shell /sbin/nologin \
            "$SERVICE_USER"
    else
        log_info "System user ${SERVICE_USER} already exists. Skipping."
    fi

    if getent group docker > /dev/null 2>&1; then
        log_info "Adding ${SERVICE_USER} to docker group..."
        usermod -aG docker "$SERVICE_USER"
    fi
}

# ─── Data Directory ────────────────────────────────────────────────────────────

create_data_dir() {
    install -d -o "$SERVICE_USER" -g "$SERVICE_GROUP" -m 0750 "$DATA_DIR"
    log_info "Data directory: ${DATA_DIR}"
}

# ─── Systemd Service Installation ─────────────────────────────────────────────

install_systemd_unit() {
    UNIT_URL="https://raw.githubusercontent.com/${REPO}/main/systemd/wadm.service"
    log_info "Installing systemd service unit..."
    curl -fsSL -o "${SYSTEMD_UNIT_DIR}/wadm.service" "$UNIT_URL"
    chmod 0644 "${SYSTEMD_UNIT_DIR}/wadm.service"
}

# ─── Sudoers Configuration ────────────────────────────────────────────────────

install_sudoers() {
    SUDOERS_URL="https://raw.githubusercontent.com/${REPO}/main/systemd/wadm.sudoers"
    SUDOERS_FILE="${SUDOERS_DIR}/wadm"
    log_info "Installing sudoers drop-in..."
    curl -fsSL -o "$SUDOERS_FILE" "$SUDOERS_URL"
    chmod 0440 "$SUDOERS_FILE"

    # Validate the sudoers file before activating it
    if ! visudo -cf "$SUDOERS_FILE" > /dev/null 2>&1; then
        log_error "Sudoers syntax validation failed. Removing invalid file."
        rm -f "$SUDOERS_FILE"
        exit 1
    fi
    log_info "Sudoers drop-in installed at ${SUDOERS_FILE}."
}

# ─── Service Activation ────────────────────────────────────────────────────────

enable_and_start_service() {
    log_info "Enabling and starting wadm service..."
    systemctl daemon-reload
    systemctl enable --now wadm.service
    log_info "WADM is running. Access the panel at http://$(hostname -I | awk '{print $1}'):8080"
}

# ─── Main ─────────────────────────────────────────────────────────────────────

main() {
    require_root
    require_command curl
    require_command tar
    require_command systemctl

    TARGET="$(detect_arch)"
    VERSION="$(fetch_latest_version)"

    log_info "Installing WADM ${VERSION} for target ${TARGET}..."

    download_binary "$TARGET" "$VERSION"
    create_service_user
    create_data_dir
    install_systemd_unit
    install_sudoers
    enable_and_start_service

    log_info "Installation complete."
}

main "$@"
