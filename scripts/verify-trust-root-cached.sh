#!/usr/bin/env bash
# Phase 49 REQ-POC-TRUST-03: Sigstore trusted-root cache smoke script.
#
# Usage:
#   scripts/verify-trust-root-cached.sh <path-to-candidate-trusted_root.json>
#
# Validates that `nono setup --from-file <CANDIDATE>` succeeds and produces
# a cache file byte-identical to the input. Exits 0 on success; non-zero
# on any failure. Maintainer-only (D-49-C3) — not wired into PR CI.
#
# Pre-commit gate for `.planning/templates/sigstore-rotation-refresh.md`
# Step 4. See that template for the full rotation-response procedure.

set -euo pipefail

if [ $# -lt 1 ]; then
  echo "usage: $0 <path-to-candidate-trusted_root.json>" >&2
  exit 2
fi

CANDIDATE="$1"
if [ ! -f "$CANDIDATE" ]; then
  echo "ERROR: candidate path does not exist or is not a file: $CANDIDATE" >&2
  exit 2
fi

TMP=$(mktemp -d -t nono-trust-root-smoke-XXXXXX)
trap 'rm -rf "$TMP"' EXIT
export NONO_TEST_HOME="$TMP"
export XDG_CONFIG_HOME="$TMP"
export NONO_NO_UPDATE_CHECK=1

echo "Running: nono setup --from-file $CANDIDATE"
nono setup --from-file "$CANDIDATE"

CACHE="$TMP/.nono/trust-root/trusted_root.json"
if [ ! -f "$CACHE" ]; then
  echo "ERROR: cache file was not created at $CACHE" >&2
  exit 1
fi

if ! cmp -s "$CANDIDATE" "$CACHE"; then
  echo "ERROR: cache file is not byte-identical to candidate" >&2
  echo "  candidate: $CANDIDATE" >&2
  echo "  cache:     $CACHE" >&2
  exit 1
fi

echo "PASS: $CANDIDATE accepted by 'nono setup --from-file' and cache is byte-identical."
