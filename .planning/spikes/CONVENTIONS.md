# Spike Conventions

Patterns and stack choices established across nono spike sessions. New spikes follow these unless the
question requires otherwise.

## Stack

- **Standalone Rust binaries**, one crate per spike under `.planning/spikes/NNN-name/`, each with an empty
  `[workspace]` table in its `Cargo.toml` to stay OUT of the root nono workspace (the root globs members and
  will otherwise claim the spike → `cargo build` error).
- **`windows-sys` (v0.59, matching the workspace)** for raw Win32 (token, SID, label) spikes — mirror the
  exact calls in `crates/nono/src/sandbox/windows.rs` (e.g. `CreateWellKnownSid(WinLowLabelSid)` +
  `SetTokenInformation(TokenIntegrityLevel)`).
- **std-only** when the spike delegates confinement to `nono run` (no raw Win32 needed) — document the
  deviation in the README.

## Structure

- `NNN-descriptive-name/` with `Cargo.toml`, `src/main.rs`, `README.md` (YAML frontmatter: spike, name, type,
  validates, verdict, related, tags).
- Build artifacts (`target/`) are git-ignored; commit only `Cargo.toml`, `src/`, `README.md` + the MANIFEST.
- The binary prints `[SPIKE-NNN] ...` lines and a final `[SPIKE-NNN] VERDICT:` line.

## Patterns

- **Operator-run on real Win11:** OS-behavior facts (token IL, console subsystem, CLM, broker spawn) cannot be
  determined from the dev host / Bash tool — the spike compiles on the dev host, the operator runs it on real
  Win11 (build-26200) and pastes back the `[SPIKE-NNN]` output; the orchestrator records the verdict.
- **User-mode only** — no kernel driver / process-creation callbacks (out of scope per the WFP-driver
  placeholder pattern).
- **Confinement reuse:** prefer delegating to the proven `nono run` primitive over re-implementing token code,
  unless the spike's question IS the token code.
- **Setup gotchas to pre-empt in every confinement spike:** grant dir must be user-owned (`takeown`, R-B3);
  use a `network.block:false` profile variant unless testing WFP; dev-layout or signed `nono.exe` for the
  broker arm; absolute paths for grant-relative writes.

## Tools & Libraries

- `windows-sys = { version = "0.59", features = ["Win32_Foundation", "Win32_Security",
  "Win32_System_SystemServices", "Win32_System_Threading", ...] }` — add features per the APIs used.
- Drive non-cooperating engines via `std::process::Command` with piped stdin (e.g. `cmd.exe`) rather than
  building a cooperating victim.
