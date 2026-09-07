#!/bin/bash
# Built-in Profile Tests
# Verifies built-in profiles load, produce correct dry-run output, and enforce expected policies

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/test_helpers.sh"

echo ""
echo -e "${BLUE}=== Profile Tests ===${NC}"

verify_nono_binary
if ! require_working_sandbox "profiles suite"; then
    print_summary
    exit 0
fi
NONO_BIN_ABS="$(cd "$(dirname "$NONO_BIN")" && pwd)/$(basename "$NONO_BIN")"

# Create test fixtures
TMPDIR=$(setup_test_dir)
trap 'cleanup_test_dir "$TMPDIR"' EXIT

mkdir -p "$TMPDIR/workdir"
echo "readable content" > "$TMPDIR/workdir/file.txt"

echo ""
echo "Test directory: $TMPDIR"
echo ""

# =============================================================================
# Profile Dry Run
# =============================================================================

echo "--- Profile Dry Run ---"

expect_success "claude-code profile dry-run exits 0" \
    "$NONO_BIN" run --profile claude-code --dry-run -- echo "test"

expect_success "codex profile dry-run exits 0" \
    "$NONO_BIN" run --profile codex --dry-run -- echo "test"

# NOTE (260907-arx): was `opencode`. Commit 2652e256 deliberately removed that
# profile from policy.json -- it moved to the registry pack
# always-further/opencode -- and said it would "substitute openclaw or swival in
# tests that previously used opencode". It updated the Rust tests and missed this
# shell suite, so Integration Tests has been red since. `openclaw` is the
# substitution that commit named.
expect_success "openclaw profile dry-run exits 0" \
    "$NONO_BIN" run --profile openclaw --dry-run -- echo "test"

expect_failure "nonexistent profile exits non-zero" \
    "$NONO_BIN" run --profile nonexistent-profile --dry-run -- echo "test"

expect_output_contains "claude-code profile lists .claude in dry-run" ".claude" \
    "$NONO_BIN" run --profile claude-code --dry-run -- echo "test"

expect_output_contains "codex profile lists .codex in dry-run" ".codex" \
    "$NONO_BIN" run --profile codex --dry-run -- echo "test"

# `openclaw` declares $HOME/.local/share/openclaw, the exact structural analogue
# of the OpenTUI data dir this assertion used to check for `opencode`, so the
# test keeps asserting the same property: a profile's declared data dir shows up
# in dry-run capabilities.
expect_output_contains "openclaw profile lists its data dir in dry-run" ".local/share/openclaw" \
    "$NONO_BIN" run --profile openclaw --dry-run -- echo "test"

expect_output_contains "dry-run output shows Capabilities section" "Capabilities:" \
    "$NONO_BIN" run --profile claude-code --dry-run -- echo "test"

# =============================================================================
# Profile Enforcement
# =============================================================================

echo ""
echo "--- Profile Enforcement ---"

# claude-code profile blocks rm by default
expect_failure "claude-code profile blocks rm" \
    "$NONO_BIN" run --profile claude-code --allow "$TMPDIR" -- rm "$TMPDIR/workdir/file.txt"

# Verify file still exists
run_test "file not deleted (rm was blocked by profile)" 0 test -f "$TMPDIR/workdir/file.txt"

# claude-code profile blocks pip by default
if command_exists pip; then
    expect_failure "claude-code profile blocks pip" \
        "$NONO_BIN" run --profile claude-code --allow "$TMPDIR" -- pip --version
else
    skip_test "claude-code profile blocks pip" "pip not installed"
fi

# claude-code profile allows cat on granted path
expect_success "claude-code profile allows cat on granted path" \
    "$NONO_BIN" run --profile claude-code --allow "$TMPDIR" -- cat "$TMPDIR/workdir/file.txt"

# codex profile allows cargo from user-local runtime paths
if command_exists cargo; then
    expect_success "codex profile allows cargo from runtime path" \
        bash -lc "cd \"$TMPDIR/workdir\" && \"$NONO_BIN_ABS\" run --profile codex --allow-cwd -- cargo --version"
else
    skip_test "codex profile allows cargo from runtime path" "cargo not installed"
fi

# =============================================================================
# Profile with Workdir
# =============================================================================

echo ""
echo "--- Profile with Workdir ---"

expect_success "profile with --workdir flag accepted" \
    "$NONO_BIN" run --profile claude-code --workdir "$TMPDIR/workdir" --dry-run -- echo "workdir test"

# With --allow-cwd and --workdir, the workdir should be accessible
expect_success "profile with --workdir and --allow-cwd accepted" \
    "$NONO_BIN" run --profile claude-code --workdir "$TMPDIR/workdir" --allow-cwd --dry-run -- echo "workdir test"

# =============================================================================
# Summary
# =============================================================================

print_summary
