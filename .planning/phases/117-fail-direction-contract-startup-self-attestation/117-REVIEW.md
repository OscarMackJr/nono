---
phase: 117-fail-direction-contract-startup-self-attestation
reviewed: 2026-08-13T16:27:04Z
depth: standard
files_reviewed: 34
files_reviewed_list:
  - .github/workflows/ci.yml
  - bindings/c/src/lib.rs
  - crates/nono-cli/Cargo.toml
  - crates/nono-cli/src/agent_daemon/launch.rs
  - crates/nono-cli/src/cli.rs
  - crates/nono-cli/src/cli_bootstrap.rs
  - crates/nono-cli/src/command_runtime.rs
  - crates/nono-cli/src/exec_strategy_windows/attestation.rs
  - crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs
  - crates/nono-cli/src/exec_strategy_windows/dacl_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/labels_guard.rs
  - crates/nono-cli/src/exec_strategy_windows/launch.rs
  - crates/nono-cli/src/exec_strategy_windows/layer_registry.rs
  - crates/nono-cli/src/exec_strategy_windows/mod.rs
  - crates/nono-cli/src/exec_strategy_windows/network.rs
  - crates/nono-cli/src/exec_strategy_windows/restricted_token.rs
  - crates/nono-cli/src/main.rs
  - crates/nono-cli/src/output.rs
  - crates/nono-cli/src/query_ext.rs
  - crates/nono-cli/src/telemetry/event.rs
  - crates/nono-cli/src/telemetry/mod.rs
  - crates/nono-cli/src/telemetry/windows.rs
  - crates/nono-cli/tests/common/mod.rs
  - crates/nono-cli/tests/layer_force_unavailable.rs
  - crates/nono-cli/tests/layer_registry_meta_test.rs
  - crates/nono-cli/tests/layer_registry_selfcheck.rs
  - crates/nono-shell-broker/Cargo.toml
  - crates/nono-shell-broker/src/main.rs
  - crates/nono/src/diagnostic/codes.rs
  - crates/nono/src/error.rs
  - crates/nono/src/lib.rs
  - crates/nono/src/machine_policy.rs
  - crates/nono/src/sandbox/windows.rs
  - proj/SPEC-windows-fail-direction-contract.md
findings:
  critical: 3
  warning: 14
  info: 0
  total: 17
status: issues_found
---

# Phase 117: Code Review Report

**Reviewed:** 2026-08-13T16:27:04Z
**Depth:** standard
**Files Reviewed:** 34
**Status:** issues_found

## Summary

Phase 117 builds a code-resident Windows layer registry (`layer_registry.rs`), a CLI-side
decision core (`attestation.rs`), three gate insertion sites (direct CLI, daemon, broker), a
D-27 operator channel (banner + HMAC-chained audit event), and a set of drift/vacuity gates.
The runtime decision core itself (`classify_row` / `decide_from_entries`) is, after five
review iterations, sound on the paths I could trace: the `Confirmed` short-circuit, the
`NotApplicable`-before-tighten ordering (WR-29), the tri-state `LayerApplication`, the
fail-secure `#[default] NotApplied`, `probe_in_job`'s null-job refusal, the broker's
fail-closed missing/empty `NONO_BROKER_REQUIRED_LAYERS` handling, `build_windows_environment_block`'s
last-wins dedup (which blocks an inherited `NONO_BROKER_REQUIRED_LAYERS` downgrade), and
`cleanup_network_enforcement_staging`'s three-layer traversal defence all hold up under
adversarial reading.

The defects are concentrated in the *verification machinery* and the *contract-to-code
bindings*, i.e. the parts of this phase whose whole purpose is to prevent green-by-absence:

- The WR-26 "class gate" written to stop unconditional event-log pointers matches **zero**
  strings in the file it names first (`launch.rs`), for two independent reasons, and its
  "production half only" filter is a no-op on that same file.
- The registry is declared the D-01 source of truth, but the **`EntryPath::Daemon` half of it
  is consulted by nothing**; the daemon runs a hand-written mirror that currently contradicts a
  registry row's declared `Abort` outcome, and no test relates the two. This is the exact
  failure mode `broker_expected_rows_are_abort_only` was written to close — re-armed one entry
  path over.
- The `HKLM\...\RequiredLayers` fleet control's parsed value is **discarded by construction**,
  which both leaves the control inert and makes the documented "wire it up next" step
  silently fail open.

Several operator-facing string literals were corrupted by an automated edit (14-22 literal
spaces embedded mid-sentence); this is not merely cosmetic — it is the mechanism that made
CR-01 vacuous.

## Critical Issues

### CR-01: The WR-26 class gate matches zero sites in the file it primarily targets

**File:** `crates/nono-cli/src/output.rs:1543-1606` (with `crates/nono-cli/src/exec_strategy_windows/launch.rs:1496`)

**Issue:** `every_operator_detail_pointer_is_conditional` exists specifically to close the class
("no operator-facing string may name a detail destination unconditionally") rather than the two
enumerated sites — its own doc says "Fixing only the two sites WR-26 named would repeat this
phase's defining failure". It covers **zero** sites in `launch.rs`, for two compounding reasons:

1. **The needle never matches.** `const NEEDLE = "Windows Application event log"`. The one
   production string in `launch.rs` that names that destination (`downgrade_detail_pointer`'s
   `EventLog` arm, `launch.rs:1496`) reads
   `"... in the Windows Application event<14 spaces>log, source \`nono\`, event id 10011 ..."` —
   14 literal spaces sit between `event` and `log`. Verified:
   `grep -c "Windows Application event log" crates/nono-cli/src/exec_strategy_windows/launch.rs`
   → **0**. The `for (label, src) in [("output.rs", ...), ("exec_strategy_windows/launch.rs", ...)]`
   loop therefore `continue`s past every line of `launch.rs`.
2. **The "production half only" filter is a no-op for `launch.rs`.**
   `production()` uses `.take_while(|(_, l)| l.trim() != "mod tests {")`. `launch.rs` has no such
   line — its twelve test modules are named `attestation_gate_tests`, `broker_dispatch_tests`,
   `job_hardening_tests`, etc. (`launch.rs:2717,2748,2882,2996,3102,3265,3353,4548,4647,5255,5518,5773`).
   So the scan runs over the whole 5862-line file, contradicting the doc's stated invariant, and
   any future test-assertion string containing the needle would fail the build spuriously.

Net effect: moving `launch.rs:1496`'s text out of the `DowngradeDetailChannel::EventLog` arm — the
exact regression this gate names — would not fail any test. The gate can relabel; it can never deny.

**Fix:**
```rust
// output.rs — normalise whitespace before matching, and use a structural
// end-of-production marker that exists in every scanned file.
fn production(src: &str) -> Vec<(usize, String)> {
    src.lines()
        .enumerate()
        .take_while(|(_, l)| !l.trim_start().starts_with("mod ")
                          || !l.trim_end().ends_with("_tests {")
                             && l.trim() != "mod tests {")
        // ^ better: scan the whole file and instead skip lines whose enclosing
        //   item is #[cfg(test)]-gated, or split production/test by an explicit
        //   `// ---- TESTS BELOW ----` sentinel asserted to exist in each file.
        .filter(|(_, l)| { let t = l.trim_start(); !t.starts_with("///") && !t.starts_with("//") })
        .map(|(i, l)| (i, l.split_whitespace().collect::<Vec<_>>().join(" ")))
        .collect()
}
```
and add a non-vacuity assertion so the gate cannot silently cover nothing:
```rust
let hits: usize = /* count of NEEDLE matches across all scanned files */;
assert!(hits >= 2, "WR-26 class gate matched {hits} site(s) — it must cover at least the \
    output.rs banner arm and launch.rs's downgrade_detail_pointer EventLog arm; a zero/low \
    match count means the needle drifted out of the production text (see CR-01)");
```
Also repair `launch.rs:1454,1496,1502,1623` (see WR-01) so the needle matches real text.

---

### CR-02: The `EntryPath::Daemon` half of the registry drives no decision, and the daemon contradicts a row's declared `Abort` outcome

**Files:**
- `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:718-749,957-966` (`DACL_PACKAGE_SID_SCOPED_EXPECTANCY`, `DaclAncestorTraverse` row: `(Daemon, None, expected: true)`, `outcome: ContractOutcome::Abort`)
- `crates/nono-cli/src/agent_daemon/launch.rs:1464-1474` (`daemon_attest_and_decide`)
- `crates/nono-cli/src/exec_strategy_windows/attestation.rs:166-176`

**Issue:** Two coupled defects.

1. **`attest_and_decide` is never called with `EntryPath::Daemon` in production.** Grepping the
   whole crate, `EntryPath::Daemon` appears only in doc comments and in `attestation.rs`'s own
   `#[cfg(test)]` module (`:1482`, `:1503`). `nono-agentd` does not `#[path]`-include
   `exec_strategy_windows/` at all (`main.rs:37-39` gates the whole tree on
   `target_os = "windows"` and only `nono`'s binary includes it), so the daemon *cannot* read the
   registry. Every `(Daemon, None)` cell in the registry — five of them, all `outcome: Abort` —
   is decoration. `daemon_attest_and_decide` is a hand-written, hardcoded four-layer mirror
   (`AppContainerProfile`, `JobObjectContainment`, `WfpEgressFilters`, `DaclPackageSidGrant`).

2. **The mirror already contradicts the registry.** `DaclAncestorTraverse` declares
   `outcome: ContractOutcome::Abort` at `(Daemon, None)`, and D-01 / the SPEC state plainly that
   "the registry wins". `daemon_attest_and_decide` instead emits a `tracing::warn!` and proceeds
   (`agent_daemon/launch.rs:1464-1474`). The rationale (WR-06: under-granting never
   under-confines) may well be right, but the *registry* — the declared source of truth — says
   `Abort`, nothing reconciles the two, and the discrepancy is not recorded in the SPEC's
   standing "Contract vs. code discrepancies" table that D-15 exists to hold it.

3. **There is no daemon analog of `broker_expected_rows_are_abort_only`.** That test
   (`layer_registry.rs:1312-1341`) exists because a broker-expected row silently excluded from a
   filter "would go completely unattested on the broker arm — the exact green-by-absence failure
   mode this phase exists to close, re-armed for the next added row." A 14th `LayerId` declared
   `expected: true` at `(Daemon, None)` with `outcome: Abort` compiles, satisfies
   `all_entries_covers_every_layer_id`, `every_registry_row_has_a_test` (via a
   `MANUALLY_VERIFIED`/`ALSO_AUTOMATED` entry), and `spec_matches_registry` (name-only), and goes
   completely unattested on `nono agent launch`.

**Fix:** Add a discovery-based drift gate that binds the daemon mirror to the registry, mirroring
the broker invariant's shape (source-text based, since the daemon binary cannot link the registry):

```rust
// crates/nono-cli/tests/layer_registry_meta_test.rs
/// Every registry row expected at (EntryPath::Daemon, expected: true) with an
/// attestable probe must be named by daemon_attest_and_decide's source text.
/// Names no LayerId — a future (Daemon, ..) row fails this without editing the test.
#[test]
fn daemon_expected_rows_are_all_named_by_the_daemon_gate() {
    let registry = read_layer_registry();
    let daemon_gate = std::fs::read_to_string(
        workspace_root().join("crates/nono-cli/src/agent_daemon/launch.rs")).unwrap();
    for row in daemon_expected_abort_rows(&registry) {   // parse from source, like ALL
        assert!(daemon_gate.contains(&format!("layer: \"{row}\"")),
            "{row} is expected at (Daemon, None) with outcome Abort, but \
             daemon_attest_and_decide never returns Abort naming it — the row goes \
             completely unattested on the daemon arm (CR-02)");
    }
}
```

and reconcile `DaclAncestorTraverse` by one of:
- changing that row's `(Daemon, None)` cell to `expected: false` (the code's real behaviour), **or**
- giving the row a distinct daemon outcome and recording the divergence in the SPEC's
  "Contract vs. code discrepancies" table per D-15.

---

### CR-03: `HKLM\SOFTWARE\Policies\nono\RequiredLayers` is parsed and then unconditionally discarded

**File:** `crates/nono/src/machine_policy.rs:684-708,729`

**Issue:**
```rust
fn warn_if_required_layers_configured(key: &RegKey) -> RequiredLayersPolicy {
    match read_list_subkey(key, "RequiredLayers") { /* ... eprintln! only ... */ }
    RequiredLayersPolicy::default()          // <- the parsed names are dropped
}
...
let required_layers = warn_if_required_layers_configured(key);
```
`MachineEgressPolicy.required_layers.required` is therefore **structurally always empty**,
regardless of what an administrator configured. Two consequences:

1. A documented fleet-wide security control ("must confirm active before it will report the
   session as enforcing") enforces nothing. CLAUDE.md's fail-secure rule is explicit that
   "Configuration load failures must be fatal. If security lists fail to load, abort" — here the
   list loads successfully and is thrown away.
2. It is a **fail-open landmine for the very next change**. The doc comment
   (`machine_policy.rs:186-191`) tells the next implementer that all that remains is "where the
   already-read machine policy is carried to the Windows launch path". Wiring
   `machine_required_layers: policy.required_layers.required` at
   `launch.rs:1563` will compile, pass every test, look wired, and still pass an empty slice —
   because the discard is three files away and the only thing marking it is a comment. The whole
   D-26 tighten path (`decide_from_entries`'s `tightened` branches, including the WR-29
   `NotApplicable`-abort fix at `attestation.rs:507-530`) stays unreachable in a shipped build.

The `eprintln!` warning helps an operator who reads stderr, but it does not stop the control from
being silently ineffective, and it does nothing about (2).

**Fix:** Populate the field truthfully and make the *consumer* side the loud gap, not the parser:
```rust
fn read_required_layers(key: &RegKey) -> RequiredLayersPolicy {
    match read_list_subkey(key, "RequiredLayers") {
        Ok(names) if names.is_empty() => RequiredLayersPolicy::default(),
        Ok(names) => {
            eprintln!("[nono] RequiredLayersNotEnforced: ... {} layer(s) ({}) ...",
                      names.len(), names.join(", "));
            RequiredLayersPolicy { required: names }   // carry the real value
        }
        Err(e) => { eprintln!("[nono] RequiredLayersNotEnforced: ... ({e}) ..."); 
                    RequiredLayersPolicy::default() }
    }
}
```
and add a test that fails the build the day the field is read but not plumbed:
```rust
#[test]
fn required_layers_round_trip_is_not_silently_dropped() {
    // parse a fixture key carrying RequiredLayers=["WfpEgressFilters"]
    assert_eq!(policy.required_layers.required, vec!["WfpEgressFilters".to_string()],
        "the parser must not discard a configured fleet control (CR-03)");
}
```
If the operator decision to enforce is genuinely still open, keep the value populated and gate at
the single consumer, so the gap is one grep away instead of hidden in a parser return.

## Warnings

### WR-01: Operator-facing message literals contain 14-22-space runs from a bad automated edit

**Files:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:1454,1466,1496,1502,1623`;
`crates/nono-cli/src/output.rs:1587,1600`;
`crates/nono-cli/src/exec_strategy_windows/attestation.rs:1419`;
`crates/nono-cli/src/exec_strategy_windows/attestation_downgrade_event.rs:287,325`;
`crates/nono-cli/src/exec_strategy_windows/launch.rs:3910,3954`

**Issue:** Verified with `cat -A`: these are single-line string literals containing literal runs
of 14-22 spaces mid-sentence (e.g. `"...unavailable                  (AUD-04: ..."`). They render
verbatim to the operator on the D-27 downgrade path, and they are the direct cause of CR-01's
vacuity. The pattern looks like multi-line literals whose newline + indentation were collapsed to
spaces without adding `\` continuations.

**Fix:** Restore proper `\`-continued literals, e.g.
```rust
DowngradeDetailChannel::EventLog => " (layer detail is in the Windows Application event \
     log, source `nono`, event id 10011; re-run with --log-file <path> to capture it \
     locally)"
    .to_string(),
```
and add a `cargo fmt`-independent lint/test asserting no production string literal in these
files contains `"  "` (two consecutive spaces) outside an intentional indentation context.

---

### WR-02: `bindings/c` silently collapses `LayerAttestationFailed` to `NonoDiagnosticCode::Other`

**Files:** `bindings/c/src/types.rs:195-241`, `crates/nono/src/diagnostic/codes.rs:38-43`

**Issue:** `crates/nono`'s new `NonoDiagnosticCode::LayerAttestationFailed` carries the explicit
rationale "Distinct from `Self::ConfigurationError` so operators can tell an attestation failure
from generic misconfiguration (D-22)". The FFI enum has no corresponding variant, and its `From`
impl ends with `_ => Self::Other` (`types.rs:238`). Every C/Python/TypeScript consumer therefore
sees `Other = 99` for exactly the case the new code exists to distinguish. Because
`nono::NonoDiagnosticCode` is `#[non_exhaustive]`, the wildcard is required by Rust and no
compile error will ever surface this or any future addition.

**Fix:**
```rust
// bindings/c/src/types.rs
pub enum NonoDiagnosticCode {
    /* ... */
    LayerAttestationFailed = 15,
    Other = 99,
}
// and in From:
nono::NonoDiagnosticCode::LayerAttestationFailed => Self::LayerAttestationFailed,
```
Add a round-trip test asserting `NonoDiagnosticCode::from(nono::NonoDiagnosticCode::LayerAttestationFailed) != Other`,
and regenerate `nono.h` (the `ffi-header` CI job at `.github/workflows/ci.yml:427` will catch the
header drift).

---

### WR-03: `ClearStaleLayerResidue`'s library doc still prescribes the remedy `main.rs` proved wrong

**File:** `crates/nono/src/diagnostic/codes.rs:76-77`

**Issue:** The doc reads "the exact remedy is layer-specific (e.g. `icacls <path>
/setintegritylevel Medium` for `MandatoryIntegrityLabel`)". `main.rs:259-276` records (CR-06,
verified on-host) that this command **writes a Medium mandatory-label ACE and re-triggers the same
abort**, and `main.rs:363-367` has an explicit regression assertion forbidding it in the rendered
output. Two mirrored sites now carry contradictory guidance for one condition — the classification
-consistency failure mode this phase's own memory notes call out.

**Fix:** Replace the example in `codes.rs` with the removal mechanism `main.rs` actually
prescribes (`SetNamedSecurityInfoW(.., LABEL_SECURITY_INFORMATION, .., <empty ACL>)`), or drop the
example entirely and point at `NonoError::remediation`'s renderer as the single source.

---

### WR-04: The "non-silenceable" D-27 banner is silenceable by a pre-planted, enumerable dedup marker

**File:** `crates/nono-cli/src/output.rs:105-113,199-232`

**Issue:** `print_attestation_downgrade_banner` returns early on `path.exists()` for a zero-byte
marker under `state_paths::sessions_dir()/<session_id>/attestation-downgrade/<key_hex>`. The key is
`format!("{:016x}", DefaultHasher::new().hash(dedup_key).finish())` — `DefaultHasher::new()` is
documented as using fixed keys, so the mapping is deterministic and offline-computable, and
`dedup_key` is a sorted comma-join drawn from a 13-element enum (at most 2^13 possible values). Any
process running as the user that can write into the sessions directory can pre-create every marker
and permanently suppress the banner for that session — a channel the function's own doc calls
unconditional ("Deliberately takes no `silent` parameter ... must reach the operator
unconditionally (Warning-8)").

Secondary: a 64-bit collision between two *different* downgraded-layer-sets in the same session
silently suppresses the second, genuinely different one. And `DefaultHasher`'s output is not
guaranteed stable across Rust releases, so a toolchain bump silently re-prints every banner once.

**Fix:** Do not treat marker presence as authoritative silence. Either (a) hold the announced set
in process memory for the lifetime of the supervisor (the hook path re-enters `spawn_windows_child`
in the *same* process only for a long-running session — for cross-process dedup, write the marker
with `OpenOptions::new().create_new(true)` and treat a pre-existing file whose content does not
match a supervisor-written HMAC as "not announced"), or (b) store the full `dedup_key` in the file
and compare contents rather than trusting the hashed filename:
```rust
if let Ok(existing) = std::fs::read_to_string(path) {
    if existing == dedup_key { return; }   // genuine repeat
}
// otherwise: collision, tamper, or stale — print.
```

---

### WR-05: `every_call_site_string_names_a_line_number` no longer checks what its name asserts

**File:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:1387-1398`

**Issue:** All `call_sites` entries are now `"file.rs::Symbol"` form (NR3-08). The assertion is
`site.contains(".rs:")`, which `.rs::Symbol` satisfies, so the test named "names a line number"
verifies only that a `.rs:` substring exists. It is additionally vacuous for the two rows with
`call_sites: &[]` (`DaclSessionSidGrant`, `MinifilterAbsence`). The real content verification lives
in `layer_registry_selfcheck.rs::registry_call_sites_exist`; this in-crate test now provides only
false reassurance.

**Fix:** Either delete it (the integration test is strictly stronger), or make it assert the
current convention and be non-vacuous:
```rust
#[test]
fn every_call_site_string_is_symbol_form() {
    let mut checked = 0usize;
    for entry in all_entries() {
        for site in entry.call_sites {
            assert!(site.contains(".rs::"), "{:?}'s call_sites entry {site:?} is not \
                \"file.rs::Symbol\" form", entry.id);
            checked += 1;
        }
    }
    assert!(checked >= 20, "only {checked} call sites checked — the loop went vacuous");
}
```

---

### WR-06: The WR-27 abort-path guard's predicate is narrower than the class it names

**Files:** `crates/nono-cli/src/output.rs:1597-1605`, `crates/nono-cli/src/main.rs:417-422`

**Issue:** Both guards assert `!line.contains("see the Windows Application event log")`, but the
class they name is "points the operator at the Windows Application event log" — a different, wider
condition. `"check the Windows Application event log"`, `"in the Windows Application event log"`,
or any other verb evades both. The narrow needle exists to tolerate `main.rs:305`'s legitimate
*negated* mention ("No Windows Application event log record is written on this path"), but the fix
for that is to exclude the negation, not to narrow the predicate to one verb.

**Fix:**
```rust
const EVENT_LOG: &str = "Windows Application event log";
let normalised = line.split_whitespace().collect::<Vec<_>>().join(" ");
if let Some(pos) = normalised.find(EVENT_LOG) {
    let preceding = &normalised[..pos];
    assert!(preceding.ends_with("No ") || preceding.ends_with("no "),
        "WR-27: main.rs:{} mentions the Windows Application event log other than as an \
         explicit negation — no record is written there on the abort path", idx + 1);
}
```

---

### WR-07: `layer_force_unavailable.rs` claims three automated rows; two exist

**File:** `crates/nono-cli/tests/layer_force_unavailable.rs:36-44`

**Issue:** The module doc says "this file automates the 3 that are BOTH (a)... (b)... (c)..." and
"The remaining 10 rows are on `layer_registry_meta_test.rs`'s `MANUALLY_VERIFIED` list". Only two
`#[test]` functions exist (`force_unavailable_mandatory_integrity_label`,
`force_unavailable_dacl_package_sid_grant`) after WR-05 removed the duplicate. And the actual split
is 2 automated + 8 `ALSO_AUTOMATED` + 3 `MANUALLY_VERIFIED` = 13, not 3 + 10. A reader auditing
coverage from this doc will look for a test that does not exist.

**Fix:** Update the counts to `2` / `8 ALSO_AUTOMATED` / `3 MANUALLY_VERIFIED`, and consider
asserting the arithmetic mechanically in `layer_registry_meta_test.rs`
(`automated + also_automated + manual == ALL.len()`), so the doc cannot drift again.

---

### WR-08: `AttestationInput::wfp_preconfirmed` is a WFP-specific fact consumed by a probe-kind-generic branch

**File:** `crates/nono-cli/src/exec_strategy_windows/attestation.rs:426-452`

**Issue:** `classify_row`'s `ProbeKind::ConfirmedByEnforcingComponentReport` arm reads
`input.wfp_preconfirmed` for **any** row carrying that probe kind, keyed on nothing but the probe
kind. Today only `WfpEgressFilters` uses it, so the coupling is invisible. A second row added with
`ConfirmedByEnforcingComponentReport` (the type's doc invites one: "a report *from* the enforcing
component itself") would be attested by the WFP service's filter count — confirmed or denied by
evidence about a different mechanism entirely. That is the same wrong-object defect CR-01/RF-01
removed from `MandatoryIntegrityLabel`, latent one row away.

**Fix:** Key the evidence by `LayerId` the way `AppliedLayers::status()` already does:
```rust
pub enforcement_reports: layer_registry::EnforcementReports, // per-LayerId Option<bool>
...
layer_registry::ProbeKind::ConfirmedByEnforcingComponentReport => {
    match input.enforcement_reports.confirmed(entry.id) { /* exhaustive over LayerId */ }
}
```
At minimum, add a `#[cfg(test)]` discovery assertion that exactly one registry row declares
`ConfirmedByEnforcingComponentReport`, with a message naming this coupling.

---

### WR-09: No CI job runs clippy on Windows, so `clippy::unwrap_used` is unenforced on this phase's code

**File:** `.github/workflows/ci.yml:454-477` (`clippy` job), `:311-356` (`windows-layer-fault-injection`)

**Issue:** The `clippy` job's matrix is `[ubuntu-latest, macos-latest]`. Every file this phase
touched under `exec_strategy_windows/`, plus `agent_daemon/launch.rs::windows_impl`,
`telemetry/windows.rs`, the Windows halves of `cli_bootstrap.rs`/`output.rs`, and
`nono-shell-broker`, is `#[cfg(target_os = "windows")]`-gated and is therefore never linted. The
new `windows-layer-fault-injection` job runs only `cargo test`. `RUSTFLAGS: -Dwarnings` catches
rustc warnings on the Windows test jobs but **not** clippy-only lints, and CLAUDE.md declares
`clippy::unwrap_used` a hard project rule. (I found no production `.unwrap()`/`.expect()` in the
reviewed Windows files today — every hit is in a `#[cfg(test)]` module — so this is a missing gate,
not a live violation.)

**Fix:** Add a Windows leg to the clippy job, or a step to `windows-layer-fault-injection`:
```yaml
      - name: Clippy (Windows, incl. fault-injection cfg)
        run: |
          cargo clippy --workspace --all-targets -- -D warnings -D clippy::unwrap_used
          cargo clippy -p nono-sandbox-cli --all-targets --features layer-fault-injection -- -D warnings -D clippy::unwrap_used
```

---

### WR-10: `LayerId::MandatoryIntegrityLabel` conflates two different kernel objects across entry paths

**Files:** `crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:212-214,872-909`,
`crates/nono-shell-broker/src/main.rs:434-450`, `proj/SPEC-windows-fail-direction-contract.md:53`

**Issue:** The row's own doc defines it as "NO_WRITE_UP (± NO_READ_UP/NO_EXECUTE_UP)
mandatory-label ACE on every compiled filesystem-policy path" — a *file* ACE — and declares
`probe: ProbeKind::ConfiguredOnly`. On the `(Broker, None)` cell, the thing actually attested is
the confined grandchild's **token integrity RID** (`rid <= SECURITY_MANDATORY_LOW_RID`), a
different kernel object that says nothing about whether any file carries the contracted ACE. RF-01
removed exactly this substitution from the CLI core on the grounds that it "would not attest what
this row claims", then reinstated it on the broker arm under the same `LayerId`. The
`required_layers_for_broker` wire contract and `BROKER_ATTESTABLE_LAYERS` therefore agree on a
name while disagreeing on the claim, and a broker-arm `MandatoryIntegrityLabel: Confirmed` does not
mean what the registry row says it means.

**Fix:** Split the concepts — e.g. add `LayerId::ChildTokenIntegrityLevel` with its own
`(Broker, None)` expectancy and `LiveTokenOrJobQuery` probe, drop the `(Broker, None)` cell from
`MandatoryIntegrityLabel`, and update `BROKER_ATTESTABLE_LAYERS` plus the SPEC row. If splitting is
deferred, record the conflation explicitly in the SPEC's "Contract vs. code discrepancies" table
(D-15) rather than in a parenthetical inside the registry table's Probe column.

---

### WR-11: `session_id_is_safe_path_component`'s doc overstates what the allow-list excludes

**File:** `crates/nono-cli/src/output.rs:234-249`

**Issue:** The doc claims the allow-list "structurally excludes separators, `..`, drive prefixes,
ADS colons, reserved-device names with extensions, trailing dots/spaces, and every non-ASCII
homoglyph trick". Bare Windows reserved device names — `CON`, `PRN`, `AUX`, `NUL`, `COM1`..`COM9`,
`LPT1`..`LPT9` — are pure ASCII alphanumerics and pass. Impact today is benign (the subsequent
`create_dir_all` fails, the marker is not written, and the banner re-prints — the fail-safe
direction), but the doc asserts a guarantee the predicate does not provide, which is how the next
consumer of this helper gets bitten.

**Fix:** Either add the exclusion or correct the claim:
```rust
const RESERVED: &[&str] = &["CON","PRN","AUX","NUL","COM1",/* ... */,"LPT9"];
!session_id.is_empty()
    && session_id.len() <= 128
    && session_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    && !RESERVED.iter().any(|r| session_id.eq_ignore_ascii_case(r))
```

---

### WR-12: The entire D-26 tighten path is unreachable in a shipped build

**Files:** `crates/nono-cli/src/exec_strategy_windows/launch.rs:1555-1563`,
`crates/nono-cli/src/exec_strategy_windows/attestation.rs:481-620`

**Issue:** The only production call site hardcodes `required_layers_override: &[]` and
`machine_required_layers: &[]`, and CR-03 guarantees the machine-policy half is empty anyway. So
`decide_from_entries`'s `tightened` variable is a compile-time-constant `false` on every shipped
launch: the `NotApplicable`-abort fix (WR-29, `:523-528`), the `DegradeWithVisibleClaim`/
`FailOpenDefect` tighten aborts (`:565-569`), the `FailOpen` tighten abort (`:577-581`), the
`SubstituteEquivalentMechanism` tighten fall-through (`:600`) and `attest_and_decide`'s
unrecognized-name rejection (`:643-658`) are all dead. This is the same "the signal never fires"
shape RF-16 was convened to fix one level up, and it means WR-29's fix is pinned only by unit tests
against synthetic rows.

**Fix:** Ship CR-03's plumbing (or a `--required-layers` CLI flag) so at least one production path
can populate a non-empty tighten set, and add a discovery test asserting the gate call site does
not pass a literal empty slice — mirroring `daemon_attestation_gate_is_wired_to_real_outcomes_not_the_gate_condition`'s
source-text approach:
```rust
#[test]
fn attestation_gate_tighten_inputs_are_not_hardcoded_empty() {
    let src = include_str!("exec_strategy_windows/launch.rs");
    assert!(!src.contains("required_layers_override: &[],"),
        "the only production gate passes an empty tighten set — every `tightened` branch in \
         decide_from_entries is dead code (WR-12)");
}
```

---

### WR-13: `LayerApplication::NotApplied` is structurally unreachable for `WfpEgressFilters`

**Files:** `crates/nono-cli/src/exec_strategy_windows/mod.rs:478-483`,
`crates/nono-cli/src/exec_strategy_windows/layer_registry.rs:465-495`

**Issue:** `wfp_composition_report` returns `Some(true)` for `WfpServiceManaged` and `None`
otherwise — it can never return `Some(false)`. `AppliedLayers::status(WfpEgressFilters)` therefore
never yields `LayerApplication::NotApplied`, and `from_tristate`'s `Some(false)` arm is dead for
that row. The field's own doc claims a genuine tri-state ("`Some(false)` when it was selected and
did not install"). The row's deny direction rests entirely on `derive_wfp_preconfirmed`, so there
is no live fail-open today — but the type advertises a negative it cannot express, which is the
class of defect NR-04/NR-05 were opened to eliminate.

**Fix:** Either make the composition report honest —
```rust
fn wfp_composition_report(guard: Option<&NetworkEnforcementGuard>) -> Option<bool> {
    match guard {
        Some(NetworkEnforcementGuard::WfpServiceManaged { installed_filter_count, .. }) =>
            Some(*installed_filter_count > 0),
        _ => None,
    }
}
```
(which also removes the single-source-of-evidence concern NR-04 flagged, at the cost of collapsing
composition and evidence again) — or narrow the field's type to `Option<()>`/a two-state enum and
correct the doc so the unreachable state is not advertised.

---

### WR-14: `ClearStaleLayerResidue` guidance fires for `LayerAttestationFailed` causes that are not residue

**Files:** `crates/nono/src/error.rs:505-509`, `crates/nono-cli/src/main.rs:285-313`

**Issue:** `NonoError::remediation()` returns `ClearStaleLayerResidue { layer }` for *every*
`LayerAttestationFailed`, but that variant is also constructed for causes with nothing stale to
clear:
- `attest_and_decide`'s unrecognized required-layer name (`attestation.rs:650-655`), where `layer`
  is an admin's typo, not a `LayerId`;
- each `layer-fault-injection` seam (`restricted_token.rs:86`, `labels_guard.rs:267`,
  `dacl_guard.rs:194,387,625`);
- `probe_in_job`'s null-job refusal (`crates/nono/src/attestation.rs:281`).

The operator is told to hunt for a stale ACE / re-run with `-vv --log-file` and search for a layer
name that, in the typo case, appears nowhere.

**Fix:** Distinguish the causes at construction — e.g. add a `NonoRemediation::CheckRequiredLayersPolicy`
for the unrecognized-name path and return `None` for the fault-injection seams — or carry a
`kind: LayerAttestationFailureKind` field on the error variant and branch `remediation()` on it.
The `_` arm in `render_error_for_operator` should not be the place that decides.

---

_Reviewed: 2026-08-13T16:27:04Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
