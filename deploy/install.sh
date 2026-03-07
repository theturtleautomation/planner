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

    # Write a locked-down Gemini settings file.
    # tools.core: [] is an empty allowlist — Planner uses Gemini purely for
    # text completions and never wants Gemini CLI tool execution.
    #
    # Gemini CLI v0.32.1 still serializes an invalid empty `tools` payload on
    # the OAuth / Code Assist path even when tools.core is empty. The install
    # step patches the installed client after npm install so an empty
    # declaration set omits the `tools` field entirely.
    # NOTE: Do NOT add tools.exclude: ["*"] — that caused a different 400.
    # The Policy Engine TOML below is belt-and-suspenders for runtime denial.
    cat > "${cli_home}/gemini/settings.json" << 'GEMINI_SETTINGS'
{
  "tools": {
    "core": []
  },
  "security": {
    "auth": {
      "selectedType": "oauth-personal"
    },
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

    # Gemini Policy Engine — deny all tools by default.
    # Planner invokes `gemini` in non-interactive mode with a specific
    # prompt; it does not need the CLI to execute tools on its own.
    # Policy files in ~/.gemini/policies/ are loaded at User tier (priority base 4).
    mkdir -p "${cli_home}/gemini/.gemini/policies"
    cat > "${cli_home}/gemini/.gemini/policies/planner-lockdown.toml" << 'GEMINI_POLICY'
# Planner service lockdown — deny all built-in and MCP tools.
# The Gemini CLI is used purely for LLM completion, not tool execution.
[[rule]]
toolName = "*"
decision = "deny"
priority = 999
GEMINI_POLICY

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

    # Claude CLI requires the CWD to be a git repo ("trusted directory").
    # Initialize as a git repo if not already one.
    if [[ ! -d "${sandbox}/.git" ]]; then
        info "  Initializing git repo in CLI sandbox (Claude CLI requires trusted directory)..."
        git -C "${sandbox}" init -q
        git -C "${sandbox}" config user.email "planner@localhost"
        git -C "${sandbox}" config user.name "Planner"
        git -C "${sandbox}" commit --allow-empty -q -m "planner: cli sandbox init"
    fi

    # Own everything by the service user
    chown -R "${SERVICE_USER}:${SERVICE_USER}" "${cli_home}"
    chown -R "${SERVICE_USER}:${SERVICE_USER}" "${sandbox}"

    info "CLI isolation ready: ${cli_home}"
}

# ---------------------------------------------------------------------------
# LLM CLI installation
# ---------------------------------------------------------------------------
# The Rust server's CliEnvironment uses env_clear() and sets a controlled
# PATH that includes /opt/planner/bin/. We install CLI binaries there so
# the service user can find them.
#
# Installation strategy per provider:
#
#   claude — Native binary (self-contained, no Node.js).
#            1. Check if invoking user has it at ~/.local/bin/claude
#            2. If found, copy it to /opt/planner/bin/claude
#            3. If not, download via Anthropic's native installer
#            The npm package (@anthropic-ai/claude-code) has known issues
#            with postinstall scripts, EEXIST errors, and broken symlinks.
#            Anthropic recommends the native installer as of late 2025.
#
#   gemini — npm install -g --prefix /opt/planner @google/gemini-cli
#   codex  — npm install -g --prefix /opt/planner @openai/codex
#
# The gemini/codex npm wrapper scripts use #!/usr/bin/env node, so we
# also copy the node binary into /opt/planner/bin/ (env_clear() strips
# the original PATH).
#
install_llm_clis() {
    info "Installing LLM CLI tools into ${INSTALL_DIR}/bin..."

    local planner_bin="${INSTALL_DIR}/bin"
    mkdir -p "${planner_bin}"

    local invoking_user="${SUDO_USER:-}"
    local user_home=""
    if [[ -n "$invoking_user" ]] && [[ "$invoking_user" != "root" ]]; then
        user_home=$(eval echo ~"${invoking_user}")
    fi

    local found=0

    installed_npm_package_version() {
        local pkg="$1"
        local package_json="${INSTALL_DIR}/lib/node_modules/${pkg}/package.json"
        [[ -f "${package_json}" ]] || return 1
        "${node_cmd}" -e "const pkg = require(process.argv[1]); process.stdout.write(pkg.version || '');" "${package_json}" 2>/dev/null
    }

    latest_npm_package_version() {
        local pkg="$1"
        "${npm_cmd}" view "${pkg}" version 2>/dev/null | tail -n 1 | tr -d '[:space:]'
    }

    patch_gemini_cli_empty_tools_bug() {
        local gemini_pkg_dir="${INSTALL_DIR}/lib/node_modules/@google/gemini-cli"
        local gemini_pkg_json="${gemini_pkg_dir}/package.json"
        local client_js="${gemini_pkg_dir}/node_modules/@google/gemini-cli-core/dist/src/core/client.js"

        if [[ ! -f "${gemini_pkg_json}" ]] || [[ ! -f "${client_js}" ]]; then
            warn "  ! gemini installed without expected package layout — skipping Planner compatibility patch"
            return 0
        fi

        local gemini_version=""
        gemini_version=$("${node_cmd}" -e "const pkg = require(process.argv[1]); process.stdout.write(pkg.version || '');" "${gemini_pkg_json}" 2>/dev/null || true)

        if grep -q 'toolDeclarations.length > 0 ?' "${client_js}" 2>/dev/null; then
            info "  ✓ gemini compatibility patch already present (${gemini_version:-unknown version})"
            return 0
        fi

        if ! grep -q 'const tools = \[{ functionDeclarations: toolDeclarations }\];' "${client_js}" 2>/dev/null; then
            warn "  ! gemini client.js layout changed (${gemini_version:-unknown version}) — skipping empty-tools patch"
            return 0
        fi

        if "${node_cmd}" - "${client_js}" <<'NODE'
const fs = require('fs');
const target = process.argv[2];
let text = fs.readFileSync(target, 'utf8');
const replacements = [
  [
    'const tools = [{ functionDeclarations: toolDeclarations }];',
    'const tools = toolDeclarations.length > 0 ? [{ functionDeclarations: toolDeclarations }] : undefined;',
  ],
  [
    'return [{ functionDeclarations: toolDeclarations }];',
    'return toolDeclarations.length > 0 ? [{ functionDeclarations: toolDeclarations }] : undefined;',
  ],
];
let changed = false;
for (const [from, to] of replacements) {
  if (text.includes(from)) {
    text = text.split(from).join(to);
    changed = true;
  }
}
if (!changed) {
  process.exit(2);
}
fs.writeFileSync(target, text);
NODE
        then
            info "  ✓ patched gemini empty-tools bug (${gemini_version:-unknown version})"
        else
            warn "  ✗ failed to patch gemini empty-tools bug (${gemini_version:-unknown version})"
        fi
    }

    # ---------------------------------------------------------------
    # Claude — native binary (no Node.js required)
    # ---------------------------------------------------------------
    if [[ -x "${planner_bin}/claude" ]]; then
        info "  ✓ claude already installed at ${planner_bin}/claude — skipping"
        found=$((found + 1))
    else
    info "  Installing claude (native binary)..."
    local claude_src=""

    # Strategy 1: Copy from invoking user's native install
    if [[ -n "$user_home" ]] && [[ -x "${user_home}/.local/bin/claude" ]]; then
        claude_src="${user_home}/.local/bin/claude"
        info "    Found native install at ${claude_src}"
    fi

    # Strategy 2: Check system-wide locations
    if [[ -z "$claude_src" ]]; then
        for candidate in /usr/local/bin/claude /usr/bin/claude; do
            if [[ -x "$candidate" ]]; then
                claude_src="$candidate"
                info "    Found at ${claude_src}"
                break
            fi
        done
    fi

    # Strategy 3: Check if invoking user has it anywhere on PATH
    if [[ -z "$claude_src" ]] && [[ -n "$invoking_user" ]]; then
        local user_claude
        user_claude=$(sudo -u "$invoking_user" bash -lc "command -v claude 2>/dev/null" 2>/dev/null || true)
        if [[ -n "$user_claude" ]] && [[ -x "$user_claude" ]]; then
            claude_src="$user_claude"
            info "    Found on user PATH at ${claude_src}"
        fi
    fi

    if [[ -n "$claude_src" ]]; then
        # Resolve to actual file (follow symlinks)
        local claude_real
        claude_real=$(readlink -f "$claude_src" 2>/dev/null || echo "$claude_src")
        local dest_real=""
        [[ -e "${planner_bin}/claude" ]] && dest_real=$(readlink -f "${planner_bin}/claude" 2>/dev/null || true)

        if [[ -n "$dest_real" ]] && [[ "$claude_real" == "$dest_real" ]]; then
            info "  \u2713 claude already at ${planner_bin}/claude"
        else
            rm -f "${planner_bin}/claude"
            cp "$claude_real" "${planner_bin}/claude"
            chmod 755 "${planner_bin}/claude"
            info "  \u2713 claude copied \u2192 ${planner_bin}/claude"
        fi
        found=$((found + 1))
    else
        # Strategy 4: Download via native installer into a temp HOME,
        # then copy the binary out.
        info "    No existing claude binary found. Downloading native installer..."
        local tmp_claude_home
        tmp_claude_home=$(mktemp -d)
        if curl -fsSL https://claude.ai/install.sh | HOME="$tmp_claude_home" bash 2>/dev/null; then
            if [[ -x "${tmp_claude_home}/.local/bin/claude" ]]; then
                rm -f "${planner_bin}/claude"
                cp "${tmp_claude_home}/.local/bin/claude" "${planner_bin}/claude"
                chmod 755 "${planner_bin}/claude"
                info "  \u2713 claude downloaded \u2192 ${planner_bin}/claude"
                found=$((found + 1))
            else
                warn "  \u2717 claude native installer ran but binary not found"
            fi
        else
            warn "  \u2717 claude native installer failed (no internet or download error)"
        fi
        rm -rf "$tmp_claude_home"
    fi

    fi  # end claude skip-if-exists

    # ---------------------------------------------------------------
    # Gemini + Codex — npm install
    # ---------------------------------------------------------------
    # Locate npm + node.  nvm does not load in non-interactive shells
    # (bash -lc won't source .bashrc behind an interactivity guard).
    # We explicitly source nvm.sh if present in the invoking user's home.
    local npm_cmd="npm"
    local node_cmd="node"

    if [[ -n "$user_home" ]]; then
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

    declare -A npm_packages=(
        [gemini]="@google/gemini-cli"
        [codex]="@openai/codex"
    )

    for cli in gemini codex; do
        local pkg="${npm_packages[$cli]}"

        # Install or update npm-based CLIs when the installed version is
        # behind npm latest.
        if [[ -x "${planner_bin}/${cli}" ]]; then
            local installed_version=""
            local latest_version=""
            installed_version=$(installed_npm_package_version "${pkg}" || true)
            latest_version=$(latest_npm_package_version "${pkg}" || true)

            if [[ -n "${installed_version}" ]] && [[ -n "${latest_version}" ]]; then
                if [[ "${installed_version}" == "${latest_version}" ]]; then
                    info "  ✓ ${cli} already installed at ${planner_bin}/${cli} (${installed_version}, latest)"
                    found=$((found + 1))
                    continue
                fi
                info "  Updating ${cli} ${installed_version} → ${latest_version}..."
            elif [[ -n "${installed_version}" ]]; then
                warn "  ! Could not verify latest ${cli} version — keeping installed ${installed_version}"
                found=$((found + 1))
                continue
            else
                warn "  ! ${cli} binary exists but installed version is unknown — reinstalling"
            fi
        fi

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

    if [[ -x "${planner_bin}/gemini" ]]; then
        patch_gemini_cli_empty_tools_bug
    fi

    # Ensure node is available in planner's bin dir.
    # The npm CLI wrapper scripts (gemini, codex) use #!/usr/bin/env node —
    # with our controlled PATH, 'env' will find node at /opt/planner/bin/node.
    # Claude's native binary does NOT need node.
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
        warn "  \u2717 Could not locate node binary — gemini/codex CLI tools may not work"
    fi

    # Fix ownership — everything under INSTALL_DIR should be accessible
    chown -R "${SERVICE_USER}:${SERVICE_USER}" "${INSTALL_DIR}/bin" 2>/dev/null || true
    chown -R "${SERVICE_USER}:${SERVICE_USER}" "${INSTALL_DIR}/lib" 2>/dev/null || true

    if [[ $found -eq 0 ]]; then
        echo ""
        warn "\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550"
        warn "  NO LLM CLI TOOLS INSTALLED"
        warn "\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550"
        warn ""
        warn "  Install manually:"
        warn "    claude:  curl -fsSL https://claude.ai/install.sh | bash"
        warn "             sudo cp ~/.local/bin/claude ${planner_bin}/claude"
        warn "    gemini:  sudo npm install -g --prefix ${INSTALL_DIR} @google/gemini-cli"
        warn "    codex:   sudo npm install -g --prefix ${INSTALL_DIR} @openai/codex"
        warn "  Then re-run: sudo $0 --update"
        warn ""
    else
        info "  ${found} provider(s) installed into ${planner_bin}"
    fi

    echo ""
    info "CLI isolation is handled automatically by the Rust server."
    info "Each CLI runs in a clean environment with no MCP servers,"
    info "no plugins, and no project-level config inheritance."
}

# ---------------------------------------------------------------------------
# LLM auth verification — check each installed CLI has valid credentials
# ---------------------------------------------------------------------------
# Checks for subscription-auth credential files. If missing, the service will
# fail at runtime when it tries to call that CLI.
#
check_llm_auth() {
    info "Verifying LLM authentication..."
    echo ""

    local planner_bin="${INSTALL_DIR}/bin"
    local cli_home="${INSTALL_DIR}/cli-home"
    local conf="${CONF_DIR}/planner.env"
    local authed=0
    local installed=0
    local unauthenticated=()

    # --- Claude ---
    if [[ -x "${planner_bin}/claude" ]]; then
        installed=$((installed + 1))
        if [[ -d "${cli_home}/claude/.claude" ]] &&              find "${cli_home}/claude/.claude" -name "*.json" -size +0c 2>/dev/null | grep -q .; then
            info "  \u2713 claude  — credentials found in ${cli_home}/claude/"
            authed=$((authed + 1))
        else
            warn "  \u2717 claude  — NOT AUTHENTICATED"
            unauthenticated+=(claude)
        fi
    fi

    # --- Gemini ---
    if [[ -x "${planner_bin}/gemini" ]]; then
        installed=$((installed + 1))
        if [[ -d "${cli_home}/gemini/.gemini" ]] &&              find "${cli_home}/gemini/.gemini" -name "*.json" -size +0c 2>/dev/null | grep -q .; then
            info "  \u2713 gemini  — credentials found in ${cli_home}/gemini/"
            authed=$((authed + 1))
        else
            warn "  \u2717 gemini  — NOT AUTHENTICATED"
            unauthenticated+=(gemini)
        fi
    fi

    # --- Codex ---
    if [[ -x "${planner_bin}/codex" ]]; then
        installed=$((installed + 1))
        if [[ -f "${cli_home}/codex/.codex/auth.json" ]] &&              [[ -s "${cli_home}/codex/.codex/auth.json" ]]; then
            info "  \u2713 codex   — credentials found in ${cli_home}/codex/"
            authed=$((authed + 1))
        else
            warn "  \u2717 codex   — NOT AUTHENTICATED"
            unauthenticated+=(codex)
        fi
    fi

    echo ""

    if [[ ${#unauthenticated[@]} -gt 0 ]]; then
        warn "\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550"
        warn "  ${#unauthenticated[@]} PROVIDER(S) NEED AUTHENTICATION"
        warn "\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550\u2550"
        echo ""
        info "Option A — Interactive login (run from a terminal with browser access):"
        echo ""
        for cli in "${unauthenticated[@]}"; do
            case "$cli" in
                claude) warn "  sudo -u ${SERVICE_USER} HOME=${cli_home}/claude ${planner_bin}/claude login" ;;
                gemini) warn "  sudo -u ${SERVICE_USER} HOME=${cli_home}/gemini ${planner_bin}/gemini auth login" ;;
                codex)  warn "  sudo -u ${SERVICE_USER} HOME=${cli_home}/codex CODEX_HOME=${cli_home}/codex/.codex ${planner_bin}/codex login" ;;
            esac
        done
        echo ""
    elif [[ $installed -gt 0 ]]; then
        info "  All ${authed} installed provider(s) are authenticated."
    fi
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
    local default_worktree_root="${DATA_DIR}/worktrees"
    local legacy_worktree_root="/tmp/planner-worktrees"

    # Create service user (skip on update)
    if [[ "$update_only" == "false" ]]; then
        if ! id "${SERVICE_USER}" &>/dev/null; then
            info "Creating service user: ${SERVICE_USER}"
            useradd --system --shell /usr/sbin/nologin --home-dir "${INSTALL_DIR}" "${SERVICE_USER}"
        fi
    fi

    # Create extra directories for CLI tools and config
    mkdir -p "${INSTALL_DIR}" "${WEB_DIR}" "${DATA_DIR}" "${CONF_DIR}"
    install -d -o "${SERVICE_USER}" -g "${SERVICE_USER}" -m 750 "${default_worktree_root}"
    install -d -o "${SERVICE_USER}" -g "${SERVICE_USER}" -m 750 "${legacy_worktree_root}"

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

    # Verify authentication for each installed CLI
    check_llm_auth
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
