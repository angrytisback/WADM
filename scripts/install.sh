#!/usr/bin/env bash
# ==============================================================================
# WADM - Modern Linux Web Administration Panel
# Production Installation Script
#
# Usage:
#   curl -sSL https://raw.githubusercontent.com/angrytisback/WADM/main/scripts/install.sh | sudo bash
#   sudo ./scripts/install.sh [options]
#
# Options:
#   --agent                 Install as a WADM Cluster Node Agent (Outbound WebSocket)
#   --hub-url <url>         Hub WebSocket URL for Agent (default: ws://localhost:8168/api/cluster/tunnel)
#   --token <token>         Authentication Join Token for Cluster Agent
#   --name <node-name>      Friendly name for this Cluster Node
#   --version <vX.Y.Z>      Specify WADM release version (default: latest)
#   --local-bin <path>      Install from a local binary instead of downloading
#   -h, --help              Show help message
# ==============================================================================

set -euo pipefail

REPO="angrytisback/WADM"
BIN_DEST="/usr/local/bin/wadm"
SERVICE_USER="wadm"
SERVICE_GROUP="wadm"
DATA_DIR="/var/lib/wadm"
RUN_DIR="/run/wadm"
SUDOERS_FILE="/etc/sudoers.d/wadm"
SYSTEMD_UNIT="/etc/systemd/system/wadm.service"

IS_AGENT=false
HUB_URL="ws://localhost:8168/api/cluster/tunnel"
AGENT_TOKEN=""
NODE_NAME=""
CUSTOM_VERSION=""
LOCAL_BIN_PATH=""

# --- ANSI Color Codes ---
BOLD="\033[1m"
GREEN="\033[1;32m"
BLUE="\033[1;34m"
YELLOW="\033[1;33m"
RED="\033[1;31m"
CYAN="\033[1;36m"
NC="\033[0m"

log_info()    { echo -e "${BLUE}[INFO]${NC}  $1"; }
log_success() { echo -e "${GREEN}[OK]${NC}    $1"; }
log_warn()    { echo -e "${YELLOW}[WARN]${NC}  $1"; }
log_error()   { echo -e "${RED}[ERROR]${NC} $1" >&2; }
die()         { log_error "$1"; exit 1; }

# --- Argument Parsing ---
parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --agent)
                IS_AGENT=true
                shift
                ;;
            --hub-url)
                [[ -n "${2:-}" ]] || die "Option --hub-url requires an argument"
                HUB_URL="$2"
                shift 2
                ;;
            --token)
                [[ -n "${2:-}" ]] || die "Option --token requires an argument"
                AGENT_TOKEN="$2"
                shift 2
                ;;
            --name)
                [[ -n "${2:-}" ]] || die "Option --name requires an argument"
                NODE_NAME="$2"
                shift 2
                ;;
            --version)
                [[ -n "${2:-}" ]] || die "Option --version requires an argument"
                CUSTOM_VERSION="$2"
                shift 2
                ;;
            --local-bin)
                [[ -n "${2:-}" ]] || die "Option --local-bin requires an argument"
                LOCAL_BIN_PATH="$2"
                shift 2
                ;;
            -h|--help)
                echo "WADM Production Installer"
                echo "Usage: sudo $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --agent               Install as WADM Cluster Node Agent"
                echo "  --hub-url <url>       Hub WebSocket tunnel URL (default: ws://localhost:8168/api/cluster/tunnel)"
                echo "  --token <token>       Node authentication join token"
                echo "  --name <name>         Friendly cluster node name"
                echo "  --version <vX.Y.Z>    Target release version (default: latest)"
                echo "  --local-bin <path>    Use local binary file instead of downloading"
                echo "  -h, --help            Show this help"
                exit 0
                ;;
            *)
                die "Unknown option: $1 (see --help)"
                ;;
        esac
    done
}

# --- System Checks ---
check_prerequisites() {
    if [[ "$(id -u)" -ne 0 ]]; then
        die "This installer must be run as root or with sudo."
    fi

    # Verify systemd init system
    if ! command -v systemctl >/dev/null 2>&1 || [[ ! -d /run/systemd/system ]]; then
        die "Systemd is required but not detected as the active init system."
    fi

    # Check basic tools
    local REQUIRED_CMDS=(tar install useradd getent)
    if [[ -z "$LOCAL_BIN_PATH" ]]; then
        REQUIRED_CMDS+=(curl)
    fi

    for cmd in "${REQUIRED_CMDS[@]}"; do
        command -v "$cmd" >/dev/null 2>&1 || die "Required command missing: $cmd. Please install it first."
    done
}

# --- Architecture Detection ---
detect_target_triple() {
    local ARCH
    ARCH="$(uname -m)"
    case "$ARCH" in
        x86_64|amd64)
            echo "x86_64-unknown-linux-gnu"
            ;;
        aarch64|arm64)
            echo "aarch64-unknown-linux-gnu"
            ;;
        *)
            die "Unsupported system architecture: $ARCH (only x86_64 and aarch64 are supported)"
            ;;
    esac
}

# --- Fetch Latest Version ---
fetch_version() {
    if [[ -n "$CUSTOM_VERSION" ]]; then
        echo "$CUSTOM_VERSION"
        return
    fi

    local LATEST
    LATEST="$(curl -fsSL -H "Accept: application/vnd.github.v3+json" "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null \
        | grep -m 1 '"tag_name"' \
        | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/' || true)"

    if [[ -z "$LATEST" ]]; then
        log_warn "Could not resolve latest release tag from GitHub API, falling back to v0.96.0"
        echo "v0.96.0"
    else
        echo "$LATEST"
    fi
}

# --- Install Binary ---
install_binary() {
    if [[ -n "$LOCAL_BIN_PATH" ]]; then
        log_info "Installing binary from local path: ${LOCAL_BIN_PATH}"
        [[ -f "$LOCAL_BIN_PATH" ]] || die "Local binary not found at: ${LOCAL_BIN_PATH}"
        install -o root -g root -m 0755 "$LOCAL_BIN_PATH" "$BIN_DEST"
        log_success "Binary installed to ${BIN_DEST}"
        return
    fi

    local TARGET VERSION ARTIFACT_NAME DOWNLOAD_URL TMP_DIR
    TARGET="$(detect_target_triple)"
    VERSION="$(fetch_version)"
    ARTIFACT_NAME="wadm-${VERSION}-${TARGET}.tar.gz"
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARTIFACT_NAME}"
    TMP_DIR="$(mktemp -d)"

    log_info "Target architecture: ${TARGET}"
    log_info "Downloading WADM ${VERSION} release archive..."

    if ! curl -fsSL -o "${TMP_DIR}/${ARTIFACT_NAME}" "$DOWNLOAD_URL"; then
        rm -rf "$TMP_DIR"
        die "Failed to download WADM release from: ${DOWNLOAD_URL}"
    fi

    log_info "Extracting ${ARTIFACT_NAME}..."
    tar -xzf "${TMP_DIR}/${ARTIFACT_NAME}" -C "$TMP_DIR"

    if [[ ! -f "${TMP_DIR}/wadm" ]]; then
        rm -rf "$TMP_DIR"
        die "Archive did not contain 'wadm' executable."
    fi

    install -o root -g root -m 0755 "${TMP_DIR}/wadm" "$BIN_DEST"
    rm -rf "$TMP_DIR"
    log_success "Single-binary installed to ${BIN_DEST}"
}

# --- Setup System User & Directories ---
setup_system_user_and_dirs() {
    log_info "Configuring system service user '${SERVICE_USER}'..."
    if ! getent group "$SERVICE_GROUP" >/dev/null 2>&1; then
        groupadd --system "$SERVICE_GROUP"
    fi

    if ! id -u "$SERVICE_USER" >/dev/null 2>&1; then
        useradd \
            --system \
            --gid "$SERVICE_GROUP" \
            --home-dir "$DATA_DIR" \
            --no-create-home \
            --shell /bin/false \
            "$SERVICE_USER"
        log_success "Created system user '${SERVICE_USER}'"
    else
        log_info "System user '${SERVICE_USER}' already exists"
    fi

    # Add to docker group if Docker is installed
    if getent group docker >/dev/null 2>&1; then
        usermod -aG docker "$SERVICE_USER"
        log_success "Added '${SERVICE_USER}' to 'docker' group"
    fi

    log_info "Setting up hardened filesystem directories..."
    # /var/lib/wadm (0755)
    install -d -o "$SERVICE_USER" -g "$SERVICE_GROUP" -m 0755 "$DATA_DIR"
    # /var/lib/wadm/plugins (0755)
    install -d -o "$SERVICE_USER" -g "$SERVICE_GROUP" -m 0755 "${DATA_DIR}/plugins"
    # /var/lib/wadm/certs (0700 - Strict SSL/TLS key isolation)
    install -d -o "$SERVICE_USER" -g "$SERVICE_GROUP" -m 0700 "${DATA_DIR}/certs"
    # /var/lib/wadm/apps (0755 - App Store Docker Compose projects)
    install -d -o "$SERVICE_USER" -g "$SERVICE_GROUP" -m 0755 "${DATA_DIR}/apps"
    # /run/wadm (0755 - Unix Domain Socket directory)
    install -d -o "$SERVICE_USER" -g "$SERVICE_GROUP" -m 0755 "$RUN_DIR"

    log_success "Directories configured with strict permissions"
}

# --- Install Sudoers Configuration ---
setup_sudoers() {
    log_info "Installing least-privilege sudoers drop-in..."
    local SCRIPT_DIR
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    local LOCAL_SUDOERS="${SCRIPT_DIR}/../systemd/wadm.sudoers"

    if [[ -f "$LOCAL_SUDOERS" ]]; then
        install -o root -g root -m 0440 "$LOCAL_SUDOERS" "$SUDOERS_FILE"
    else
        local SUDOERS_URL="https://raw.githubusercontent.com/${REPO}/main/systemd/wadm.sudoers"
        curl -fsSL -o "$SUDOERS_FILE" "$SUDOERS_URL"
        chmod 0440 "$SUDOERS_FILE"
        chown root:root "$SUDOERS_FILE"
    fi

    # Validate syntax before leaving in place
    if ! visudo -c -f "$SUDOERS_FILE" >/dev/null 2>&1; then
        rm -f "$SUDOERS_FILE"
        die "Sudoers syntax verification failed with visudo. Removed ${SUDOERS_FILE}."
    fi

    log_success "Sudoers rules verified and installed at ${SUDOERS_FILE}"
}

# --- Install and Start Systemd Unit ---
setup_systemd_service() {
    log_info "Configuring systemd service..."

    if [[ "$IS_AGENT" == true ]]; then
        log_info "Generating WADM Cluster Node Agent service..."
        local NAME_OPT=""
        if [[ -n "$NODE_NAME" ]]; then
            NAME_OPT="--name ${NODE_NAME}"
        fi

        cat <<EOF > "$SYSTEMD_UNIT"
[Unit]
Description=WADM Cluster Node Agent
Documentation=https://github.com/angrytisback/WADM
After=network.target docker.service
Wants=docker.service

[Service]
Type=simple
User=${SERVICE_USER}
Group=${SERVICE_GROUP}
WorkingDirectory=${DATA_DIR}
ExecStart=${BIN_DEST} --agent --hub-url ${HUB_URL} --token ${AGENT_TOKEN} ${NAME_OPT}
Restart=always
RestartSec=5s
LimitNOFILE=65536
Environment=RUST_LOG=info
Environment=WADM_DATA_DIR=${DATA_DIR}

# Process Hardening
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ProtectControlGroups=true
ProtectKernelModules=true
ProtectKernelTunables=true
ReadWritePaths=${DATA_DIR} ${RUN_DIR}
LockPersonality=true
RemoveIPC=true

[Install]
WantedBy=multi-user.target
EOF
        chmod 0644 "$SYSTEMD_UNIT"
    else
        local SCRIPT_DIR
        SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
        local LOCAL_UNIT="${SCRIPT_DIR}/../systemd/wadm.service"

        if [[ -f "$LOCAL_UNIT" ]]; then
            install -o root -g root -m 0644 "$LOCAL_UNIT" "$SYSTEMD_UNIT"
        else
            local UNIT_URL="https://raw.githubusercontent.com/${REPO}/main/systemd/wadm.service"
            curl -fsSL -o "$SYSTEMD_UNIT" "$UNIT_URL"
            chmod 0644 "$SYSTEMD_UNIT"
            chown root:root "$SYSTEMD_UNIT"
        fi
    fi

    systemctl daemon-reload
    systemctl enable --now wadm.service
    log_success "Service wadm.service enabled and started"
}

# --- Completion Display ---
print_completion_banner() {
    local HOST_IP
    HOST_IP="$(hostname -I 2>/dev/null | awk '{print $1}' || echo "127.0.0.1")"
    if [[ -z "$HOST_IP" ]]; then
        HOST_IP="127.0.0.1"
    fi

    echo ""
    echo -e "${CYAN}==================================================================${NC}"
    echo -e "${CYAN}  __          __     _____  __  __ ${NC}"
    echo -e "${CYAN}  \ \        / /\   |  __ \|  \/  |${NC}   ${BOLD}WADM Linux Administration${NC}"
    echo -e "${CYAN}   \ \  /\  / /  \  | |  | | \  / |${NC}   ${GREEN}Production Deployment${NC}"
    echo -e "${CYAN}    \ \/  \/ / /\ \ | |  | | |\/| |${NC}   Single-Binary & Reverse Proxy"
    echo -e "${CYAN}     \  /\  / ____ \| |__| | |  | |${NC}"
    echo -e "${CYAN}      \/  \/_/    \_\_____/|_|  |_|${NC}"
    echo -e "${CYAN}==================================================================${NC}"
    echo ""

    if [[ "$IS_AGENT" == true ]]; then
        echo -e "${GREEN}${BOLD}[OK] WADM Cluster Node Agent Successfully Installed!${NC}"
        echo ""
        echo -e "  ${BOLD}Hub Tunnel:${NC}      ${HUB_URL}"
        echo -e "  ${BOLD}Node Name:${NC}       ${NODE_NAME:-"(Auto-detected hostname)"}"
        echo -e "  ${BOLD}Service Status:${NC}  systemctl status wadm"
        echo -e "  ${BOLD}Live Logs:${NC}       journalctl -u wadm -f"
    else
        echo -e "${GREEN}${BOLD}[OK] WADM Panel Successfully Installed & Started!${NC}"
        echo ""
        echo -e "  ${BOLD}Web Dashboard:${NC}   ${CYAN}http://${HOST_IP}:8168${NC}"
        echo -e "  ${BOLD}Local Access:${NC}    ${CYAN}http://localhost:8168${NC}"
        echo -e "  ${BOLD}Data Directory:${NC}  ${DATA_DIR}"
        echo -e "  ${BOLD}Single Binary:${NC}   ${BIN_DEST}"
        echo ""
        echo -e "  ${YELLOW}First Visit:${NC} Open http://${HOST_IP}:8168 in your browser to create the Admin account."
        echo ""
        echo -e "  ${BOLD}Manage Service:${NC}"
        echo -e "    Status:   ${BLUE}systemctl status wadm${NC}"
        echo -e "    Restart:  ${BLUE}systemctl restart wadm${NC}"
        echo -e "    Logs:     ${BLUE}journalctl -u wadm -f${NC}"
    fi
    echo ""
    echo -e "${CYAN}==================================================================${NC}"
    echo ""
}

main() {
    parse_args "$@"
    check_prerequisites
    install_binary
    setup_system_user_and_dirs
    setup_sudoers
    setup_systemd_service
    print_completion_banner
}

main "$@"
