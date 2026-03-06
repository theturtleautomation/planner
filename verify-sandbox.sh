#!/usr/bin/env bash
# Planner sandbox environment verification
# Run inside the LXC container after enabling Landlock on PVE host

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "  ${GREEN}✓${NC} $1"; }
fail() { echo -e "  ${RED}✗${NC} $1"; }
warn() { echo -e "  ${YELLOW}!${NC} $1"; }

echo "=== Planner Sandbox Environment Check ==="
echo ""

# 1. Landlock ABI
echo "1. Landlock"
if [ -f /sys/kernel/security/landlock/abi_version ]; then
    abi=$(cat /sys/kernel/security/landlock/abi_version)
    pass "Landlock ABI version: ${abi}"
else
    fail "Landlock not available (/sys/kernel/security/landlock/abi_version missing)"
    echo "     Fix: on PVE host, add 'lsm=landlock,lockdown,yama,integrity,apparmor' to GRUB_CMDLINE_LINUX_DEFAULT"
fi
echo ""

# 2. bwrap
echo "2. Bubblewrap (bwrap)"
if command -v bwrap &>/dev/null; then
    bwrap_ver=$(bwrap --version 2>&1 || true)
    pass "bwrap found: ${bwrap_ver}"
    if bwrap --unshare-all \
        --ro-bind /usr /usr \
        --symlink usr/bin /bin \
        --symlink usr/lib /lib \
        --proc /proc \
        --dev /dev \
        -- true 2>/dev/null; then
        pass "bwrap --unshare-all probe: succeeded"
    else
        err=$(bwrap --unshare-all \
            --ro-bind /usr /usr \
            --symlink usr/bin /bin \
            --symlink usr/lib /lib \
            --proc /proc \
            --dev /dev \
            -- true 2>&1 || true)
        fail "bwrap --unshare-all probe: failed"
        echo "     ${err}"
    fi
else
    warn "bwrap not installed (optional if Landlock works)"
fi
echo ""

# 3. User namespaces
echo "3. User namespaces"
if [ -f /proc/sys/kernel/unprivileged_userns_clone ]; then
    val=$(cat /proc/sys/kernel/unprivileged_userns_clone)
    if [ "$val" = "1" ]; then
        pass "unprivileged_userns_clone = 1"
    else
        warn "unprivileged_userns_clone = ${val} (bwrap needs 1)"
    fi
else
    warn "/proc/sys/kernel/unprivileged_userns_clone not present (may be always-on)"
fi
echo ""

# 4. Codex CLI
echo "4. Codex CLI"
if command -v codex &>/dev/null; then
    codex_ver=$(codex --version 2>&1 || true)
    pass "codex found: ${codex_ver}"

    # Quick exec test with --full-auto (workspace-write + Landlock)
    tmpdir=$(mktemp -d /tmp/planner-sandbox-test.XXXXXX)
    mkdir -p "${tmpdir}"
    cd "${tmpdir}"
    git init --quiet
    echo "# test" > README.md
    git add -A && git commit -m "init" --quiet

    echo "   Testing: codex exec --full-auto ..."
    result=$(echo 'Create a file called hello.txt containing "sandbox works"' | \
        codex exec --full-auto --ephemeral -c project_doc_max_bytes=0 - 2>&1) && exit_code=$? || exit_code=$?

    if [ $exit_code -eq 0 ] && [ -f "${tmpdir}/hello.txt" ]; then
        pass "codex exec --full-auto: succeeded (file written)"
    else
        fail "codex exec --full-auto: failed (exit=${exit_code})"
        echo "     Output: ${result:0:300}"

        # Try danger-full-access as comparison
        echo "   Testing fallback: codex exec --sandbox danger-full-access ..."
        result2=$(echo 'Create a file called hello2.txt containing "fallback works"' | \
            codex exec --sandbox danger-full-access --ephemeral -c project_doc_max_bytes=0 - 2>&1) && exit_code2=$? || exit_code2=$?
        if [ $exit_code2 -eq 0 ] && [ -f "${tmpdir}/hello2.txt" ]; then
            warn "danger-full-access works but --full-auto does not — Landlock may not be active"
        else
            fail "danger-full-access also failed (exit=${exit_code2})"
        fi
    fi

    rm -rf "${tmpdir}"
else
    fail "codex not found on PATH"
fi
echo ""

# 5. Stale probe cache
echo "5. Planner probe cache"
cache_file="${HOME}/.cache/planner/sandbox-probe"
if [ -f "${cache_file}" ]; then
    content=$(cat "${cache_file}")
    if echo "${content}" | grep -q "danger-full-access\|workspace-write"; then
        warn "Stale cache detected: ${cache_file}"
        echo "     Contents: ${content}"
        echo "     Run: rm -f ${cache_file}"
    else
        pass "Cache exists: $(tail -1 "${cache_file}")"
    fi
else
    pass "No stale cache (will re-probe on next run)"
fi
echo ""

# Summary
echo "=== Summary ==="
if [ -f /sys/kernel/security/landlock/abi_version ]; then
    echo -e "${GREEN}Landlock is available. Planner will use --full-auto (workspace-write).${NC}"
    echo "No danger-full-access required."
else
    echo -e "${RED}Landlock is NOT available. Planner will fall back to danger-full-access.${NC}"
    echo "Enable Landlock on the PVE host to fix this."
fi
