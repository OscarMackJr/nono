---
phase: 63-minifilter-spike-groundwork-macos-divergence-ledger-audit
plan: 03
ledger_type: macos-audit
range: v0.57.0..v0.61.2
upstream_head_at_audit: 61a589f4b2e4022637ef3460e527ad774ed95635
refetch_date: 2026-06-06
drift_tool_sh_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_ps1_sha: 0834aa664fbaf4c5e41af5debece292992211559
drift_tool_invocation: 'bash scripts/check-upstream-drift.sh --from v0.57.0 --to v0.61.2 --format json'
fork_baseline: v0.57.0 (Phase 48 UPST6 sync point — 42 commits across v0.55.0..v0.57.0 absorbed 2026-05-25)
total_unique_commits: 63
date: 2026-06-06
---

# Phase 63 macOS Audit — Upstream v0.57.0..v0.61.2 Divergence Ledger

## Headline

macOS-scoped audit of **63 unique upstream commits** across `v0.57.0..v0.61.2`, grouped into
**19 clusters**. The range extends Phase 54 UPST7 (which covered `v0.57.0..v0.59.0`) through
`v0.60.0` and `v0.61.x` (the UPST8 extension).

Disposition breakdown: **will-sync 12** (C3, C4, C6, C7, C9, C10, C11, C12, C14, C15, C16, C18),
**split 3** (C2, C5, C8), **fork-preserve 0**, **won't-sync 4** (C1 release commits, C13 sigstore
platform gaps, C17 tls_intercept/Keychain — fork lacks that module, C19 libdbus-removal — Linux CI only).
**macos-only: yes** clusters: **C14** (Seatbelt ordering P1 x3), **C15** (--trust-proxy-ca
Keychain P2), **C16** (java_runtime — touches macos.rs), **C17** (Keychain consolidation — depends on C15
tls_intercept/ which fork lacks).

**SUPERSESSION OF PHASE 54 C14 (D-13):** Phase 54 UPST7's cluster C14 dispositioned commits
`8f84d454`, `362ada22`, and `8f1b0b74` as **`won't-sync`** under the rationale "unix/macOS-only
N/A per REQUIREMENTS § Out of Scope" — because Phase 54's milestone was Windows-only (v2.7/v2.8).
**This ledger overrides that verdict to `will-sync` for all three.** The scope change driving the
override is v2.10's explicit macOS-parity objective: the three commits fix Seatbelt last-match-wins
ordering (T-63-10, security) and symlink/`$PWD` CWD capture (T-63-11, correctness), and are the
primary reason Phase 63 exists. Silently copying Phase 54's C14 verdict would drop the exact
security/correctness commits this milestone was opened to absorb (Pitfall F). Phase 64 cherry-picks
these three commits first.

The marquee finding is that all three P1 commits map cleanly to fork-carried files:
`8f84d454` and `c6730e43` touch `crates/nono/src/sandbox/macos.rs`; `362ada22` and `8f1b0b74`
touch `crates/nono-cli/src/sandbox_prepare.rs` — both present in the fork with the same call-site
structure. The diff-inspect (D-11) confirms no cross-cluster re-export prereq blocks a clean
Phase 64 cherry-pick.

## Reproduction

- **Precondition (MANDATORY — Pitfall E):** `git fetch upstream --tags` then assert
  `git cat-file -t 3e605f27` prints `commit`. v0.61.2 was absent in the local store before this
  fetch (`[VERIFIED: git ls-remote / git cat-file]` 2026-06-06); without the fetch, the range
  silently truncates to v0.61.1.
- **upstream_head_at_audit:** `61a589f4b2e4022637ef3460e527ad774ed95635`
- **refetch_date:** 2026-06-06
- **Invocation (verbatim):** `bash scripts/check-upstream-drift.sh --from v0.57.0 --to v0.61.2 --format json`
  (Make fallback: `make check-upstream-drift ARGS="--from v0.57.0 --to v0.61.2 --format json"`)
- **JSON output:** `ci-logs-local/drift/63-macos-v057-v0612.json` (gitignored, NOT committed)
- **drift_tool sha pin:** `0834aa664fbaf4c5e41af5debece292992211559` — this is the git commit SHA
  (`git log --oneline scripts/check-upstream-drift.sh | head -1`) asserted before relying on the
  tool output.
- **total_unique_commits:** 63 (source of truth; merges excluded by the tool)
- **tag split:** 40 commits in `v0.57.0..v0.59.0` (covered by Phase 54), 23 new commits in
  `v0.59.0..v0.61.2` (UPST8 extension — new territory for Phase 63)
- **auditor-rerun recipe:** re-fetch upstream tags (`git fetch upstream --tags`), assert
  `git cat-file -t 3e605f27` prints `commit`, assert the `drift_tool_sh_sha` pin equals
  `git log --format='%H' scripts/check-upstream-drift.sh | head -1`, then re-run the verbatim
  invocation. The same range + HEAD reproduces the 63-commit set. macOS-surface narrowing:
  `git log --no-merges --format='%h %s' v0.57.0..v0.61.2 -- crates/nono/src/sandbox/macos.rs`
  and `-- crates/nono-cli/src/sandbox_prepare.rs`.

## Cluster Summary

| cluster_id | theme | commits | disposition | macos-only | rationale |
|------------|-------|---------|-------------|------------|-----------|
| C1 | Release commits (v0.58–v0.61.2) | 5 | won't-sync | no | Version bumps are fork-managed |
| C2 | Supervisor named-socket IPC (#959) | 9 | split | no | AF_UNIX socket mechanism unix-only; Windows AIPC fork-preserve → Phase 59 |
| C3 | allow_domain path+method | 2 | will-sync | no | Cross-platform fine-grained network filtering → absorbed Phase 56 |
| C4 | proxy 502 hardening | 2 | will-sync | no | Cross-platform proxy robustness → absorbed Phase 55 |
| C5 | TLS-intercept ordering + rcgen bump | 2 | split | no | fork-preserve (RouteStore already satisfies ordering); rcgen in absent tls_intercept/ |
| C6 | Bitwarden bw:// credential source | 2 | will-sync | no | Cross-platform credential source → absorbed Phase 57 |
| C7 | Profile JSONC + target_binary + refactors | 7 | will-sync | no | Cross-platform profile system → absorbed Phase 55 |
| C8 | Session lifecycle hooks (#954) | 2 | split | no | hook_runtime unix-only gated; Windows ADR → Phase 58 |
| C9 | pack-update-hint robustness | 2 | will-sync | no | Cross-platform → absorbed Phase 55 |
| C10 | diagnostic / output / denial polish | 7 | will-sync | no | Cross-platform → Phase 55 + Phase 64 |
| C11 | timeout constants | 3 | will-sync | no | Cross-platform → absorbed Phase 55 |
| C12 | policy test (ENV_LOCK) | 1 | will-sync | no | Test-only cross-platform hardening → absorbed Phase 55 |
| C13 | sigstore dep bump 0.8.0 | 1 | split | no | Cargo bump portable; scrub.rs diff-verify needed → Phase 55 |
| C14 | macOS Seatbelt ordering + symlink CWD (P1 x3) | 3 | **will-sync** | **yes** | **SUPERSEDES Phase 54 C14 won't-sync** — v2.10 macOS-parity scope change; T-63-10/T-63-11 security/correctness → Phase 64 |
| C15 | --trust-proxy-ca macOS Keychain integration (P2) | 1 | will-sync | **yes** | macOS System Keychain trust store integration; P2 priority → Phase 64 |
| C16 | java_runtime group + java-dev profile | 2 | will-sync | **yes** | Touches macos.rs (sysctl-write seatbelt rule); cross-platform policy group → Phase 64 |
| C17 | Keychain CA consolidation + hardening | 5 | won't-sync | **yes** | Depends on C15 tls_intercept/ca.rs (absent from fork) → won't-sync until tls_intercept/ surface resolved |
| C18 | network-policy + keyring + libdbus + deny-by-default | 5 | will-sync | no | Cross-platform security hardening → Phase 64 |
| C19 | UPST8 new profile / diagnostic features | 7 | will-sync | no | Cross-platform profile/diagnostic → Phase 64 |

---

### Cluster C1: Release commits

**Commits:** 5 — chore: release v0.58.0, v0.59.0, v0.60.0, v0.61.0, v0.61.2
**Disposition:** won't-sync
**macOS-only:** no
**Rationale:** Version bumps touch only `crates/nono/Cargo.toml`. Fork manages its own versioning
(MSI version flows from git tag; crate version stays fork-controlled). CHANGELOG entries referenced
but Cargo.toml/Cargo.lock version bumps are dropped. Matches Phase 54 C1 pattern exactly.

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| 54c4deb | chore: release v0.58.0 | v0.58.0 | other | 1 | no |
| fb9be2d8 | chore: release v0.59.0 | v0.59.0 | other | 1 | no |
| 9a05a4ff | chore: release v0.60.0 | v0.60.0 | other | 1 | no |
| 658e40f8 | chore: release v0.61.0 | v0.61.0 | other | 1 | no |
| 3e605f27 | chore: release v0.61.2 | v0.61.2 | other | 1 | no |

---

### Cluster C2: Supervisor named-socket IPC (#959)

**Commits:** 9 (same as Phase 54 C2)
**Disposition:** split
**macOS-only:** no
**Rationale:** AF_UNIX named socket mechanism (`crates/nono/src/supervisor/socket.rs`) is unix-only.
The fork's Windows supervisor uses AIPC/named-pipe (`exec_strategy_windows/`) — fork-preserve on
that path. Cross-platform keep-alive/timeout/blocking-mode intent carried conceptually. The
`UnixSocketCapability` grant (`4a22e94c`) is relevant on macOS but the socket mechanism is unix.
Routed to Phase 59 (REQ-IPC-01) per Phase 54 plan. macOS path: Phase 64 may absorb the
`UnixSocketCapability` grant if the capability type is used for macOS sandbox supervisor.
**Cross-cluster re-export check:** N/A (split, not will-sync).

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| be7681c | fix: replace fd-based IPC with named socket for URL open helpers (#959) | v0.58.0 | other | 6 | no |
| f956fb6 | fix: set accepted listener connections to blocking mode | v0.58.0 | other | 1 | no |
| 9820a2e | fix: include URL listener in supervisor loop keep-alive conditions | v0.58.0 | other | 1 | no |
| 51f56b8 | fix: keep supervisor loop alive when child closes direct IPC socket | v0.58.0 | other | 1 | no |
| d1851c9 | fix: increase supervisor listener read timeout to 5s for URL open | v0.58.0 | other | 3 | no |
| c15c76a | fix: address review comments on supervisor socket IPC | v0.58.0 | other | 3 | no |
| 284ae1d | fix: add read timeout on accepted listener connections | v0.58.0 | other | 1 | no |
| 4a22e94 | fix: grant UnixSocketCapability for supervisor socket in child sandbox | v0.58.0 | other | 1 | no |
| ed47520 | style: format debug message for line length | v0.58.0 | other | 1 | no |

---

### Cluster C3: allow_domain path+method (fine-grained network filtering)

**Commits:** 2 (same as Phase 54 C3 — absorbed in Phase 56)
**Disposition:** will-sync
**macOS-only:** no
**Rationale:** Cross-platform CLI + proxy filtering (REQ-NET-01). Already absorbed in Phase 56.
**Cross-cluster re-export check:** Clean — `0ced085` introduces only `pub(crate) fn merge_allow_domain`
intra-cluster. No cross-cluster re-export dep.

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| 0ced085 | feat(cli): support fine-grained method+path restrictions in allow_domain (#960) | v0.59.0 | other,profile | 12 | no |
| 75b2265 | feat(cli): allow-domain accepts URL with path for endpoint restriction | v0.59.0 | other | 3 | no |

---

### Cluster C4: proxy 502 hardening

**Commits:** 2 (same as Phase 54 C4 — absorbed in Phase 55)
**Disposition:** will-sync
**macOS-only:** no
**Rationale:** Both touch `crates/nono-proxy/src/connect.rs` (cross-platform). Absorbed in Phase 55.
**Cross-cluster re-export check:** Clean — no cross-cluster re-export.

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| d11193f | fix(proxy): return 502 with audit entry on upstream connect failure | v0.58.0 | proxy | 1 | no |
| 4ad708d | fix(proxy): preserve upstream error and sanitise 502 reason line | v0.58.0 | proxy | 1 | no |

---

### Cluster C5: TLS-intercept ordering + rcgen bump

**Commits:** 2 (same as Phase 54 C5)
**Disposition:** split
**macOS-only:** no
**Rationale:** Phase 54's full diff-inspect assessment applies: `22e6c405`'s
`tls_intercept/handle.rs` portion targets a module the fork does not carry (fork-preserve via
`RouteStore`/`CredentialStore` decoupling); the `proxy_runtime.rs` snippet rides with Phase 56
C3 absorb. `8e78dafd` bumps rcgen inside absent `tls_intercept/` — won't-sync. No macOS specificity.
**Cross-cluster re-export check:** N/A (split). Function-call dep C5→C3 still recorded.

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| 22e6c40 | fix(proxy): enforce endpoint rules before credential selection in TLS intercept | v0.59.0 | other,proxy | 2 | no |
| 8e78daf | chore(deps): bump rcgen from 0.13.2 to 0.14.8 | v0.58.0 | proxy | 2 | no |

---

### Cluster C6: Bitwarden bw:// credential source

**Commits:** 2 (same as Phase 54 C6 — absorbed in Phase 57)
**Disposition:** will-sync
**macOS-only:** no
**Rationale:** `crates/nono/src/keystore.rs` cross-platform credential source (REQ-CRED-01).
Absorbed in Phase 57. Zeroize hygiene cross-platform.
**Cross-cluster re-export check:** Clean — no cross-cluster re-export.

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| 47bbd8c | feat: add Bitwarden credential source (bw:// URI scheme) | v0.58.0 | other,profile | 4 | no |
| f28a115 | fix(keystore): use Zeroizing<String> for Bitwarden item fields and in-place truncation | v0.58.0 | other | 1 | no |

---

### Cluster C7: Profile system (JSONC + target_binary + JSONC-centralize + wiring + refactors)

**Commits:** 7 — JSONC profile parsing, target_binary, opencode extraction, centralized JSONC
module, wiring JSONC support, chained-if-let refactor, review fixes.
**Disposition:** will-sync
**macOS-only:** no
**Rationale:** Cross-platform profile-system ports. `53a0c521` + `9398a139` + `2bd9b4d5` +
`cfa24f3d` + `e15aa53c` already absorbed in Phase 55. `ac74efbd` (centralize JSONC parsing) and
`3e61bf20` (wiring JSONC) are UPST8-new straight ports.
**Cross-cluster re-export check:** Clean — `pub(crate) fn resolve_user_profile_path` and
`is_file_path_ref` are intra-cluster definitions; `ac74efbd`'s `crate::jsonc::parse` is a new
intra-cluster public-crate function, no cross-cluster re-export.

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| 9398a13 | feat(profile): allow profiles to specify a target binary | v0.58.0 | other,policy,profile | 4 | no |
| 53a0c52 | feat(profile): add JSONC support for profile files | v0.58.0 | other,profile | 4 | no |
| 2bd9b4d | refactor(profile): extract opencode profile from built-ins | v0.59.0 | other,profile | 5 | no |
| cfa24f3 | refactor: use chained if let for conditional statements | v0.58.0 | other,profile | 2 | no |
| e15aa53 | fix: review fixes | v0.58.0 | other,profile | 3 | no |
| ac74efbd | refactor(jsonc): centralize jsonc parsing | v0.60.0 | other,profile | 4 | no |
| 3e61bf20 | feat(wiring): support jsonc in wiring directives | v0.60.0 | other | 2 | no |

---

### Cluster C8: Session lifecycle hooks (#954)

**Commits:** 2 (same as Phase 54 C8 — absorbed in Phase 58)
**Disposition:** split
**macOS-only:** no
**Rationale:** `1335351f` gates `hook_runtime` unix-only; runtime needs Windows ADR. Schema
cross-platform portion absorbed Phase 58. macOS runtime hook execution works on Phase 58's unix path.
**Cross-cluster re-export check:** N/A (split).

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| daa55c8 | feat: session lifecycle hooks (#954) | v0.58.0 | other,profile | 6 | no |
| 1335351 | refactor(hook_runtime): gate module unix-only, drop dead non-unix branches | v0.58.0 | other | 3 | no |

---

### Cluster C9: pack-update-hint robustness

**Commits:** 2 (same as Phase 54 C9 — absorbed Phase 55)
**Disposition:** will-sync
**macOS-only:** no
**Rationale:** Cross-platform atomic state writes and detached refresh. Absorbed Phase 55.
**Cross-cluster re-export check:** Clean.

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| 74fbbf1 | refactor(pack-hints): refresh in detached process to avoid threads | v0.58.0 | other | 6 | no |
| b1a650a | fix(pack-update-hint): make state file writes atomic | v0.58.0 | other | 1 | no |

---

### Cluster C10: diagnostic / output / denial polish (v0.58–v0.61 combined)

**Commits:** 7 — rfind access-mode split (+test), pre-compute canonical denial paths, bold-only-path
in footer, annotate suppressed denials, interactive denied path selector, denial selector visible
range extraction, limit visible items in denial selector, compact formatting style.
**Disposition:** will-sync
**macOS-only:** no
**Rationale:** Cross-platform diagnostic/output polish. `668e341`, `a606b5b5`, `7cb315c0`, `8fd8da0c`
absorbed Phase 55. `1cfb5363` (interactive selector), `9ec343a3` (visible-range extraction),
`f9271fd2` (limit items), `8f84ed73` (style compact) are UPST8-new and use `crossterm` (cross-platform
terminal UI) — will-sync into Phase 64.
**Cross-cluster re-export check:** Clean — `1cfb5363` introduces only intra-cluster functions in
`profile_save_runtime.rs`; no cross-cluster re-export dep.

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| 7cb315c | fix: annotate suppressed denials and style save prompt paths (#984) | v0.59.0 | other | 3 | no |
| a606b5b | diagnostic: pre-compute canonical denial paths to avoid repeated fs I/O | v0.59.0 | other | 2 | no |
| 668e341 | fix: use rfind for access mode spliting; add test | v0.59.0 | other | 1 | no |
| 8fd8da0 | Bold only path in diagnostic footer, not access type or labels | v0.59.0 | other | 1 | no |
| 1cfb5363 | feat(cli): introduce interactive denied path selector | v0.60.0 | other | 2 | no |
| 9ec343a3 | refactor(denial-selector): extract visible range logic | v0.60.0 | other | 1 | no |
| f9271fd2 | fix(cli): limit visible items in denial selector | v0.60.0 | other | 1 | no |
| 8f84ed73 | style(formatting): make expressions more compact | v0.60.0 | other | 2 | no |

---

### Cluster C11: timeout constants

**Commits:** 3 (same as Phase 54 C11 — absorbed Phase 55)
**Disposition:** will-sync
**macOS-only:** no
**Rationale:** Cross-platform centralized timeouts. Absorbed Phase 55.
**Cross-cluster re-export check:** Clean.

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| 194788e | feat(cli): centralize timeout constants and make user-facing timeouts configurable | v0.59.0 | other | 8 | no |
| 69af73d | fix: tighten up overflow checks | v0.59.0 | other | 2 | no |
| 1442818 | fix: formatting | v0.59.0 | other | 1 | no |

---

### Cluster C12: policy test (ENV_LOCK)

**Commits:** 1 (same as Phase 54 C12 — absorbed Phase 55)
**Disposition:** will-sync
**macOS-only:** no
**Rationale:** Test-only ENV_LOCK hardening. Absorbed Phase 55.
**Cross-cluster re-export check:** Clean — test-only.

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| 1a764d0 | test: lock ENV_LOCK in test_all_groups_no_deny_within_allow_overlap | v0.58.0 | policy | 1 | no |

---

### Cluster C13: sigstore dep bump 0.8.0

**Commits:** 1 (same as Phase 54 C13)
**Disposition:** split
**macOS-only:** no
**Rationale:** Cargo.toml bump (will-sync) + `scrub.rs` change must be diff-verified against the
fork's Phase-49 trust-root surface. Not a blind cherry-pick. Routed to Phase 55 verification.
**Cross-cluster re-export check:** N/A (split).

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| e581569 | chore(deps): update sigstore crates to 0.8.0 | v0.58.0 | other | 2 | no |

---

### Cluster C14: macOS Seatbelt ordering + symlink CWD (P1 x3) — SUPERSEDES Phase 54 C14

**Commits:** 3 — emit platform rules after user write allows (macOS); `$PWD` symlink-CWD capture;
preserve symlink path when adding CWD capability on macOS.
**Disposition:** **will-sync**
**macOS-only:** **yes**
**Rationale:** **OVERRIDES Phase 54 C14 won't-sync.** v2.10 scope is macOS parity; these three
commits are the highest-security items in the range. The Phase 54 verdict ("unix/macOS-only N/A
per REQUIREMENTS § Out of Scope") was correct for the Windows-only v2.7 milestone but is
superseded here. Phase 64 absorbs these as the first cherry-pick batch (T-63-10, T-63-11).

**DIFF-INSPECT NOTE (D-11):**

**`8f84d454` — `fix(macos): emit platform rules after user write allows`**
- **Target site:** `crates/nono/src/sandbox/macos.rs::generate_profile` (lines ~663-700 in the
  fork's current state). The fork's `generate_profile` CURRENTLY places `platform_rules()` BETWEEN
  read-allows and write-allows (the pre-fix ordering). Confirmed by reading the fork's macos.rs:
  the comment says "Platform deny rules are placed BETWEEN read and write rules."
- **Upstream fix:** moves `platform_rules()` loop to AFTER the write-allows loop, so Seatbelt's
  last-match-wins semantics let targeted denies (e.g. `(deny file-write* (subpath "/protected"))`)
  win over any preceding write-allow.
- **Call-site match:** The fork's `generate_profile` call site that emits platform rules
  (`for rule in caps.platform_rules()`) is at the SAME structural location (between
  extension-reads and write-allows) — the fork has NOT applied this fix. Phase 64 cherry-pick
  applies cleanly: diff is purely a block-reordering within `generate_profile`, no new symbols.
- **Cross-cluster re-export check:** Clean — commit is self-contained in macos.rs; no new
  `pub use`/`pub mod`/`pub(crate)` symbols introduced across clusters.

**`362ada22` — `fix(sandbox): use $PWD to capture symlink CWD without --workdir`**
- **Target site:** `crates/nono-cli/src/sandbox_prepare.rs::resolved_workdir`. The fork does NOT
  have a `resolved_workdir` function at all — the fork uses an inline expression (`workdir.clone()
  .or_else(|| current_dir().ok())`) inside `prepare_sandbox`. The upstream commit introduces the
  `resolved_workdir` helper and adds `$PWD` preference logic.
- **Call-site match:** The fork's equivalent CWD capture is `std::env::current_dir()` (always
  returns the canonical real path) — identical to the pre-fix upstream code. Phase 64 cherry-pick
  introduces `resolved_workdir` as a new function extracted from the inline CWD logic. The diff-
  inspect shows this commit depends on `8f1b0b74` (which first extracted `resolved_workdir`
  from the inline expression). Absorption order: `8f1b0b74` first, then `362ada22`.
- **Cross-cluster re-export check:** Clean — `resolved_workdir` is `fn` (private), not re-exported.

**`8f1b0b74` — `fix(sandbox): preserve symlink path when adding CWD capability on macOS`**
- **Target site:** `crates/nono-cli/src/sandbox_prepare.rs` — the `prepare_sandbox` CWD block.
  The fork currently builds `FsCapability::new_dir(cwd_canonical.clone(), access)` (using the
  already-canonicalized path), meaning the symlink path is never in `original`. The upstream fix
  changes this to `FsCapability::new_dir(&workdir, access)` (using the workdir/symlink path), and
  adds a `#[cfg(target_os = "macos")]` block to unconditionally emit the symlink cap when
  `workdir != workdir.canonicalize()`. Also introduces the `resolved_workdir` function.
- **Call-site match:** The fork's CWD capability construction at `FsCapability::new_dir(cwd_canonical
  .clone(), access)` is exactly the pre-fix pattern. The `#[cfg(target_os = "macos")]` block is
  absent from the fork. Phase 64 cherry-pick applies cleanly: the diff targets well-isolated lines
  in `prepare_sandbox` that the fork carries unchanged.
- **add_platform_rule cross-check:** `8f1b0b74` does NOT touch `policy.rs::add_platform_rule` —
  it only touches `sandbox_prepare.rs`. The platform-rule call sites (`policy.rs` lines 448-472,
  692-704) are unaffected by `362ada22`/`8f1b0b74`. The `8f84d454` ordering fix IS the
  consumer of those rules; the symlink commits fix only how the CWD capability is captured.
- **Cross-cluster re-export check:** Clean — `resolved_workdir` is a private crate function.

**Absorption order for Phase 64:** `8f1b0b74` → `362ada22` → `8f84d454` (the first two are
interdependent; `8f1b0b74` extracts `resolved_workdir`, which `362ada22` then modifies).

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| 8f84d454 | fix(macos): emit platform rules after user write allows | v0.58.0 | other | 1 | yes |
| 362ada22 | fix(sandbox): use $PWD to capture symlink CWD without --workdir | v0.58.0 | other | 1 | yes |
| 8f1b0b74 | fix(sandbox): preserve symlink path when adding CWD capability on macOS | v0.58.0 | other | 1 | yes |

Note: `fe233db4` is the merge commit for PR #680 (merges `362ada22` + `8f1b0b74`). The drift tool
correctly excludes merges; this ledger dispositions the two non-merge SHAs above, not the merge.

---

### Cluster C15: --trust-proxy-ca macOS Keychain integration (P2)

**Commits:** 1 — `729697c2`
**Disposition:** will-sync
**macOS-only:** yes
**Rationale:** P2 priority (below C14 P1 x3). Integrates the proxy CA with the macOS system trust
store via Security.framework so Go TLS tools (github-cli, terraform) accept nono's proxy certs.
The feature introduces `crates/nono-cli/src/macos_trust.rs` (new file — no merge conflict possible)
and touches `crates/nono-proxy/src/tls_intercept/ca.rs` (the fork lacks `tls_intercept/` — see
**DIFF-INSPECT NOTE** and C17 below).

**DIFF-INSPECT NOTE (D-11):**
- **Target site:** `crates/nono-cli/src/macos_trust.rs` is a wholly new file; the fork does not
  carry it. Addition is clean.
- **`tls_intercept/ca.rs` (+190 lines):** The fork does NOT carry `crates/nono-proxy/src/
  tls_intercept/`. This is the blocker: `729697c2` modifies `tls_intercept/ca.rs` to expose
  `PreloadedCa` struct and `generate_ecdsa_p256_ca()` used by `macos_trust.rs`. Without
  `tls_intercept/`, the `macos_trust.rs` code cannot compile. Phase 64 must either (a) cherry-pick
  enough of `tls_intercept/` to supply the CA surface, or (b) carve a fork-specific CA abstraction.
  This is the primary complexity risk for C15 absorption.
- **Cross-cluster re-export check:** Clean for pub-use surfaces — `macos_trust.rs` exports only
  `pub(crate) fn load_or_create_trusted_ca` which is intra-CLI. The dependency on `tls_intercept/`
  is a module-existence prereq, not a pub-use cross-cluster dep.
- **Disposition rationale:** `will-sync` despite the tls_intercept/ prereq: the security value
  (system Keychain trust store integration for Go TLS clients) is macOS-specific and non-trivial to
  implement from scratch. Phase 64 resolves the prereq by scoping the cherry-pick.

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| 729697c2 | feat(proxy): add --trust-proxy-ca for macOS system trust store integration | v0.58.0 | other,proxy | 14 | yes |

---

### Cluster C16: java_runtime group + java-dev profile

**Commits:** 2 — add java_runtime group and java-dev profile; address review comments on java_runtime.
**Disposition:** will-sync
**macOS-only:** yes
**Rationale:** `c6730e43` touches `crates/nono/src/sandbox/macos.rs` (2 lines — adds
`sysctl-write kern.grade_cputype` to the Seatbelt profile for JVM CPU-translation-state query
on Apple Silicon) AND `crates/nono-cli/data/policy.json` (java_runtime group + java-dev profile).
`6dd8dc97` moves the sysctl-write rule from macos.rs into `unsafe_macos_seatbelt_rules` on the
java-dev profile (scoping it to java workloads only — principle of least privilege).
The java_runtime group is cross-platform policy (Linux JVM paths included) but the `macos-only`
flag reflects the macos.rs touch.

**DIFF-INSPECT NOTE (D-11):**
- **`c6730e43` target sites:** `sandbox/macos.rs` addition is 2 lines in the global seatbelt
  profile section (an `(allow sysctl-write …)` rule). The fork's `generate_profile` carries that
  section unchanged — clean addition. `policy.json` java_runtime group addition is purely additive
  (new JSON block). No cross-cluster re-export.
- **`6dd8dc97` target sites:** Moves the sysctl-write rule OUT of macos.rs's unconditional profile
  and INTO `unsafe_macos_seatbelt_rules` on the java-dev profile entry in policy.json. Both files
  are fork-carried; the diff is a small deletion in macos.rs + addition in policy.json. Clean apply.
- **Cross-cluster re-export check:** Clean — no new pub-use or pub(crate) symbols; both changes
  are data (policy.json) or a small rule in the profile generator.

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| c6730e43 | feat(policy): add java_runtime group and java-dev profile | v0.60.0 | other,policy | 2 | yes |
| 6dd8dc97 | fix(policy): address review comments on java_runtime group | v0.60.0 | policy | 2 | yes |

---

### Cluster C17: Keychain CA consolidation + hardening (depends on C15 tls_intercept/)

**Commits:** 5 — consolidate Keychain CA storage to single combined PEM entry; detect user-cancelled
trust prompts via OSStatus codes; clean up Keychain on trust failure; align leaf cert expiry with CA;
disambiguate AsRef call on Cow<[u8]> for typed_path compat.
**Disposition:** won't-sync
**macOS-only:** yes
**Rationale:** All five commits depend on `crates/nono-cli/src/macos_trust.rs` (introduced by C15)
AND `crates/nono-proxy/src/tls_intercept/ca.rs` (absent from the fork). `6c472224` refactors both
`macos_trust.rs` and `tls_intercept/ca.rs`; `4e1c7957` refactors all of `tls_intercept/ca.rs`,
`cert_cache.rs`, and `server.rs`; `ad6b0ac8` fixes a compilation issue in `tls_intercept/ca.rs`.
Without the `tls_intercept/` surface (which C15 depends on creating), these commits cannot apply.
Disposition: **won't-sync** until C15 tls_intercept/ prereq is resolved. They are recorded here
for completeness and to prevent accidental re-evaluation as independent items.

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| 6c472224 | refactor(proxy): consolidate Keychain CA storage to single combined PEM entry | v0.60.0 | proxy | 2 | yes |
| 197008ae | fix(proxy): detect user-cancelled trust prompts via OSStatus codes | v0.60.0 | other | 1 | yes |
| 2f4e1a37 | fix(proxy): clean up Keychain on trust failure and expand security docs | v0.60.0 | other,proxy | 2 | yes |
| 4e1c7957 | feat(proxy): align leaf cert expiry with CA and add --proxy-ca-validity flag | v0.60.0 | other,proxy | 10 | yes |
| ad6b0ac8 | fix(proxy): disambiguate AsRef call on Cow<[u8]> for typed_path compat | v0.60.0 | proxy | 1 | yes |

---

### Cluster C18: network-policy security + keyring + deny-by-default (cross-platform UPST8)

**Commits:** 5 — network-policy: do not enable credentials by default in profiles; ensure keyring
features are explicit; feat: remove libdbus dependency on Linux; deny-by-default when network.block
is set; docs credential-injection update.
**Disposition:** will-sync
**macOS-only:** no
**Rationale:** Cross-platform security hardening. `0fb59375` removes implicit credential routes from
embedded profiles (security improvement, cross-platform). `c30a136d` makes keyring features explicit
(cross-platform Cargo.toml hygiene). `c5960164` removes libdbus on Linux (Linux CI/build change;
no macOS impact but fork should sync the `zbus` migration as it affects the keyring backend table
and CI). `bd4c469a` (deny-by-default network.block) adds `strict_filter` to ProxyConfig + threads
the block intent through PreparedSandbox (cross-platform proxy + sandbox_prepare.rs change).
`563535e0` is docs-only (credential-injection.mdx) — will-sync.
**Cross-cluster re-export check:** Clean — `bd4c469a` adds `strict_filter: bool` field to
`ProxyConfig` (intra-cluster) and `acceptable_temp_roots` is in C19 (separate cluster).

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| 0fb59375 | refactor(network-policy): do not enable credentials by default in profiles | v0.61.0 | other | 2 | no |
| c30a136d | ensure keyring features are explicit | v0.60.0 | other | 2 | no |
| c5960164 | feat: remove libdbus dependency on linux | v0.60.0 | other | 17 | no |
| bd4c469a | fix(proxy): deny-by-default when network.block is set (#1082) | v0.61.2 | other,proxy | 8 | no |
| 563535e0 | docs(cli): update credential injection with bitwarden and custom keyring | v0.60.0 | other | 1 | no |

---

### Cluster C19: UPST8 new profile / diagnostic features

**Commits:** 7 — allow registry refs in profile extends; suppress system service diagnostics;
accept cap file under any known temp root; suppress system service diagnostics; deps bumps.
**Disposition:** will-sync
**macOS-only:** no
**Rationale:** Cross-platform profile and diagnostic improvements (UPST8 new territory).
`cc21229f` (suppress system-service diagnostics) touches `sandbox_prepare.rs` (cross-platform)
and adds a profile schema change. `4911d6f1` (cap file under any known temp root) adds
`acceptable_temp_roots()` including `/var/folders` (macOS-specific but returns safely on other
platforms). `20cc5df9` (registry refs in profile extends) is cross-platform. Three dep-bump
commits (`89a1a05c`, `8b54d323`, `cbf652c8`, `35a8cb89`) from v0.61.2 are either Cargo or CI
dep bumps — will-sync where they touch Cargo.lock.

| sha | subject | upstream-tag | categories | files-changed | macos-only |
|-----|---------|--------------|------------|---------------|------------|
| cc21229f | feat(diagnostic): add profile option to suppress system service diagnostics (#1059) | v0.61.0 | other | 11 | no |
| 4911d6f1 | fix(cli): accept cap file under any known temp root for why --self | v0.61.0 | other | 1 | no |
| 20cc5df9 | feat(profile): allow registry refs in profile extends (#1061) | v0.61.0 | other,profile | 2 | no |
| 89a1a05c | chore(deps): bump rustls-native-certs from 0.8.3 to 0.8.4 | v0.61.2 | other | 2 | no |
| 8b54d323 | chore(deps): bump jsonschema from 0.46.4 to 0.46.5 | v0.61.2 | other | 2 | no |
| cbf652c8 | chore(deps): bump actions/checkout from 6.0.2 to 6.0.3 | v0.61.2 | other | 1 | no |
| 35a8cb89 | chore(deps): bump docker/setup-qemu-action from 4.0.0 to 4.1.0 | v0.61.2 | other | 1 | no |

---

## ADR review

Phase 33 ADR (`docs/architecture/upstream-parity-strategy.md`, Option A `continue`, Accepted
2026-05-11, re-confirmed at v2.4 + v2.5 + v2.6 + v2.7 + v2.8 closes) is the subject of this review.
Phase 63 macOS-audit is the **63-commit v0.57.0..v0.61.2 evidence base**, the first macOS-scoped
audit since the scope change from Windows-only to macOS-parity at v2.10.

| dimension | verdict | rationale |
|-----------|---------|-----------|
| security | H | C14 (P1 x3: Seatbelt ordering + symlink CWD) are security/correctness fixes the fork's macOS backend lacks. Absorbing them is the primary Phase 63/64 obligation. C18 (deny-by-default network.block) is a cross-platform security hardening item. High pressure — NOT absorbing means shipping a macOS Seatbelt backend with a known last-match-wins bypass and a symlink CWD EPERM regression. |
| windows | L | No UPST8 clusters are macos-only:no-but-windows-relevant (the Windows-parity work was v2.7–v2.9). The macos-only:yes clusters (C14/C15/C16/C17) have no Windows equivalent surface. Low. |
| maintenance | M | 63 commits (larger than Phase 54's 40), but 12 will-sync clusters are low-labor (Phase 55–58 already absorbed the v0.58/v0.59 portion); only 7 new will-sync clusters remain for Phase 64. The C15/C17 tls_intercept/ prereq is the one structural risk item. Moderate. |
| divergence | M | C14 is the defining example: Phase 54 dispositioned these as won't-sync (divergence preserved); Phase 63 closes the loop (convergent). C17's won't-sync is a deliberate fork-preserve (fork lacks tls_intercept/) not a new divergence. Overall: moving from divergent to convergent on the macOS-critical path. Moderate improvement. |
| contributor | L | The fork absorbs upstream here; Phase 64 cherry-picks feed back via the Phase 40/43 umbrella-PR pattern. Low friction. |

**Outcome:** (a) Confirm. Phase 33 ADR Option A 'continue' remains correct — the macOS-scoped
evidence (three P1 security fixes + cross-platform hardening, manageable tls_intercept/ prereq)
supports continuing the parity strategy with the v2.10 scope extension. The C14 supersession closes
the last known macOS security divergence from the Phase 54 era.

## Empirical cross-check

Spot-checks fork-shared macOS-relevant files against the upstream `v0.57.0..v0.61.2` log to
confirm no drift-tool gaps, per D-11 (diff-inspect re-export, NOT `--name-only`). Closes the
`feedback_cluster_isolation_invalid` lesson empirically (5 files checked, requirement ≥4).

### File: crates/nono/src/sandbox/macos.rs
- Walked upstream log: `git log --no-merges v0.57.0..v0.61.2 -- crates/nono/src/sandbox/macos.rs`
- Commits touching this file in range: **3** (`8f84d454`, `c6730e43`, `6dd8dc97`)
- Cluster mapping: C14 (`8f84d454`), C16 (`c6730e43`, `6dd8dc97`).
- Drift-tool coverage: **PASS** — all 3 present in the 63-commit drift inventory.
- Re-export check: `8f84d454` adds no new symbols (pure reordering). `c6730e43` adds 2 lines of
  inline Seatbelt profile string (no pub-use). `6dd8dc97` REMOVES 2 lines (no new symbols).
  No cross-cluster re-export dep.

### File: crates/nono-cli/src/sandbox_prepare.rs
- Walked upstream log: `git log --no-merges v0.57.0..v0.61.2 -- crates/nono-cli/src/sandbox_prepare.rs`
- Commits touching this file in range: **8** (`bd4c469a`, `cc21229f`, `cfa24f3d`, `0ced085a`,
  `daa55c8e`, `362ada22`, `8f1b0b74` + a 2nd touch in `8f1b0b74`)
- Cluster mapping: C2 (`daa55c8e` touches sandbox_prepare indirectly via hook args), C3 (`0ced085a`),
  C7 (`cfa24f3d`), C8 (`daa55c8e`), C14 (`362ada22`, `8f1b0b74`), C18 (`bd4c469a`), C19 (`cc21229f`).
- Drift-tool coverage: **PASS** — all 8 non-merge commits present in drift inventory.
- Re-export check: `362ada22` and `8f1b0b74` introduce `resolved_workdir` as a private `fn` —
  no cross-cluster pub-use/pub(crate) re-export.

### File: crates/nono-cli/src/policy.rs
- Walked upstream log: `git log --no-merges v0.57.0..v0.61.2 -- crates/nono-cli/src/policy.rs`
- Commits touching this file in range: **2** (`9398a139`, `cc21229f`)
- Cluster mapping: C7 (`9398a139` target_binary also touches policy.rs), C19 (`cc21229f`).
- Drift-tool coverage: **PASS** — both present in drift inventory.
- Re-export check: `cc21229f` adds `suppress_system_services` field — intra-cluster, no cross-cluster
  dep. The `add_platform_rule` call sites (Lines 448-472, 692-704) are untouched by C14 commits
  (confirmed by diff-inspect of `8f84d454`, `362ada22`, `8f1b0b74` — none touch policy.rs).

### File: crates/nono-proxy/src/config.rs
- Walked upstream log: `git log --no-merges v0.57.0..v0.61.2 -- crates/nono-proxy/src/config.rs`
- Commits touching this file in range: **3** (`729697c2`, `2f4e1a37`, `4e1c7957`)
- Cluster mapping: C15 (`729697c2`), C17 (`2f4e1a37`, `4e1c7957`).
- Drift-tool coverage: **PASS** — all 3 present in drift inventory.
- Re-export check: `729697c2` adds `PreloadedCa` usage in config.rs — the struct comes from
  `tls_intercept/ca.rs` (absent from fork). This is the C15 prereq dep, already recorded in
  the C15 diff-inspect note. C17 adds further refinements to the same struct. Consistent with
  C17's won't-sync disposition.

### File: crates/nono-cli/data/policy.json
- Walked upstream log: `git log --no-merges v0.57.0..v0.61.2 -- crates/nono-cli/data/policy.json`
- Commits touching this file in range: **4** (`c6730e43`, `6dd8dc97`, `0fb59375`, `0ced085a`)
- Cluster mapping: C16 (`c6730e43`, `6dd8dc97`), C18 (`0fb59375`), C3 (`0ced085a`).
- Drift-tool coverage: **PASS** — all 4 present in drift inventory.
- Re-export check: policy.json is data (JSON), no pub-use surface. Java_runtime group addition
  (`c6730e43`) and credential-removal (`0fb59375`) are purely additive/subtractive JSON edits;
  no cross-cluster symbol dep.

**Result:** 5 files walked (≥4 required); zero drift-tool gaps; the C15 tls_intercept/ prereq
is the only structural gap between the ledger's will-sync verdicts and a clean Phase 64 cherry-pick.

## Cross-cluster re-export deps detected

Diff-inspect scan (`git show <lead-sha>` for added `pub use` / `pub mod` / `extern crate` /
`pub(crate)`) run on all **12 will-sync** cluster lead commits (C3, C4, C6, C7, C9, C10, C11,
C12, C14, C15, C16, C18, C19).

- **`pub use` / `pub mod` / `extern crate` re-export deps:** **none detected.** New symbols
  surfaced are intra-cluster `pub(crate) fn` definitions (`merge_allow_domain` in C3,
  `resolve_user_profile_path` + `is_file_path_ref` in C7, `resolved_workdir` in C14,
  `load_or_create_trusted_ca` in C15, `denial_selector_visible_range` in C10,
  `acceptable_temp_roots` in C19). None are cross-cluster re-exports.

- **Module-existence prereq (NOT a pub-use dep, but a compile prereq):**
  - **C15 (`729697c2`) → `tls_intercept/` module:** `macos_trust.rs` imports from
    `crates/nono-proxy/src/tls_intercept/ca.rs` (absent from fork). This is a **module-existence**
    prereq, not a pub-use re-export dep (the Phase 43 pattern), but it has the same practical
    consequence: C15 cannot be cherry-picked until the fork carries enough of `tls_intercept/ca.rs`
    to satisfy `PreloadedCa` + `generate_ecdsa_p256_ca`. Phase 64 must scope this.
  - **C17 → C15:** C17 commits further refine `macos_trust.rs` + `tls_intercept/ca.rs`; C17's
    won't-sync disposition correctly captures the dep.

- **Function-call cross-cluster dep (C5 → C3, carried from Phase 54):** `22e6c40`'s
  `proxy_runtime.rs` portion calls `network_policy::partition_allow_domain` (C3) — already
  absorbed Phase 56. No new function-call cross-cluster deps in C14/C15/C16.

- **Absorption ordering consequence:** Phase 64 cherry-pick order for macOS-primary clusters:
  1. C14: `8f1b0b74` → `362ada22` → `8f84d454` (interdep within cluster)
  2. C16: `c6730e43` → `6dd8dc97` (review-fix follows feature)
  3. C15: after resolving tls_intercept/ prereq (may require Phase 64 scoping decision)
  4. C17: deferred (won't-sync until C15 prereq resolved)
