#!/usr/bin/env bash
# ==============================================================================
# WADM - Modern Linux Web Administration Panel
# Production Uninstallation Script
#
# Usage:
#   sudo ./scripts/uninstall.sh [options]
#
# Options:
#   --purge     Completely remove all persistent data, databases, and certs (/var/lib/wadm)
#   -y, --yes   Skip interactive confirmation prompt
#   -h, --help  Show help message
# ==============================================================================

set -euo pipefail

BIN_DEST="/usr/local/bin/wadm"
SERVICE_USER="wadm"
SERVICE_GROUP="wadm"
DATA_DIR="/var/lib/wadm"
RUN_DIR="/run/wadm"
SUDOERS_FILE="/etc/sudoers.d/wadm"
SYSTEMD_UNIT="/etc/systemd/system/wadm.service"

PURGE_DATA=false
SKIP_PROMPT=false

# --- ANSI Color Codes ---
BOLD="\033[1m"
GREEN="\033[1;32m"
BLUE="\033[1;34m"
YELLOW="\033[1;33m"
RED="\033[1;31m"
NC="\033[0m"

log_info()    { echo -e "${BLUE}[INFO]${NC}  $1"; }
log_success() { echo -e "${GREEN}[OK]${NC}    $1"; }
log_warn()    { echo -e "${YELLOW}[WARN]${NC}  $1"; }
log_error()   { echo -e "${RED}[ERROR]${NC} $1" >&2; }
die()         { log_error "$1"; exit 1; }

parse_args() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --purge)
                PURGE_DATA=true
                shift
                ;;
            -y|--yes)
                SKIP_PROMPT=true
                shift
                ;;
            -h|--help)
                echo "WADM Uninstaller"
                echo "Usage: sudo $0 [OPTIONS]"
                echo ""
                echo "Options:"
                echo "  --purge       Delete all data directories (/var/lib/wadm and databases)"
                echo "  -y, --yes     Bypass confirmation prompt"
                echo "  -h, --help    Show this help"
                exit 0
                ;;
            *)
                die "Unknown option: $1 (see --help)"
                ;;
        esac
    done
}

check_root() {
    if [[ "$(id -u)" -ne 0 ]]; then
        die "This uninstaller must be run as root or with sudo."
    fi
}

confirm_uninstall() {
    if [[ "$SKIP_PROMPT" == true ]]; then
        return
    fi

    echo ""
    echo -e "${YELLOW}${BOLD}WARNING: You are about to uninstall WADM from this system.${NC}"
    if [[ "$PURGE_DATA" == true ]]; then
        echo -e "${RED}Danger: --purge flag is set! All databases, SSL certs, and app configs in ${DATA_DIR} will be permanently erased!${NC}"
    else
        echo -e "Note: Persistent data in ${DATA_DIR} will be preserved. Use --purge to remove it."
    fi
    echo ""

    read -r -p "Are you sure you want to proceed? [y/N]: " response
    case "$response" in
        [yY][eE][sS]|[yY])
            ;;
        *)
            echo "Uninstallation cancelled by user."
            exit 0
            ;;
    esac
}

stop_and_remove_service() {
    if command -v systemctl >/dev/null 2>&1; then
        if systemctl is-active --quiet wadm 2>/dev/null; then
            log_info "Stopping wadm service..."
            systemctl stop wadm || log_warn "Failed to stop wadm service cleanly"
        fi

        if systemctl is-enabled --quiet wadm 2>/dev/null; then
            log_info "Disabling wadm service..."
            systemctl disable wadm || log_warn "Failed to disable wadm service"
        fi

        if [[ -f "$SYSTEMD_UNIT" ]]; then
            log_info "Removing systemd service unit ${SYSTEMD_UNIT}..."
            rm -f "$SYSTEMD_UNIT"
            systemctl daemon-reload
            log_success "Systemd unit removed"
        fi
    fi
}

remove_sudoers() {
    if [[ -f "$SUDOERS_FILE" ]]; then
        log_info "Removing sudoers drop-in: ${SUDOERS_FILE}..."
        rm -f "$SUDOERS_FILE"
        log_success "Sudoers drop-in removed"
    fi
}

remove_binary() {
    if [[ -f "$BIN_DEST" ]]; then
        log_info "Removing WADM binary: ${BIN_DEST}..."
        rm -f "$BIN_DEST"
        log_success "Binary removed"
    fi
}

remove_user() {
    if id -u "$SERVICE_USER" >/dev/null 2>&1; then
        log_info "Removing system user '${SERVICE_USER}'..."
        userdel "$SERVICE_USER" 2>/dev/null || log_warn "Could not delete user ${SERVICE_USER}"
    fi

    if getent group "$SERVICE_GROUP" >/dev/null 2>&1; then
        groupdel "$SERVICE_GROUP" 2>/dev/null || log_warn "Could not delete group ${SERVICE_GROUP}"
    fi
    log_success "System user and group removed"
}

cleanup_runtime_and_data() {
    if [[ -d "$RUN_DIR" ]]; then
        log_info "Cleaning up runtime socket directory: ${RUN_DIR}..."
        rm -rf "$RUN_DIR"
    fi

    if [[ "$PURGE_DATA" == true ]]; then
        if [[ -d "$DATA_DIR" ]]; then
            log_warn "Purging persistent data directory: ${DATA_DIR}..."
            rm -rf "$DATA_DIR"
            log_success "Data directory ${DATA_DIR} purged"
        fi
    else
        if [[ -d "$DATA_DIR" ]]; then
            log_info "Persistent data preserved at ${DATA_DIR}. To delete manually: rm -rf ${DATA_DIR}"
        fi
    fi
}

main() {
    parse_args "$@"
    check_root
    confirm_uninstall
    stop_and_remove_service
    remove_sudoers
    remove_binary
    remove_user
    cleanup_runtime_and_data

    echo ""
    log_success "WADM has been successfully uninstalled."
}

main "$@"
