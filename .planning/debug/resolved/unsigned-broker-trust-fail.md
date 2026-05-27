---
slug: unsigned-broker-trust-fail
status: resolved
trigger: "nono run --profile claude-code -- claude --version fails on Windows: Trust verification failed for C:\\Program Files\\nono\\nono.exe — Authenticode status is Unsigned (expected Valid). Self-trust-anchor unavailable; refusing to spawn broker."
created: 2026-05-26
updated: 2026-05-26
---

# Debug Session: unsigned-broker-trust-fail

## Symptoms

**Expected behavior:**
`nono run --profile claude-code -- claude --version` should apply the Windows
sandbox and spawn the Low-IL broker, then run `claude --version` inside the
sandbox. (The supervised Windows path spawns a broker process — see Phase 31
shell-broker and Phase 51/52 supervised-run hardening.)

**Actual behavior:**
Sandbox application proceeds (label-guard warnings are emitted for several
pre-existing-mandatory-label paths, which is benign), then execution fails at
broker spawn during the "shutting-down" phase:

```
nono: Sandbox initialization failed: Windows supervised execution failed during
shutting-down (session: supervised-11652-1779848657452457000,
transport: windows-supervisor-anon-11652-9919927aa2d68628754749e06f650bd6,
supervisor_audit_entries: 0): Trust verification failed for
C:\Program Files\nono\nono.exe: nono.exe Authenticode status is Unsigned
(expected Valid). Self-trust-anchor unavailable; refusing to spawn broker.
```

**Error messages:**
- `Trust verification failed for C:\Program Files\nono\nono.exe`
- `nono.exe Authenticode status is Unsigned (expected Valid)`
- `Self-trust-anchor unavailable; refusing to spawn broker.`
- (benign) repeated `WARN label guard: path has pre-existing mandatory-label ACE; skipping apply + revert`
- (benign) `WARN Profile policy path '$HOME/Library/Keychains' does not exist, skipping` — macOS path leaking into Windows profile resolution; not the failure, but worth noting.

**Timeline / source:**
- nono v0.57.2, installed from the **locally-built (unsigned) MSI** at
  `C:\Program Files\nono\nono.exe`.
- Known context (memory `project_release_yml_broken`): shipped MSIs are
  locally-built + UNSIGNED because the release.yml docker reusable-call job was
  broken; signing pipeline not yet exercised. So the binary being Unsigned is
  expected; the question is the trust gate's handling of it.

**Reproduction:**
```
cd C:\Users\OMack\nono-poc
nono run --profile claude-code -- claude --version
# answer y to the --allow-cwd prompt
```
Platform: Windows 11 Enterprise build 26200.

## Goal / Decision Frame

User wants **both**: (1) root-cause WHY the self-trust-anchor fallback is
"unavailable" for an unsigned local build, then (2) decide whether the fix is
to make the self-trust-anchor fallback work for unsigned dev/local builds, or
to treat MSI Authenticode signing as the real fix and confirm the refusal is
correct fail-secure behavior.

Security note: this is a security-critical trust boundary (broker spawn elevates
to Medium-IL on Windows). Any fix that loosens the trust gate must preserve
fail-secure semantics — do NOT silently trust arbitrary unsigned binaries.

## Current Focus

- hypothesis: CONFIRMED — There is NO self-trust-anchor "fallback" for unsigned
  production-layout binaries. The "self-trust-anchor" IS nono.exe's own
  Authenticode (subject+thumbprint); when nono.exe is Unsigned that anchor cannot
  be extracted, so the gate fail-closes. The only skip is a Cargo `target/`
  install-layout detector, which a `Program Files` MSI install does not match.
  The refusal is working exactly as designed (ADR D-32-12, fail-closed, no
  escape hatch).
- next_action: Present fix-decision checkpoint to user (sign the MSI = real fix,
  vs. broaden the dev-skip = security tradeoff).
- test: n/a (static code trace; no behavioral test needed to confirm)
- expecting: n/a

## Evidence

- timestamp: 2026-05-26 — Error reproduced by user (pasted terminal output above). Binary confirmed as locally-built unsigned MSI.
- timestamp: 2026-05-26 — Located the exact error string. `crates/nono-cli/src/exec_strategy_windows/launch.rs:2014-2028` `verify_broker_authenticode()`: when `query_authenticode_status(nono_exe)` returns anything other than `AuthenticodeStatus::Valid {..}`, it returns `NonoError::TrustVerification { path, reason: "nono.exe Authenticode status is {other:?} (expected Valid). Self-trust-anchor unavailable; refusing to spawn broker." }`. For the unsigned MSI binary, `other` = `Unsigned`.
- timestamp: 2026-05-26 — The gate is invoked at TWO dispatch sites, both guarded identically: `launch.rs:1341` (`WindowsTokenArm::BrokerLaunch`, PTY path, Phase 31/32) and `launch.rs:1664` (`BrokerLaunchNoPty`, no-PTY path, Phase 51 T-51C-04). Both call `verify_broker_authenticode(&nono_exe, &broker_path)?` only when `!is_dev_build_layout(&nono_exe)`.
- timestamp: 2026-05-26 — `is_dev_build_layout()` (launch.rs:1984-1990) is the ONLY skip path. It returns true ONLY if the exe path string contains `\target\debug\`, `\target\release\`, `/target/debug/`, or `/target/release/`. `C:\Program Files\nono\nono.exe` matches none → gate runs unconditionally. There is NO env-var override and NO CLI flag (grep confirms; ADR D-32-12 states "NO escape-hatch flag, NO env-var override").
- timestamp: 2026-05-26 — `query_authenticode_status` (exec_identity_windows.rs:119-210): returns `AuthenticodeStatus::Unsigned` when `WinVerifyTrust` returns `TRUST_E_NOSIGNATURE` (0x800B0100). An unsigned PE produces exactly this. So `nono_status` = `Unsigned`, never `Valid`, so the anchor (subject+thumbprint) is never extractable.
- timestamp: 2026-05-26 — Design intent confirmed by ADR `docs/architecture/broker-trust-anchor.md` (Phase 32, D-32-11..14). Option (d) chosen: "nono.exe extracts ITS OWN Authenticode signature, requires broker to match." The dev-skip is explicitly an install-layout substring detector (not `#[cfg(debug_assertions)]`, Pitfall 6). The ADR's "Diagnostic Surface" section: `nono setup --check-only` already prints `<Unsigned>` for this binary as the operator's advance warning that the gate will fail-closed (P32-CHK-003, implemented at setup.rs:1077/1088 `print_self_authenticode_status`).

## Eliminated

- "A self-trust-anchor fallback mechanism exists but is unavailable here" —
  ELIMINATED. There is no separate fallback. The self-trust-anchor literally IS
  nono.exe's own valid Authenticode signature. "Unavailable" means nono.exe is
  Unsigned, so there is no anchor to read. The wording in the error implies a
  fallback, but the code has none. (Investigation starting-point hypothesis
  disproven by reading launch.rs:2004-2060.)
- A pinned-hash / sidecar-file / build-feature anchor that the MSI fails to
  produce — ELIMINATED. No such mechanism exists in the code; options (a) baked
  constant, (b) config file, (c) sigstore bundle were all explicitly REJECTED in
  the ADR in favor of self-introspection (d).

## Root Cause

`verify_broker_authenticode` (launch.rs:2004) implements the Phase 32 D-32-13
"self-trust-anchor": nono.exe reads its OWN Authenticode signer subject +
thumbprint and requires the sibling `nono-shell-broker.exe` to match exactly.
The anchor only exists when nono.exe's own signature is `Valid`. The shipped
v0.57.2 MSI is UNSIGNED (the release.yml signing pipeline was never exercised —
memory `project_release_yml_broken`), so `query_authenticode_status(nono.exe)`
returns `Unsigned`. With no `Valid` anchor to extract, and the install at
`C:\Program Files\nono\` (production layout, not a Cargo `target/` dir so the
`is_dev_build_layout` skip does not fire), the gate fail-closes with
`NonoError::TrustVerification` and refuses to spawn the broker.

**This is correct, intended fail-secure behavior** (ADR D-32-12: fail-closed, no
escape hatch). It is not a bug in the trust gate; it is the trust gate
functioning as designed against an unsigned production-layout install. The real
defect is upstream: the MSI is unsigned because the signing pipeline is broken.

## Resolution

**Root cause:** Not a bug. `verify_broker_authenticode` (launch.rs:2004) fail-closes
because the locally-built v0.57.2 MSI at `C:\Program Files\nono\nono.exe` is UNSIGNED,
so there is no `Valid` Authenticode self-trust-anchor to extract, and the production
install layout does not match the `is_dev_build_layout` skip. This is the Phase 32
D-32-12 fail-secure design working as intended (no escape hatch).

**Decision (user, 2026-05-26):** "Both a1 + a2", refined after investigation:

- **a1 — unblock this machine: dev-layout run (chosen over local self-signing).**
  Investigation showed local self-signing would require importing a self-signed cert
  into machine-wide Trusted Root + Trusted Publisher stores (because the gate uses
  `WinVerifyTrust` full chain trust) — a real trust-surface cost requiring admin. The
  intended dev path is simpler and was chosen instead: build + run from a `target\release`
  layout, where `is_dev_build_layout()` skips the broker gate. No signing, no admin, no
  trust-store changes.
  - Built `target\release\nono.exe` + `target\release\nono-shell-broker.exe`
    (`cargo build --release -p nono-cli -p nono-shell-broker`, exit 0).
  - Run command for the user (from a profile-covered cwd, e.g. the poc dir):
    `C:\Users\OMack\nono\target\release\nono.exe run --profile claude-code -- claude --version`

- **a2 — release pipeline: NO code fix needed (re-scoped).** `.github/workflows/release.yml`
  already implements complete fail-closed Windows signing (check-secrets → sign
  nono.exe + broker + both MSIs via `scripts/sign-windows-artifacts.ps1` → verify
  Authenticode → upload). Shipped MSIs are unsigned only because the repo secrets
  `WINDOWS_SIGNING_CERT` / `WINDOWS_SIGNING_CERT_PASSWORD` are unset (and, historically,
  the now-fixed `5c90c4cf` startup_failure blocked the workflow). Remaining a2 work is
  OPERATIONAL (obtain a real CA code-signing cert, set the two repo secrets, verify on
  next `v*` tag push) — not a code change. See [[project_release_yml_broken]].

**Guard improvements applied (a2 "improve doc/guard"):**
- `crates/nono-cli/src/exec_strategy_windows/launch.rs` — the broker-gate failure message
  now points operators at the signing guide AND the dev-layout workaround (was a bare
  "Self-trust-anchor unavailable; refusing to spawn broker.").
- `docs/cli/development/windows-signing-guide.mdx` — new "Runtime symptom: unsigned install
  cannot spawn the broker" section connecting the runtime error to its two supported fixes
  (signed install via CI secret, or dev-layout run) and explaining why in-place self-signing
  is not a supported shortcut.

**files_changed:**
- crates/nono-cli/src/exec_strategy_windows/launch.rs (error-message guard)
- docs/cli/development/windows-signing-guide.mdx (runtime-symptom section)

**verification:** dev-layout build exit 0; nono-cli rebuild folds in the message edit;
no test pins the changed substring (broker_authenticode.rs asserts only the mismatch-branch
text, untouched). Functional confirmation of `claude --version` inside the dev-layout sandbox
is the user's to run (interactive: cwd prompt + claude network/login).
