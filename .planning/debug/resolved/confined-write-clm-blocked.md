---
slug: confined-write-clm-blocked
status: resolved
trigger: "Live Win11 finding (2026-06-13): the confined PowerShell spawned for Claude Code tool calls runs in Constrained Language Mode (CLM). CLM blocks .NET method calls ([Convert]::FromBase64String, [System.IO.File]::WriteAllText/ReadAllText, [System.Text.Encoding]) — which are exactly what the Phase-60 hook's Write/Edit/MultiEdit confined-write payloads are built from (claude_code_hook.rs:440/458/478). So a denied Write -> Bash retry cannot land a file under the affected arm. A nested Claude session inside the sandbox self-reported: 'PowerShell is in Constrained Language Mode, so .NET method calls ([Convert], [System.Text.Encoding]) are blocked ... The sandbox this shell runs in is an AppContainer.'"
created: 2026-06-13
updated: 2026-06-13
---

# Debug: confined-write .NET payloads blocked by Constrained Language Mode (R-A6)

## Symptoms

- **Expected:** a denied `Write`/`Edit`/`MultiEdit` is retried as a confined Bash call running the hook's `additionalContext` PowerShell, and the file operation LANDS inside the granted CWD.
- **Actual:** in a live Claude Code session on Win11, the confined PowerShell reported it was in **Constrained Language Mode**, blocking `[Convert]::FromBase64String`, `[System.IO.File]::WriteAllText`, `[System.Text.Encoding]` — the exact constructs the hook emits. The confined write does not complete.
- **Impact:** the Phase-60 confined-write vehicle is broken on the affected arm. Read/Bash-rewrite still work; Write/Edit/MultiEdit retries fail. Distinct from R-A1 (wrapper JSON integrity, fixed `f48ec206`) and a recurrence-class of F-60-UAT-05 (.NET-at-Low-IL), but with a NEW root (CLM, not BaseNamedObjects/WRITE_RESTRICTED).
- **Timeline:** surfaced during R-A1/R-B4 live verification (branch `fix/win-confinement-rb4-ra1`). 60-HUMAN-UAT (2026-06-01) recorded UAT 1 PASS with `[System.IO.File]::WriteAllText` landing a file — so this either regressed or the UAT exercised a DIFFERENT arm than live tool calls use (see hypothesis).
- **Reproduction:** in a real Claude Code session with the nono tool-wrapping hook (runner profile `windows_low_il_broker:true`, no PTY), trigger a `Write`; observe the Bash retry's `[Convert]`/`[System.IO.File]` payload fail under CLM.

## Current Focus

reasoning_checkpoint:
  hypothesis: "The Edit/MultiEdit confined payloads use `Get-Content -Raw -Encoding UTF8` -> `-creplace` -> `Set-Content -NoNewline -Encoding UTF8`. Under PS 5.1, `Set-Content -Encoding UTF8` on a STRING always prepends a UTF-8 BOM (EF BB BF) because PS 5.1 has no utf8NoBOM option. This corrupts every confined Edit/MultiEdit (leading BOM breaks shebangs/source; non-start edits rewrite the whole file with a spurious head BOM)."
  confirming_evidence:
    - "Orchestrator ran the exact EDIT vehicle under powershell.exe 5.1: produced `EF BB BF 66 6E 20 72 75 6E 28 29` = leading UTF-8 BOM on `fn run()`."
    - "Orchestrator ran the WRITE vehicle (byte-array literal + Set-Content -Encoding Byte): correct `68 69 C3 A9`, 4 bytes, NO BOM. The string-write path is the BOM source, not the byte-write path."
    - "Microsoft docs: PS 5.1 Set-Content/Out-File `-Encoding UTF8` writes a BOM; no utf8NoBOM until PS 6+."
  falsification_test: "If the rewritten Edit/MultiEdit payload, run under PS 5.1, produces a file with NO leading EF BB BF and byte-identical post-replace content, the BOM hypothesis-fix is correct. (Live Win11 Format-Hex check in the human-verify checkpoint.)"
  fix_rationale: "Move Edit/MultiEdit to the same byte vehicle as Write. The hook (Medium IL outer trampoline) pre-computes old/new UTF-8 bytes as PowerShell [byte[]] literals; the inner CLM script reads the target as a byte[] (`Get-Content -Encoding Byte -Raw`, CLM-safe), does a pure-PowerShell literal BYTE-sequence find-and-replace (no .NET), and writes with `Set-Content -Encoding Byte` (BOM-free, byte-faithful). This removes the only BOM source (`-Encoding UTF8` string write) while staying CLM-safe and keeping LITERAL match semantics + MultiEdit one-read/one-write atomicity."
  blind_spots: "Pure-PowerShell byte search/replace correctness in CLM (array slicing/concatenation operators only) is asserted by unit tests over payload SHAPE; the actual runtime byte-replace behavior under PS 5.1 CLM is gated on the live Win11 Format-Hex checkpoint. Empty old_string (Write-style create) and overlapping matches are edge cases covered by the loop semantics (first-match, non-overlapping, left-to-right)."
- **test:** rewrite Edit/MultiEdit payload builders to the byte vehicle; assert via unit tests no `-Encoding UTF8` string write and no .NET calls remain; live Format-Hex gated.
- **expecting:** Edit/MultiEdit payloads emit `[byte[]]` literals for old/new + a CLM byte search/replace loop + `Set-Content -Encoding Byte`; 0 occurrences of `-Encoding UTF8` and 0 .NET method calls.
- **next_action:** Implement byte-vehicle Edit/MultiEdit in claude_code_hook.rs, extend tests, run cargo test + clippy, update Resolution.

## Investigation notes / evidence

- timestamp: 2026-06-13 — Live nested-session self-report: "PowerShell is in Constrained Language Mode, so .NET method calls ([Convert], [System.Text.Encoding]) are blocked ... that's why the hook's base64 approach fails too. The sandbox this shell runs in is an AppContainer."
- timestamp: 2026-06-13 — Hook payloads are pure .NET: `build_confined_write_cmd` (`crates/nono-cli/src/claude_code_hook.rs:440`) → `[Convert]::FromBase64String` + `[System.IO.File]::WriteAllText` + `[System.Text.Encoding]::UTF8.GetString`; `build_confined_edit_cmd` (`:458`) + `build_confined_multiedit_cmd` (`:478`) → `[System.IO.File]::ReadAllText/WriteAllText`.
- timestamp: 2026-06-13 — Arm selection: `select_windows_token_arm()` (`crates/nono-cli/src/exec_strategy_windows/launch.rs:1166-1208`): `has_pty` → `BrokerLaunch` (Low-IL PRIMARY token, no AppContainer); `!is_detached && !has_pty && prefers_low_il_broker && has_session_sid` → `BrokerLaunchNoPty` (AppContainer, per broker no-PTY path / Plan 62-12). A Claude Code Bash tool call is a captured subprocess with NO PTY → `BrokerLaunchNoPty` → AppContainer.
- timestamp: 2026-06-13 — Broker fail-closes if `--no-pty` lacks `--app-container-name` (`crates/nono-shell-broker/src/main.rs:202-205`) and spawns the confined child as a per-run AppContainer carrying `SECURITY_CAPABILITIES{AppContainerSid, CapabilityCount:0}` (`main.rs:418-437,536+`). Capability-less AppContainer → PowerShell CLM (Microsoft `about_Language_Modes`).
- timestamp: 2026-06-01 — `60-HUMAN-UAT.md` UAT 1 PASS: `[System.IO.File]::WriteAllText` landed a 12 B file via "broker arm" — but on the then-PRIMARY-token form of the arm (FullLanguage), per git history (see Eliminated).

## Candidate fixes (for the debugger to evaluate AFTER confirming root cause)

- **(A) CLM-safe vehicle:** rewrite the confined Write/Edit/MultiEdit payloads to avoid .NET method calls — use CLM-allowed cmdlets (`Set-Content`/`Get-Content`) + byte-array literals. CHOSEN.
- **(B) Force the primary-token (FullLanguage) arm for tool-call confined writes** — REJECTED (confinement downgrade; loses WFP package-SID scoping R-N1).
- **(C) Relax CLM for the confined shell** — REJECTED (weakens the jail).

Prefer (A) if a faithful CLM-safe write/edit can be built; else (B) with a careful confinement-equivalence argument. → (A) was buildable.

## Specialist Review

- specialist_hint: `rust`. No dedicated Rust specialist skill in the dispatch map → proceeded directly (no skill invoked). The fix lives entirely inside `#[cfg(target_os = "windows")]` blocks of a Rust CLI crate; self-verification (23/23 unit tests, clippy clean) covers the Rust-idiom surface. Idiomatic/Windows-CLM correctness is gated on the live human-verify checkpoint below.

## Eliminated

- hypothesis: "60-HUMAN-UAT regressed / the same arm both passed and now fails"
  evidence: NOT a regression of behavior on the same arm. Git history shows `BrokerLaunchNoPty` was a Low-IL PRIMARY token (FullLanguage, .NET works) on the 60-HUMAN-UAT date (2026-06-01, commit `7553a927`); Plan 62-12/62-13 (2026-06-02, `cb341165`+`e3a22895`) REDEFINED the arm to spawn an AppContainer (CLM). The Phase-60 .NET payloads were correct for the arm that existed when UAT 1 passed; Phase 62 silently changed the arm's language mode underneath them.
  timestamp: 2026-06-13
- hypothesis: "Direction B (force the primary-token arm for tool-call confined writes)"
  evidence: Rejected as a confinement downgrade. The AppContainer arm provides WFP package-SID network scoping (finding R-N1; broker FAIL-CLOSES if `--no-pty` lacks `--app-container-name`, main.rs:202-205). Falling back to the Low-IL primary-token arm would lose per-session WFP enforcement. Direction A (CLM-safe payloads) keeps the AppContainer arm intact, so it is strictly preferred.
  timestamp: 2026-06-13

## Resolution

- root_cause: A Claude Code Bash tool call is a captured subprocess with NO PTY; with runner profile `windows_low_il_broker:true` it deterministically selects `WindowsTokenArm::BrokerLaunchNoPty` (launch.rs:1186-1193), which (since Plan 62-12/62-13, 2026-06-02) spawns the confined child as a per-run AppContainer carrying `SECURITY_CAPABILITIES{CapabilityCount:0}` (broker main.rs:418-437,536+). PowerShell in a capability-less AppContainer runs in Constrained Language Mode, which blocks all .NET type/method calls. The Phase-60 confined Write/Edit/MultiEdit payloads were built entirely from `[Convert]::FromBase64String`, `[System.IO.File]::WriteAllText/ReadAllText`, and `[System.Text.Encoding]` — all CLM-blocked — so the confined write never executes. This is NOT a recurrence of F-60-UAT-05 (BaseNamedObjects/WRITE_RESTRICTED) and NOT R-A1 (wrapper JSON). The 60-HUMAN-UAT pass (2026-06-01) used the then-current PRIMARY-token form of the same arm (FullLanguage), which is why .NET worked there; Phase 62 changed the arm to AppContainer underneath the Phase-60 payloads.
  CLM driver: the per-run AppContainer execution context (not WDAC/AppLocker, not `__PSLockdownPolicy`). Microsoft `about_Language_Modes` documents app-restricted/allowlist contexts forcing CLM; the live nested-session self-report ("the sandbox this shell runs in is an AppContainer") corroborates.
- fix: Direction A — all three payload builders in `crates/nono-cli/src/claude_code_hook.rs` are CLM-safe (zero .NET method calls). **Write** (unchanged, already correct): `[byte[]]@(...)` UTF-8 byte literal + `Set-Content -LiteralPath ... -Encoding Byte` (byte-faithful: no BOM, no trailing newline). **Edit/MultiEdit (BOM FIX, this cycle):** the prior cycle's `-creplace` + `Set-Content -Encoding UTF8` STRING round-trip was empirically confirmed to prepend a leading UTF-8 BOM (`EF BB BF`) under PS 5.1 (no utf8NoBOM option for string writes), corrupting every confined edit. Replaced with the same byte vehicle as Write: read the target as a byte array (`$src = [byte[]](Get-Content -LiteralPath ... -Encoding Byte -Raw)`), apply a pure-PowerShell literal BYTE-sequence find-and-replace (new helper `byte_replace_statement` — `while`/`-lt`/`-le`/`-ne`/index/`.Length`/`+=`/`@(...)` only; old & new strings carried as `[byte[]]` UTF-8 literals pre-computed by the Medium-IL hook; left-to-right non-overlapping; empty-replacement guarded as deletion), then `Set-Content -LiteralPath ... -Value $src -Encoding Byte`. MultiEdit chains one byte-replace block per edit through `$src` (one read, sequential in-memory replacements, one write → atomic, byte-exact). Removed the now-unused `-creplace` regex helpers (`regex_escape_literal`, `replace_substitution_escape`). Kept the AppContainer arm (preserves WFP package-SID scoping R-N1).
- verification: self-verified — `cargo test -p nono-cli --bin nono claude_code_hook` 23/23 PASS (incl. CLM-safety tests asserting NO .NET constructs and NO `-Encoding UTF8` string write on any path, exact UTF-8 byte literals embedded for old/new, no `-replace`/`-creplace` regex surface, MultiEdit single-read/single-write with one chained byte-replace block per edit, empty-new deletion guard). `cargo clippy -p nono-cli --bin nono -- -D warnings -D clippy::unwrap_used` CLEAN. Cross-target clippy (linux-gnu/apple-darwin) PARTIAL — blocked by missing cross C-toolchain (`x86_64-linux-gnu-gcc` not found; aws-lc-sys/ring need it), deferred to live CI per `.planning/templates/cross-target-verify-checklist.md`. NOTE: ALL changes are inside `#[cfg(target_os = "windows")]` blocks → cannot introduce Unix cfg drift. PENDING live Win11 human-verify (see checkpoint): LanguageMode on the arm + confined Write/Edit/MultiEdit landing byte-faithfully with NO BOM (Format-Hex).
  files_changed: crates/nono-cli/src/claude_code_hook.rs
- known_blind_spot: The pure-PowerShell byte search/replace loop is verified for payload SHAPE by unit tests (no .NET, byte literals embedded, single read/write, deletion guard) — its actual RUNTIME byte-replace behavior under PS 5.1 Constrained Language Mode (array slicing/`+=` concatenation/`.Length` property semantics, large-file `+=` perf) is gated on the live Win11 Format-Hex checkpoint. The earlier BOM blind spot is now CLOSED at the design level: no payload uses `-Encoding UTF8` on a string write anymore (a unit test, `assert_no_utf8_string_encoding`, enforces this on Write/Edit/MultiEdit), so Edit/MultiEdit are byte-faithful by construction exactly like Write.
- status: NOT COMMITTED — awaiting orchestrator decision after live Win11 human-verify checkpoint passes.
