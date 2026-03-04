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
# LLM CLI Isolation Setup
# ---------------------------------------------------------------------------
# Creates isolated home directories for each LLM CLI provider.
# Each gets its own HOME so auth credentials, config, and cache
# are completely separated from any user account.
#
# Directory layout:
#   /opt/planner/cli-home/
#     claude/           ← HOME for claude CLI
#       .claude/        ← auth + config (CLAUDE_CONFIG_DIR)
#     gemini/           ← HOME for gemini CLI
#       .gemini/        ← user-level settings
#       settings.json   ← system-level lockdown (no extensions/MCPs)
#     codex/            ← HOME for codex CLI
#       .codex/         ← auth + config (CODEX_HOME)
#       .config/        ← XDG_CONFIG_HOME
#       .local/         ← XDG_DATA_HOME parent
#       .cache/         ← XDG_CACHE_HOME
#   /opt/planner/cli-sandbox/   ← Empty CWD for all CLI invocations
#
setup_cli_isolation() {
    info "Setting up CLI isolation directories..."

    local cli_home="${INSTALL_DIR}/cli-home"
    local sandbox="${INSTALL_DIR}/cli-sandbox"

    # Claude
    mkdir -p "${cli_home}/claude/.claude"

    # Gemini
    mkdir -p "${cli_home}/gemini/.gemini"

    # Write a locked-down Gemini system settings file.
    # System settings have highest precedence and override user/project settings.
    cat > "${cli_home}/gemini/settings.json" << 'GEMINI_SETTINGS'
{
  "tools": {
    "core": [],
    "exclude": ["*"]
  },
  "security": {
    "disableYoloMode": true,
    "blockGitExtensions": true,
    "enablePermanentToolApproval": false
  },
  "hooksConfig": {
    "disabled": ["*"]
  },
  "admin": {
    "extensions": {
      "enabled": false
    }
  }
}
GEMINI_SETTINGS

    # Codex
    mkdir -p "${cli_home}/codex/.codex"
    mkdir -p "${cli_home}/codex/.config"
    mkdir -p "${cli_home}/codex/.local/share"
    mkdir -p "${cli_home}/codex/.cache"

    # Write a clean Codex config.toml with NO MCP servers.
    # If this file doesn't exist, codex may try to create one or
    # fall back to discovery that could find project-level configs.
    if [[ ! -f "${cli_home}/codex/.codex/config.toml" ]]; then
        cat > "${cli_home}/codex/.codex/config.toml" << 'CODEX_CONFIG'
# Planner service — clean Codex config.
# No MCP servers. No project-level config inheritance.
# Auth credentials are stored alongside this file by `codex login`.
CODEX_CONFIG
    fi

    # Sandbox directory — must be empty, used as CWD to prevent
    # project-level config discovery (.claude/, .gemini/, .codex/)
    mkdir -p "${sandbox}"

    # Own everything by the service user
    chown -R "${SERVICE_USER}:${SERVICE_USER}" "${cli_home}"
    chown -R "${SERVICE_USER}:${SERVICE_USER}" "${sandbox}"

    info "CLI isolation ready: ${cli_home}"
}

# ---------------------------------------------------------------------------
# LLM CLI installation — npm install into /opt/planner/{bin,lib}
# ---------------------------------------------------------------------------
# The Rust server's CliEnvironment uses env_clear() and sets a controlled
# PATH that includes /opt/planner/bin/. We install the CLI packages directly
# into /opt/planner/ using npm's --prefix flag, which places:
#   - binaries  → /opt/planner/bin/{claude,gemini,codex}
#   - packages  → /opt/planner/lib/node_modules/...
#
# This avoids symlink permission issues (service user can't traverse
# /home/<user>/) and makes the installation self-contained.
#
# We also copy the node binary into /opt/planner/bin/ so the npm wrapper
# scripts can find their runtime (env_clear() strips the original PATH).
#
install_llm_clis() {
    info "Installing LLM CLI packages into ${INSTALL_DIR}..."

    local planner_bin="${INSTALL_DIR}/bin"
    mkdir -p "${planner_bin}"

    # ---------------------------------------------------------------
    # Locate npm + node.
    # nvm does not load in non-interactive shells (bash -lc won't
    # source .bashrc behind an interactivity guard). We explicitly
    # source nvm.sh if present in the invoking user's home.
    # ---------------------------------------------------------------
    local npm_cmd="npm"
    local node_cmd="node"
    local invoking_user="${SUDO_USER:-}"

    if [[ -n "$invoking_user" ]] && [[ "$invoking_user" != "root" ]]; then
        local user_home
        user_home=$(eval echo ~"${invoking_user}")

        # Try sourcing nvm explicitly, then resolve npm/node
        local nvm_script="${user_home}/.nvm/nvm.sh"
        local nvm_source=""
        if [[ -s "$nvm_script" ]]; then
            nvm_source=". \"${nvm_script}\" --no-use 2>/dev/null; nvm use default >/dev/null 2>&1;"
        fi

        local user_npm
        user_npm=$(sudo -u "$invoking_user" bash -c "${nvm_source} command -v npm 2>/dev/null" 2>/dev/null || true)
        if [[ -n "$user_npm" ]]; then
            npm_cmd="$user_npm"
        fi
        local user_node
        user_node=$(sudo -u "$invoking_user" bash -c "${nvm_source} command -v node 2>/dev/null" 2>/dev/null || true)
        if [[ -n "$user_node" ]]; then
            node_cmd="$user_node"
        fi
    fi

    info "  Using npm: ${npm_cmd}"
    info "  Using node: ${node_cmd}"

    # Map of CLI name → npm package
    declare -A cli_packages=(
        [claude]="@anthropic-ai/claude-code"
        [gemini]="@google/gemini-cli"
        [codex]="@openai/codex"
    )

    local found=0

    for cli in claude gemini codex; do
        local pkg="${cli_packages[$cli]}"

        # Remove stale symlinks/files from prior installs to prevent EEXIST
        rm -f "${planner_bin}/${cli}"

        info "  Installing ${pkg}..."
        local npm_output
        if npm_output=$("${npm_cmd}" install -g --prefix "${INSTALL_DIR}" --force "${pkg}" 2>&1); then
            if [[ -x "${planner_bin}/${cli}" ]]; then
                info "  \u2713 ${cli} installed \u2192 ${planner_bin}/${cli}"
                found=$((found + 1))
            else
                warn "  \u2717 ${cli} package installed but binary not found at ${planner_bin}/${cli}"
                echo "$npm_output" | grep -i "error\|warn" | tail -10
            fi
        else
            warn "  \u2717 ${cli} installation failed"
            echo "$npm_output" | grep -i "error" | tail -10
        fi
    done

    # Ensure node is available in planner's bin dir.
    # The CLI wrapper scripts use #!/usr/bin/env node — with our controlled
    # PATH, 'env' will find node at /opt/planner/bin/node.
    local dest_node="${planner_bin}/node"
    local node_real
    node_real=$(readlink -f "$node_cmd" 2>/dev/null || true)
    if [[ -n "$node_real" ]] && [[ -x "$node_real" ]]; then
        local dest_real=""
        [[ -e "$dest_node" ]] && dest_real=$(readlink -f "$dest_node" 2>/dev/null || true)
        if [[ "$node_real" == "$dest_real" ]]; then
            info "  \u2713 node already at ${dest_node}"
        else
            rm -f "$dest_node"
            cp "$node_real" "$dest_node"
            chmod 755 "$dest_node"
            info "  \u2713 node copied \u2192 ${dest_node}"
        fi
    elif [[ -x "$dest_node" ]]; then
        info "  \u2713 node already at ${dest_node}"
    else
        warn "  \u2717 Could not locate node binary — CLI tools may not work"
    fi

    # Fix ownership — everything under INSTALL_DIR should be accessible
    chown -R "${SERVICE_USER}:${SERVICE_USER}" "${INSTALL_DIR}/bin" "${INSTALL_DIR}/lib" 2>/dev/null || true

    local cli_home="${INSTALL_DIR}/cli-home"

    if [[ $found -eq 0 ]]; then
        echo ""
        warn "\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550"
        warn "  NO LLM CLI TOOLS INSTALLED"
        warn "\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550"
        warn ""
        warn "  npm install failed. Try installing manually as your regular user:"
        warn "    sudo npm install -g --prefix ${INSTALL_DIR} @anthropic-ai/claude-code"
        warn "    sudo npm install -g --prefix ${INSTALL_DIR} @google/gemini-cli"
        warn "    sudo npm install -g --prefix ${INSTALL_DIR} @openai/codex"
        warn "  Then re-run: sudo $0 --update"
        warn ""
    else
        info "  ${found} provider(s) installed into ${planner_bin}"
    fi

    echo ""
    info "CLI isolation is handled automatically by the Rust server."
    info "Each CLI runs in a clean environment with no MCP servers,"
    info "no plugins, and no project-level config inheritance."
    echo ""
    info "To authenticate, run as the planner user with isolated HOME:"
    echo ""
    warn "  sudo -u planner HOME=${cli_home}/claude ${planner_bin}/claude login"
    warn "  sudo -u planner HOME=${cli_home}/gemini ${planner_bin}/gemini auth login"
    warn "  sudo -u planner HOME=${cli_home}/codex CODEX_HOME=${cli_home}/codex/.codex ${planner_bin}/codex login"
    echo ""
    info "Credentials are stored in ${cli_home}/<provider>/"
    info "and are isolated from any personal user accounts."
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

    # Create extra directories for CLI tools and config
    mkdir -p "${INSTALL_DIR}" "${WEB_DIR}" "${DATA_DIR}" "${CONF_DIR}"

    # Set up CLI isolation directories
    setup_cli_isolation

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

    # Install LLM CLIs into /opt/planner/bin/ and copy node runtime
    install_llm_clis
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
