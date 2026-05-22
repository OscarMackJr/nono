#!/usr/bin/env bash
# scripts/regenerate-tuf-test-fixtures.sh
#
# Regenerate the hermetic TUF test fixtures for Phase 50.
#
# Codex R-50-08: committing this script collapses the previous
# fixture-strategy `checkpoint:decision` task into a planning-time
# decision. Checked-in fixtures + a committed regen script are more
# stable than `tough::editor` in test code and don't block on human
# input.
#
# Outputs:
#   - crates/nono-cli/tests/fixtures/tuf-repo-happy/
#       1.root.json
#       1.snapshot.json
#       1.targets.json
#       timestamp.json
#       targets/<sha256>.trusted_root.json   (consistent-snapshot layout)
#   - crates/nono-cli/tests/fixtures/tuf-repo-bad-sig/
#       (copy of tuf-repo-happy with one byte flipped in
#        1.root.json's first signature -> tough rejects with sig error)
#   - crates/nono-cli/tests/fixtures/tuf-repo-malformed/
#       (copy of tuf-repo-happy with 1.root.json truncated to 100 bytes
#        -> tough rejects with parse error)
#   - crates/nono-cli/tests/fixtures/tuf/trusted_root_baseline.json
#       (Codex R-50-03: serde_json::to_string_pretty output of
#        TrustedRoot::from_json(<happy fixture trusted_root.json bytes>).
#        Test 4 (cache_bytes_match_baseline) asserts byte-equality
#        against this baseline, NOT against the raw fixture bytes —
#        the chain-walk's produced bytes must match this captured
#        upstream serialization, which is the SPEC Req 4 byte-identical
#        cache contract.)
#
# Usage:
#   bash scripts/regenerate-tuf-test-fixtures.sh
#
# Prerequisites:
#   - tuftool (install via `cargo install tuftool`)
#   - python3 (for byte-flip + truncate variants)
#   - Rust 1.85+ (for the throwaway baseline-gen binary)
#
# The script is idempotent: it deletes existing fixture directories
# before regenerating, so no stale state leaks through.

set -euo pipefail

# Resolve repo root from the script's location so the script can be
# invoked from any cwd.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

FIXTURE_ROOT="${REPO_ROOT}/crates/nono-cli/tests/fixtures"
HAPPY_DIR="${FIXTURE_ROOT}/tuf-repo-happy"
BAD_SIG_DIR="${FIXTURE_ROOT}/tuf-repo-bad-sig"
MALFORMED_DIR="${FIXTURE_ROOT}/tuf-repo-malformed"
BASELINE_DIR="${FIXTURE_ROOT}/tuf"
BASELINE_FILE="${BASELINE_DIR}/trusted_root_baseline.json"

# Source target file: a real TrustedRoot JSON that exercises the
# upstream parser/serializer end-to-end. Phase 32 already shipped this
# fixture in nono/tests/fixtures so we reuse it.
SOURCE_TARGET="${REPO_ROOT}/crates/nono/tests/fixtures/trust-root-frozen.json"

# Prerequisites check.
command -v tuftool >/dev/null 2>&1 || {
  echo "FATAL: tuftool not found on PATH. Install via: cargo install tuftool" >&2
  exit 1
}
command -v python3 >/dev/null 2>&1 || {
  echo "FATAL: python3 not found on PATH (needed for byte-flip + truncate variants)" >&2
  exit 1
}
[ -f "${SOURCE_TARGET}" ] || {
  echo "FATAL: source target file missing: ${SOURCE_TARGET}" >&2
  exit 1
}

# Work in a tempdir so we never pollute the repo with private keys or
# scratch artifacts. Deleted on exit.
GEN_DIR="$(mktemp -d -t nono-tuf-gen-XXXXXX)"
trap 'rm -rf "${GEN_DIR}"' EXIT

echo "[regen] working dir: ${GEN_DIR}"

cd "${GEN_DIR}"
mkdir -p keys targets-input repo
cp "${SOURCE_TARGET}" targets-input/trusted_root.json

# Step A: tuftool root scaffolding.
echo "[regen] initializing root.json"
tuftool root init root.json
tuftool root expire root.json 'in 520 weeks'   # ~10 years; tuftool requires
                                                # weeks/days/hours units
for ROLE in root snapshot targets timestamp; do
  tuftool root set-threshold root.json "${ROLE}" 1
done

# Single RSA key signs all four roles. `gen-rsa-key` adds the key to
# each --role in one call (it both generates and assigns).
echo "[regen] generating single-key RSA, attaching to all 4 roles"
tuftool root gen-rsa-key root.json keys/root.pem --bits 2048 \
  --role root --role snapshot --role targets --role timestamp

echo "[regen] signing root.json"
tuftool root sign root.json --key keys/root.pem

# Step B: tuftool create -> emits 1.root.json / 1.snapshot.json /
# 1.targets.json / timestamp.json under repo/metadata/ and a
# <sha256>.trusted_root.json symlink under repo/targets/.
echo "[regen] creating TUF repo"
tuftool create \
  --root root.json \
  --key keys/root.pem \
  --add-targets targets-input \
  --targets-expires 'in 520 weeks' \
  --targets-version 1 \
  --snapshot-expires 'in 520 weeks' \
  --snapshot-version 1 \
  --timestamp-expires 'in 520 weeks' \
  --timestamp-version 1 \
  --outdir repo

# Step C: flatten the generated layout into the fixture root.
# Metadata files at top level, target under targets/. Resolve the
# consistent-snapshot symlink to the real file bytes so the fixture is
# portable across operating systems (Windows symlink semantics differ).
echo "[regen] flattening into ${HAPPY_DIR}"
rm -rf "${HAPPY_DIR}"
mkdir -p "${HAPPY_DIR}/targets"
cp repo/metadata/*.json "${HAPPY_DIR}/"
for f in repo/targets/*; do
  cp -L "$f" "${HAPPY_DIR}/targets/$(basename "$f")"
done

# Step D: bad-sig + malformed variants.
echo "[regen] generating bad-sig variant"
rm -rf "${BAD_SIG_DIR}"
cp -r "${HAPPY_DIR}" "${BAD_SIG_DIR}"
python3 - "${BAD_SIG_DIR}/1.root.json" <<'PY'
import json, sys
path = sys.argv[1]
with open(path, 'rb') as f:
    raw = f.read()
d = json.loads(raw)
sig = d['signatures'][0]['sig']
# Flip a byte so tough's signature verification rejects.
d['signatures'][0]['sig'] = ('00' + sig[2:]) if sig[:2] != '00' else ('aa' + sig[2:])
# Write LF-only (binary mode) with explicit trailing newline so the
# fixture's byte size matches across platforms (Windows python text mode
# otherwise emits CRLF + omits the trailing newline, which causes spurious
# diffs in git index and breaks cross-OS CI).
serialized = json.dumps(d, indent=2).encode('utf-8') + b'\n'
with open(path, 'wb') as f:
    f.write(serialized)
print(f'flipped signature byte in {path}')
PY

echo "[regen] generating malformed variant"
rm -rf "${MALFORMED_DIR}"
cp -r "${HAPPY_DIR}" "${MALFORMED_DIR}"
python3 - "${MALFORMED_DIR}/1.root.json" <<'PY'
import sys
path = sys.argv[1]
with open(path, 'rb') as f:
    data = f.read()
# Truncate to 100 bytes -> invalid JSON, tough's parser rejects.
with open(path, 'wb') as f:
    f.write(data[:100])
print(f'truncated {path} to 100 bytes')
PY

# Step E (R-50-03): captured-upstream baseline. Build a throwaway
# cargo project that calls
#   serde_json::to_string_pretty(&TrustedRoot::from_json(<bytes>))
# against the happy fixture's target file. The OUTPUT bytes are what
# Test 4 (cache_bytes_match_baseline) asserts the chain-walk produces
# byte-identically.
echo "[regen] generating baseline via throwaway baseline-gen binary"
mkdir -p baseline-gen/src
cat > baseline-gen/Cargo.toml <<'TOML'
[package]
name = "baseline-gen"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
sigstore-verify = "0.7.0"
serde_json = "1"
TOML
cat > baseline-gen/src/main.rs <<'RUST'
use sigstore_verify::trust_root::TrustedRoot;
use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: baseline-gen <input-trusted-root-json> <output-baseline-json>");
        std::process::exit(1);
    }
    let json = fs::read_to_string(&args[1])?;
    let parsed: TrustedRoot = TrustedRoot::from_json(&json)?;
    let serialized = serde_json::to_string_pretty(&parsed)?;
    fs::write(&args[2], serialized.as_bytes())?;
    println!("baseline written to {}", &args[2]);
    Ok(())
}
RUST

# Find the (single) target file under repo/targets and pass it as the
# input to baseline-gen. We dereference the symlink with cp -L upstream;
# here we run against the generated repo's target file (same bytes as
# the project fixture).
HAPPY_TARGET_FILE="$(find "${HAPPY_DIR}/targets" -type f -name '*.trusted_root.json' | head -1)"
[ -n "${HAPPY_TARGET_FILE}" ] || {
  echo "FATAL: could not locate happy target file under ${HAPPY_DIR}/targets" >&2
  exit 1
}

mkdir -p "${BASELINE_DIR}"
( cd baseline-gen && cargo run --release -- "${HAPPY_TARGET_FILE}" "${BASELINE_FILE}" )

echo "[regen] DONE."
echo "  - ${HAPPY_DIR}/"
echo "  - ${BAD_SIG_DIR}/"
echo "  - ${MALFORMED_DIR}/"
echo "  - ${BASELINE_FILE}"
