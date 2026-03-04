#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# Planner — install / update script
#
# Builds from source, installs the server binary and web assets, and
# configures a systemd service. Run as root or with sudo.
#
# Usage:
#   sudo ./deploy/install.sh              # full install
#   sudo ./deploy/install.sh --update     # rebuild + restart, skip user/dir setup
#   sudo ./deploy/install.sh --uninstall  # remove everything
# ---------------------------------------------------------------------------

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
INSTALL_DIR="/opt/planner"
BIN_DIR="/usr/local/bin"
CONF_DIR="/etc/planner"
DATA_DIR="${INSTALL_DIR}/data"
WEB_DIR="${INSTALL_DIR}/web"
SERVICE_NAME="planner"
SERVICE_USER="planner"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()  { echo -e "${GREEN}[+]${NC} $*"; }
warn()  { echo -e "${YELLOW}[!]${NC} $*"; }
error() { echo -e "${RED}[x]${NC} $*" >&2; }
die()   { error "$@"; exit 1; }

require_root() {
    [[ $EUID -eq 0 ]] || die "Run with sudo or as root"
}

# ---------------------------------------------------------------------------
# Uninstall
# ---------------------------------------------------------------------------
do_uninstall() {
    require_root
    info "Stopping service..."
    systemctl stop "${SERVICE_NAME}" 2>/dev/null || true
    systemctl disable "${SERVICE_NAME}" 2>/dev/null || true

    info "Removing files..."
    rm -f "/etc/systemd/system/${SERVICE_NAME}.service"
    rm -f "${BIN_DIR}/planner-server"
    rm -rf "${INSTALL_DIR}"
    rm -rf "${CONF_DIR}"

    if id "${SERVICE_USER}" &>/dev/null; then
        info "Removing service user..."
        userdel "${SERVICE_USER}" 2>/dev/null || true
    fi

    systemctl daemon-reload
    info "Uninstalled."
    exit 0
}

# ---------------------------------------------------------------------------
# Preflight checks
# ---------------------------------------------------------------------------
check_deps() {
    local missing=()
    command -v cargo  &>/dev/null || missing+=(cargo)
    command -v node   &>/dev/null || missing+=(node)
    command -v npm    &>/dev/null || missing+=(npm)
    command -v git    &>/dev/null || missing+=(git)

    if [[ ${#missing[@]} -gt 0 ]]; then
        die "Missing dependencies: ${missing[*]}"
    fi

    info "Dependencies OK (cargo, node, npm, git)"
}

# ---------------------------------------------------------------------------
# Build
# ---------------------------------------------------------------------------
do_build() {
    info "Building Rust release binary..."
    # Build as the invoking user, not root (cargo doesn't like root)
    local build_user="${SUDO_USER:-$(whoami)}"
    if [[ "$build_user" != "root" ]]; then
        sudo -u "$build_user" bash -c "cd '${REPO_ROOT}' && cargo build --release --workspace"
    else
        cd "${REPO_ROOT}" && cargo build --release --workspace
    fi

    info "Building web frontend..."
    if [[ "$build_user" != "root" ]]; then
        sudo -u "$build_user" bash -c "cd '${REPO_ROOT}' && npm install --prefix planner-web && npm run build --prefix planner-web"
    else
        cd "${REPO_ROOT}" && npm install --prefix planner-web && npm run build --prefix planner-web
    fi

    info "Build complete."
}

# ---------------------------------------------------------------------------
# Install
# ---------------------------------------------------------------------------
do_install() {
    local update_only="${1:-false}"

    # Create service user (skip on update)
    if [[ "$update_only" == "false" ]]; then
        if ! id "${SERVICE_USER}" &>/dev/null; then
            info "Creating service user: ${SERVICE_USER}"
            useradd --system --shell /usr/sbin/nologin --home-dir "${INSTALL_DIR}" "${SERVICE_USER}"
        fi
    fi

    # Create directories
    info "Setting up directories..."
    mkdir -p "${INSTALL_DIR}" "${WEB_DIR}" "${DATA_DIR}" "${CONF_DIR}"

    # Stop service before replacing binary (avoids "Text file busy")
    if systemctl is-active --quiet "${SERVICE_NAME}" 2>/dev/null; then
        info "Stopping service for update..."
        systemctl stop "${SERVICE_NAME}"
    fi

    # Copy binary
    local binary="${REPO_ROOT}/target/release/planner-server"
    [[ -f "$binary" ]] || die "Release binary not found at ${binary} — run build first"
    info "Installing binary → ${BIN_DIR}/planner-server"
    cp "$binary" "${BIN_DIR}/planner-server"
    chmod 755 "${BIN_DIR}/planner-server"

    # Copy web assets
    local dist="${REPO_ROOT}/planner-web/dist"
    [[ -d "$dist" ]] || die "Web dist not found at ${dist} — run build first"
    info "Installing web assets → ${WEB_DIR}"
    rsync -a --delete "${dist}/" "${WEB_DIR}/"

    # Install env file (don't overwrite existing config)
    if [[ ! -f "${CONF_DIR}/planner.env" ]]; then
        info "Installing default env config → ${CONF_DIR}/planner.env"
        cp "${REPO_ROOT}/deploy/planner.env" "${CONF_DIR}/planner.env"
        chmod 640 "${CONF_DIR}/planner.env"
        chown root:${SERVICE_USER} "${CONF_DIR}/planner.env"
    else
        warn "Env config already exists — not overwriting ${CONF_DIR}/planner.env"
    fi

    # Install systemd unit
    info "Installing systemd service..."
    cp "${REPO_ROOT}/deploy/planner.service" "/etc/systemd/system/${SERVICE_NAME}.service"

    # Fix ownership
    chown -R "${SERVICE_USER}:${SERVICE_USER}" "${INSTALL_DIR}"

    # Reload and enable
    systemctl daemon-reload
    systemctl enable "${SERVICE_NAME}"

    # Start or restart
    if systemctl is-active --quiet "${SERVICE_NAME}"; then
        info "Restarting service..."
        systemctl restart "${SERVICE_NAME}"
    else
        info "Starting service..."
        systemctl start "${SERVICE_NAME}"
    fi

    # Status check
    sleep 1
    if systemctl is-active --quiet "${SERVICE_NAME}"; then
        info "Service is running."
        local port
        port=$(grep -oP '(?<=--port )\d+' /etc/systemd/system/${SERVICE_NAME}.service || echo "3100")
        local ip
        ip=$(ip -4 route get 1 2>/dev/null | awk '{for(i=1;i<=NF;i++) if($i=="src") print $(i+1)}' || hostname -f 2>/dev/null || echo "localhost")
        info "Access at: http://${ip}:${port}"
    else
        error "Service failed to start. Check: journalctl -u ${SERVICE_NAME} -n 50"
    fi
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
case "${1:-}" in
    --uninstall)
        do_uninstall
        ;;
    --update)
        require_root
        check_deps
        do_build
        do_install true
        info "Update complete."
        ;;
    --help|-h)
        echo "Usage: sudo $0 [--update|--uninstall|--help]"
        echo ""
        echo "  (no args)    Full install: build, create user, install, enable service"
        echo "  --update     Rebuild + reinstall + restart (skip user/dir creation)"
        echo "  --uninstall  Stop service, remove binary, web assets, user, config"
        echo ""
        echo "Paths:"
        echo "  Binary:   ${BIN_DIR}/planner-server"
        echo "  Web:      ${WEB_DIR}"
        echo "  Data:     ${DATA_DIR}"
        echo "  Config:   ${CONF_DIR}/planner.env"
        echo "  Service:  /etc/systemd/system/${SERVICE_NAME}.service"
        echo "  Logs:     journalctl -u ${SERVICE_NAME}"
        exit 0
        ;;
    "")
        require_root
        check_deps
        do_build
        do_install false
        info "Installation complete."
        ;;
    *)
        die "Unknown option: $1 (try --help)"
        ;;
esac
