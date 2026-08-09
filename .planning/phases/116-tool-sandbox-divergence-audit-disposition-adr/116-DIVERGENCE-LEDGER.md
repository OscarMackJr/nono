---
phase: 116-tool-sandbox-divergence-audit-disposition-adr
plan: 01
ledger_type: tool-sandbox-audit
range: "0551eba27ea53b0eda20f4f796f757dd168adb92..0055bf3c686de6fa665decd9831dc5686b282cb0"
upstream_head_at_audit: 6bd8a393fd25f7b4e331b0b064242f90db77a068
refetch_date: 2026-08-09
fork_baseline: 6509bcc904794d878a6c323cb75eb87a686c707f
total_unique_commits: 38
date: 2026-08-09
---

## Headline

**This ledger covers the tool-sandbox subsystem's full upstream history, `v0.64.1..v0.71.0`
(D-01) — not Phase 108's fenced `v0.66.0..v0.69.0`.** The *git range* (`0551eba2..0055bf3c`)
contains **187** non-merge commits total; the subsystem's *dispositioned surface* inside that
range — the set this ledger actually accounts for, per the 3-path union
(`crates/nono-cli/src/tool-sandbox/`, `crates/nono-cli/src/command_policy.rs`,
`crates/nono-cli/src/lineage_cgroup.rs`) — is **38** commits. These are two different numbers by
design (Pitfall 4, `116-RESEARCH.md`): the remaining 149 non-tool-sandbox commits in the full
window belong to Phase 108's already-closed audit (for the `v0.66.0..v0.69.0` portion) or to a
future UPST13/FUT-08 sync (for the `v0.69.0..v0.71.0` portion) — neither is this phase's job.
`total_unique_commits` in this file's frontmatter is set to **38** (the dispositioned surface),
not 187 (the full range), for exactly this reason — stated explicitly so a reader does not have to
infer which of the two numbers the field means.

The 38-commit surface splits into three windows:

| window | range | commits | source |
|--------|-------|--------:|--------|
| pre-fence | `v0.64.1..v0.66.0` | **7** | new in this ledger (Task 1/2, below) |
| fenced | `v0.66.0..v0.69.0` | **20** (9 pure / 11 split) | already dispositioned by `108-DIVERGENCE-LEDGER.md`; cross-referenced here, not duplicated (D-13) |
| post-fence | `v0.69.0..v0.71.0` | **11** | new in this ledger (Task 1/2, below) |
| **Total** | `v0.64.1..v0.71.0` | **38** | 7 + 20 + 11 = 38 — matches the live 3-path-union measurement exactly (see Reproduction) |

**D-03 module-set re-derivation finding (Task 1):** the re-derived set is **identical to D-06's
inherited three** (`tool_sandbox`, `command_policy`, `lineage_cgroup`). All four D-03-named
candidate modules (`command_blocking_deprecation.rs`, `instruction_deny.rs`,
`open_url_runtime.rs`, `hook_runtime.rs`) are explicit **OUT** calls — zero static cross-reference
in either direction and (per the Pitfall-3 control-flow read) no runtime coupling to the
tool-sandbox entrypoint either. See "D-03: Module-Set Re-Derivation" below for the full grep
evidence and the broader 82-module main.rs sweep this task ran to avoid confirming only the four
pre-named guesses (D-16's "discover, don't confirm" rule).

**This plan's scope (Plans 116-01, Task 1+2 only):** Reproduction block, D-03 module-set
re-derivation, and full pre-fence/post-fence per-commit disposition tables with `windows-touch`
flags. **Full D-04 residue accounting for `split`-flagged commits, the carve-out re-touch check
(D-19), and the completeness sweep are out of scope for this plan** — deferred to Plans
116-03/116-04 per `116-01-PLAN.md`'s own objective statement. Disposition values below (adopt /
adapt / skip / split) are the D-02 classification only; they do not yet carry per-path
absorb/defer/noise buckets for the `split` rows (that is Plan 116-03's job).

---

## Reproduction

```bash
# The `upstream` remote already points at nolabs-ai/nono (repointed in v3.3 Phase 94/confirmed
# Phase 108). No remote relocation work this cycle.
git remote -v
# upstream  https://github.com/nolabs-ai/nono.git (fetch/push)

git fetch upstream --tags
git ls-remote --tags upstream v0.64.1 v0.71.0
```

**Live output (2026-08-09):**

```
0551eba27ea53b0eda20f4f796f757dd168adb92  refs/tags/v0.64.1
0055bf3c686de6fa665decd9831dc5686b282cb0  refs/tags/v0.71.0
```

Both SHAs **match `116-CONTEXT.md` D-01's pinned values verbatim, and match the
`116-RESEARCH.md` §2a live-verified values from the same day exactly.** No escalation needed —
upstream has not moved either tag since research time.

```bash
# SHA reachability guard (all 4 pinned SHAs written into this ledger)
for sha in 0551eba27ea53b0eda20f4f796f757dd168adb92 \
           0055bf3c686de6fa665decd9831dc5686b282cb0 \
           d817ed53663c6bba4669ee7a5bfb41b35971fd1b \
           59bdace7e905c05c127f480dc6d2a8c3a3331392; do
  git cat-file -t "${sha}"
done
# -> commit (x4, all confirmed 2026-08-09)

# upstream/main boundary — a NAMED boundary, not an audited SHA (D-01)
git rev-parse upstream/main
# -> 6bd8a393fd25f7b4e331b0b064242f90db77a068 (2026-08-09T16:20:46Z)
```

**`upstream/main` has moved since the discussion-time research session**: it was `4ede9ccc`
(2026-08-05, per `116-CONTEXT.md`/`116-RESEARCH.md`) and is now `6bd8a393` (2026-08-09, this
session's live re-fetch). This is expected drift on a live, actively-released remote and does
**not** affect the pinned-SHA window (`v0.64.1..v0.71.0`) this ledger audits — the window tip is
the `v0.71.0` **tag**, not the moving branch head (D-01). Recorded here as the new named boundary
for whoever runs the next sync; a `v0.72.0` tag now also exists upstream (observed during the
`git fetch --tags` above) — also out of this window, also not audited here.

```bash
RANGE="0551eba27ea53b0eda20f4f796f757dd168adb92..0055bf3c686de6fa665decd9831dc5686b282cb0"

git log --no-merges --oneline $RANGE | wc -l
# -> 187  (full window, ALL subsystems -- NOT the tool-sandbox-scoped figure; see Headline)

git log --no-merges --oneline $RANGE -- \
  'crates/nono-cli/src/tool-sandbox/' \
  'crates/nono-cli/src/command_policy.rs' \
  'crates/nono-cli/src/lineage_cgroup.rs' | wc -l
# -> 38
```

**Reconciliation against D-01's hypothesis table:** `116-CONTEXT.md` D-01 records the 3-path-union
surface as **38** (measured 2026-08-09 during discussion). This session's live re-run of the exact
same command against the same pinned SHAs also returns **38**. **No delta.** The superseded/prior
figure (38) and this session's re-measured figure (38) are identical — recorded per D-17's "in
place, including the case where they agree" rule, not silently assumed.

### Pathspec Hazard Reproduction (Pitfall 1)

`116-RESEARCH.md` §2d already reproduced Phase 108's D-06 substring-glob hazard once, live, on
2026-08-09 during research. This task re-runs it independently (not copied from the research doc)
against the same pinned SHAs, as the reusable evidence every later `git log -- <pattern>`
invocation in this ledger must avoid repeating:

```bash
# THE HAZARD FORM -- never use this as an authoritative count:
git log --no-merges --oneline $RANGE -- '*tool-sandbox*' | wc -l
# -> 37

# THE CORRECT DIRECTORY-ONLY FORM (trailing slash, single-quoted):
git log --no-merges --oneline $RANGE -- 'crates/nono-cli/src/tool-sandbox/' | wc -l
# -> 35

# Isolate the over-counted SHA(s):
comm -23 <(git log --no-merges --format='%H' $RANGE -- '*tool-sandbox*' | sort) \
         <(git log --no-merges --format='%H' $RANGE -- 'crates/nono-cli/src/tool-sandbox/' | sort)
# -> 5a7447d3ed30835bd9bd647b7812ee18ea80a155
# -> ebd51cbb9546aec872136301249d048074a05e38
```

```bash
git show --name-only --format='' 5a7447d3ed30835bd9bd647b7812ee18ea80a155 | grep -i tool-sandbox
# -> docs/cli/features/tool-sandbox.mdx
git show --name-only --format='' ebd51cbb9546aec872136301249d048074a05e38 | grep -i tool-sandbox
# -> docs/cli/features/tool-sandbox.mdx
```

**Confirmed:** both over-counted commits' *only* substring-matching path is
`docs/cli/features/tool-sandbox.mdx` — the same two SHAs Phase 108's own D-06 finding already
named for the `v0.66.0..v0.69.0` window. The hazard reproduces identically in this larger window
because these two commits happen to also fall inside `v0.64.1..v0.71.0`. **Directory-only form
(35) is NOT the authoritative tool-sandbox count for this window** — the 3-path union (38) is,
and is used throughout this ledger.

**Union arithmetic breakdown** (confirms 38 = 35 + 3):

```bash
git log --no-merges --oneline $RANGE -- 'crates/nono-cli/src/command_policy.rs' | wc -l
# -> 17
git log --no-merges --oneline $RANGE -- 'crates/nono-cli/src/lineage_cgroup.rs' | wc -l
# -> 1

comm -23 <(git log --no-merges --format='%H' $RANGE -- \
             'crates/nono-cli/src/command_policy.rs' 'crates/nono-cli/src/lineage_cgroup.rs' \
             | sort -u) \
         <(git log --no-merges --format='%H' $RANGE -- 'crates/nono-cli/src/tool-sandbox/' \
             | sort -u)
# -> 5a7447d3ed30835bd9bd647b7812ee18ea80a155
# -> c808f000db582ad69a10c1a2a1b8221b020fff94
# -> ebd51cbb9546aec872136301249d048074a05e38
```

35 (directory-only) + 3 (unique to `command_policy.rs`/`lineage_cgroup.rs`, listed above) = **38**.
`5a7447d3`/`ebd51cbb` appear in both this list and the substring-hazard list above (they are
genuinely `command_policy.rs`-touching, not docs-only in the union sense); `c808f000` is the third
member, unique to this breakdown (see its full disposition in the Pre-Fence table below).

---

## D-03: Module-Set Re-Derivation

**Method (per `116-CONTEXT.md` D-03 and the Pitfall-3 correction `116-RESEARCH.md` §2h already
found):** follow `mod`/`use` edges out of `tool-sandbox/mod.rs` AND `main.rs` (the wiring for the
subsystem's `mod` declarations lives at the crate root, not inside the subsystem's own files —
this is itself a re-confirmed structural finding, not assumed from the research doc), then grep
both directions (does each sibling module reference the tool-sandbox set; does the tool-sandbox
set reference each sibling module) and read `main.rs`'s function body for runtime coupling that a
static `mod`/`use` grep cannot see.

### Step 1 — `tool-sandbox/mod.rs`'s own declared children

```bash
git show v0.71.0:crates/nono-cli/src/tool-sandbox/mod.rs | \
  grep -n "^\s*\(pub(crate)\s*\)\?mod \|^\s*pub mod "
```

Result: `audit_context`, `credentials`, `dynamic_providers` (`pub(crate)`), `env`, `launch`,
`policy`, `protocol`, `token_broker` (`pub(crate)`), `url_shim`, plus `platform::{linux, macos}`
(cfg-gated). All 9 are already inside the D-06 inherited module set (they are
`tool-sandbox/*.rs`'s own files) — no new candidates surface from this step.

### Step 2 — `main.rs`'s full top-level `mod`/`use` block (the FULL list, not four pre-named guesses)

```bash
git show v0.71.0:crates/nono-cli/src/main.rs | grep -n "^mod \|^use \|#\[path"
```

Returns **82 sibling `mod` declarations** (excluding `mod test_env;` and the `#[cfg(test)] mod
tests` block, both test-only) plus the `#[path = "tool-sandbox/mod.rs"] mod tool_sandbox;`
re-alias. Full list (alphabetical): `app_runtime`, `approval_runtime`, `audit_attestation`,
`audit_client`, `audit_commands`, `audit_event_reader`, `audit_integrity`, `audit_ledger`,
`audit_session`, `capability_ext`, `cli`, `cli_bootstrap`, `command_blocking_deprecation`,
`command_display`, `command_policy`, `command_runtime`, `completions`, `config`,
`credential_runtime`, `deprecated_policy`, `deprecated_schema`, `deprecation_warnings`,
`diagnostic`, `exec_strategy`, `execution_runtime`, `hook_runtime`, `instruction_deny`, `jsonc`,
`launch_runtime`, `legacy_cleanup`, `lineage_cgroup`, `macos_trust`, `migration`,
`network_policy`, `open_url_runtime`, `output`, `pack_update_hint`, `package`, `package_cmd`,
`package_status`, `platform`, `platform_client`, `policy`, `profile`, `profile_cmd`,
`profile_runtime`, `profile_save_runtime`, `protected_paths`, `proxy_command`, `proxy_runtime`,
`pty_proxy`, `pull_ui`, `query_ext`, `registry_client`, `resource_cgroup`, `rollback_commands`,
`rollback_preflight`, `rollback_runtime`, `rollback_session`, `rollback_ui`, `sandbox_log`,
`sandbox_prepare`, `sandbox_state`, `session`, `session_commands`, `setup`, `startup_prompt`,
`startup_runtime`, `state_paths`, `supervised_runtime`, `terminal_approval`, `theme`, `timeouts`,
`tool_sandbox`, `trust_cmd`, `trust_intercept`, `trust_keystore`, `trust_scan`, `update_check`,
`url_open`, `why_runtime`, `wiring`.

Of these, 3 are the D-06 inherited module-set members themselves (`tool_sandbox`,
`command_policy`, `lineage_cgroup`); the remaining **79** are candidates the derivation must call
in-or-out on. The 4 D-03-named candidates (`command_blocking_deprecation`, `instruction_deny`,
`open_url_runtime`, `hook_runtime`) are among these 79.

### Step 2b — Bulk forward grep: which of the 79 siblings reference the module set

```bash
git grep -n -e 'tool_sandbox' -e 'command_policy' -e 'lineage_cgroup' v0.71.0 \
  -- 'crates/nono-cli/src/*.rs' \
  | grep -v -e ':crates/nono-cli/src/command_policy.rs:' -e ':crates/nono-cli/src/lineage_cgroup.rs:' \
  | sed 's/^v0.71.0://' | cut -d: -f1 | sort -u
```

Result (29 files, mapped to their top-level `main.rs` module name where the file is the module's
own file): `approval_runtime`, `audit_commands`, `audit_event_reader`, `audit_integrity`,
`capability_ext`, `cli`, `command_runtime`, `exec_strategy` (+ its own submodule
`exec_strategy/supervisor_linux.rs`), `execution_runtime`, `launch_runtime`, `main` (the crate
root itself — expected, it declares the `mod`), `policy`, `profile` (`profile/mod.rs`),
`profile_cmd`, `profile_runtime`, `profile_save_runtime`, `proxy_runtime`, `sandbox_prepare`,
`supervised_runtime`, `why_runtime` — **19 non-module top-level siblings with a forward
cross-reference**, plus the 9 tool-sandbox-set files and `command_policy.rs`/`mod.rs` themselves
(expected self-matches, excluded from the count above via the `grep -v`).

**None of the 4 D-03-named candidates appear in this forward list.**

### Step 2c — Bulk reverse grep: which siblings are referenced BY the module set

```bash
git grep -hoE 'crate::[a-z_]+' v0.71.0 \
  -- 'crates/nono-cli/src/tool-sandbox/*.rs' 'crates/nono-cli/src/tool-sandbox/platform/*.rs' \
     'crates/nono-cli/src/command_policy.rs' 'crates/nono-cli/src/lineage_cgroup.rs' \
  | sed 's/^crate:://' | sort -u
```

Result: `approval_runtime`, `audit_integrity`, `cli_bootstrap`, `command_policy` (self),
`exec_strategy`, `lineage_cgroup` (self), `policy`, `profile`, `pty_proxy`, `resource_cgroup`,
`state_paths`, `terminal_approval`, `test_env` (test-only), `tool_sandbox` (self), `url_open` —
**11 non-module top-level siblings with a reverse cross-reference** (`approval_runtime`,
`audit_integrity`, `cli_bootstrap`, `exec_strategy`, `policy`, `profile`, `pty_proxy`,
`resource_cgroup`, `state_paths`, `terminal_approval`, `url_open`).

**None of the 4 D-03-named candidates appear in this reverse list either.**

### Step 2d — Explicit per-candidate evidence (the 4 D-03-named modules, minimum required)

```bash
git grep -n -e 'command_blocking_deprecation' -e 'instruction_deny' -e 'open_url_runtime' \
             -e 'hook_runtime' v0.71.0 \
  -- 'crates/nono-cli/src/tool-sandbox/*.rs' 'crates/nono-cli/src/tool-sandbox/platform/*.rs' \
     'crates/nono-cli/src/command_policy.rs' 'crates/nono-cli/src/lineage_cgroup.rs'
echo "exit=$?"
# -> (no output); exit=1  (zero hits, confirmed both directions -- Step 2b/2c already showed the
#    forward direction too)
```

Commit-history count for the 4 candidates within the full window:

```bash
git log --no-merges --oneline $RANGE -- \
  crates/nono-cli/src/command_blocking_deprecation.rs crates/nono-cli/src/instruction_deny.rs \
  crates/nono-cli/src/open_url_runtime.rs crates/nono-cli/src/hook_runtime.rs | wc -l
# -> 1
```

Per-file breakdown: `command_blocking_deprecation.rs` — 0 commits in window;
`instruction_deny.rs` — 0; `open_url_runtime.rs` — **1**
(`4cc0af2c52119c3794843416b292c2ed88d22c8c`, `fix(tests): use /tmp for socket test dirs to stay
under SUN_LEN limit (#1303)`); `hook_runtime.rs` — 0. **This single commit does NOT touch the
3-path-union set** (it is not one of the 38 dispositioned commits — its own touched paths are
`crates/nono-cli/src/open_url_runtime.rs`, `crates/nono-cli/tests/url_open_integration.rs`,
`crates/nono/src/supervisor/socket.rs`, none of which are `tool-sandbox/`, `command_policy.rs`, or
`lineage_cgroup.rs`). **Conclusion: the in-or-out call is moot for this ledger's 38-commit
dispositioned surface** — even if `open_url_runtime.rs` were called IN, it would add zero commits
to this ledger's accounting, since its one touching commit already falls outside the union. (This
commit is itself pre-existing project knowledge: Phase 108's proposed-Phase-112 RES-02 residual
item already names `4cc0af2c52` as test-infra residual — unrelated to tool-sandbox, cross-checked
here for completeness, not re-litigated.)

**LOC re-verification** (live, this session — not inherited from research):
`command_blocking_deprecation.rs` = 192 LOC, `instruction_deny.rs` = 159 LOC,
`open_url_runtime.rs` = 210 LOC, `hook_runtime.rs` = 601 LOC (all via
`git show v0.71.0:<path> | wc -l`, matches `116-RESEARCH.md` §4 exactly — re-confirmed, not
copied forward per D-16).

### Step 3 (Pitfall 3) — `main.rs` control-flow read for the 4 candidates

Static `mod`/`use` absence is necessary but not sufficient evidence of "no relationship" — a
sibling module's free function can be called without any `use` edge, given the `mod` declaration
already present at the crate root. `main.rs`'s actual `fn main()` body (not just its header) was
read:

```bash
git show v0.71.0:crates/nono-cli/src/main.rs | sed -n '95,155p'
```

`fn main()` calls, in order: `tool_sandbox::maybe_run_internal_tool_sandbox_entrypoint()` (early
return if true) -> `tool_sandbox::record_main_start()` -> CLI arg collection ->
`command_blocking_deprecation::collect_cli_warnings()` /
`command_blocking_deprecation::print_warnings()` -> `run_cli(cli)` (the actual command dispatch,
opaque to this static read) -> `tool_sandbox::log_main_total()`.

**Finding: `command_blocking_deprecation`'s two functions run in the *same* `fn main()` body as
the tool-sandbox entrypoint calls, sequentially — but neither gates nor is gated by the other.**
They are independent statements in the same function, not a control-flow dependency. This is
exactly the shape Pitfall 3 warns "a static grep alone would find nothing, but reading the body
finds co-location" — co-location is confirmed, causal coupling is not.

For the other 3 candidates, their call sites are NOT in `main.rs`'s `fn main()` at all — a
targeted grep for `<module>::` usage across the crate (excluding each candidate's own file) finds:

```bash
git grep -n "instruction_deny::" v0.71.0 -- 'crates/nono-cli/src/*.rs' | grep -v instruction_deny.rs:
# -> crates/nono-cli/src/launch_runtime.rs:510:    instruction_deny::write_protect_verified_files(...)
git grep -n "open_url_runtime::" v0.71.0 -- 'crates/nono-cli/src/*.rs' | grep -v open_url_runtime.rs:
# -> crates/nono-cli/src/app_runtime.rs:6:use crate::open_url_runtime::run_open_url_helper;
git grep -n "hook_runtime::" v0.71.0 -- 'crates/nono-cli/src/*.rs' | grep -v hook_runtime.rs:
# -> crates/nono-cli/src/execution_runtime.rs:466,699 (execute_before_hook / execute_after_hook)
```

**Finding: `instruction_deny`, `open_url_runtime`, and `hook_runtime` are each called from deep
inside the command-execution pipeline (`launch_runtime.rs`, `app_runtime.rs`,
`execution_runtime.rs`) — not from `fn main()`'s early-dispatch block at all, and not adjacent to
or gated by any tool-sandbox entrypoint call.** No runtime coupling found for these 3, beyond
being registered as sibling top-level modules in the same `main.rs` (the same conclusion the
static grep already reached).

### Final in-or-out table

| module | declared in `main.rs`? | fwd cross-ref to module set? | rev cross-ref from module set? | in-or-out | reasoning |
|--------|:---:|:---:|:---:|:---:|-----------|
| `command_blocking_deprecation` | yes | no (Step 2b) | no (Step 2c) | **OUT** | Zero static cross-ref either direction; runs in same `fn main()` body as tool-sandbox calls but not gated by/gating them (Step 3); 0 commits touch it in-window |
| `instruction_deny` | yes | no (Step 2b) | no (Step 2c) | **OUT** | Zero static cross-ref either direction; called from `launch_runtime.rs`, not from the tool-sandbox entrypoint path (Step 3); 0 commits touch it in-window |
| `open_url_runtime` | yes | no (Step 2b) | no (Step 2c) | **OUT** | Zero static cross-ref either direction; called from `app_runtime.rs`, not from the tool-sandbox entrypoint path (Step 3); 1 commit touches it in-window (`4cc0af2c`) but that commit does not touch the 3-path union — moot for this ledger's 38-commit surface |
| `hook_runtime` | yes | no (Step 2b) | no (Step 2c) | **OUT** | Zero static cross-ref either direction; called from `execution_runtime.rs`, not from the tool-sandbox entrypoint path (Step 3); 0 commits touch it in-window |
| `approval_runtime`, `audit_commands`, `audit_event_reader`, `audit_integrity`, `capability_ext`, `cli`, `cli_bootstrap`, `command_runtime`, `exec_strategy`, `execution_runtime`, `launch_runtime`, `policy`, `profile`, `profile_cmd`, `profile_runtime`, `profile_save_runtime`, `proxy_runtime`, `pty_proxy`, `resource_cgroup`, `sandbox_prepare`, `state_paths`, `supervised_runtime`, `terminal_approval`, `url_open`, `why_runtime` (25 modules) | yes | 19 of 25 (Step 2b) | 11 of 25 (Step 2c; overlaps the 19) | **OUT** (of the module set — remain CONSUMERS) | Each has a static cross-ref in one or both directions, but on the same pattern Phase 108's residue accounting already established for these exact files (`execution_runtime.rs`, `launch_runtime.rs`, `main.rs`, `profile_runtime.rs`, `sandbox_prepare.rs`, `proxy_runtime.rs`, `capability_ext.rs`, `policy.rs` all appear as "wiring-only" non-module call-sites in `108-DIVERGENCE-LEDGER.md`'s tool-sandbox-split residue tables) — general CLI runtime/utility files the module set calls into or is called from, not structural members of the subsystem itself |
| remaining 54 modules (`app_runtime`, `audit_attestation`, `audit_client`, `audit_ledger`, `audit_session`, `command_display`, `completions`, `config`, `credential_runtime`, `deprecated_policy`, `deprecated_schema`, `deprecation_warnings`, `diagnostic`, `jsonc`, `legacy_cleanup`, `macos_trust`, `migration`, `network_policy`, `output`, `pack_update_hint`, `package`, `package_cmd`, `package_status`, `platform`, `platform_client`, `protected_paths`, `proxy_command`, `pull_ui`, `query_ext`, `registry_client`, `rollback_commands`, `rollback_preflight`, `rollback_runtime`, `rollback_session`, `rollback_ui`, `sandbox_log`, `sandbox_state`, `session`, `session_commands`, `setup`, `startup_prompt`, `startup_runtime`, `theme`, `timeouts`, `trust_cmd`, `trust_intercept`, `trust_keystore`, `trust_scan`, `update_check`, `wiring`) | yes | no | no | **OUT** | Zero grep hits in either direction (Step 2b/2c bulk sweep) — no relationship of any kind to the tool-sandbox module set found |

**Final call: the re-derived module set is identical to D-06's inherited three
(`tool_sandbox`, `command_policy`, `lineage_cgroup`).** No expansion, no reduction. All 4
D-03-named candidates and all other 75 `main.rs` siblings are explicit **OUT** calls, each with
cited grep evidence on the record — not implied by omission. **This confirms, rather than
contradicts, D-06's original 3-module set** — the re-derivation was run from first principles
(full 82-module sweep, both directions, plus a control-flow read) and independently landed on the
same boundary, which is itself the finding this task exists to produce (a verified confirmation
carries the same evidentiary weight as a correction, per D-16).

---

## Disposition Vocabulary

adopt/adapt/skip/split describe what each commit's content would require IF a future absorb executes — a descriptive classification of the commit itself, not a decision about whether to execute an absorb. adopt = would land on the re-derived module set verbatim if ever absorbed. adapt = would require modification to fit fork conventions. skip = purely upstream-process content with no fork-relevant substance. split = touches the module set AND at least one other production path outside it (residue accounting handled in a later plan). This classification takes no position on Pole A vs. Pole B.

**Classification rule applied below (structural, per the definition above):** a commit is `split`
if `git show --name-only --format='' <sha>` lists any path under `crates/*/src/` or
`bindings/c/src/` outside the 3-path module set, **regardless of whether that path's diff content
is itself trivial** (e.g. a one-line string rename) — the structural rule is what determines
`split` vs. non-split; content triviality is recorded as a note, not used to reclassify a
structurally-split commit as `skip`. This mirrors `108-DIVERGENCE-LEDGER.md`'s own methodology
(e.g. `a519ee62`, `72a98830` were `split` there despite carrying zero absorb-worthy residue on
inspection).

---

## Pre-Fence Commits (v0.64.1..v0.66.0)

7 commits (live-measured; matches D-01's hypothesis of 7 — no delta).

```bash
git log --no-merges --oneline 0551eba27ea53b0eda20f4f796f757dd168adb92..d817ed53663c6bba4669ee7a5bfb41b35971fd1b -- \
  'crates/nono-cli/src/tool-sandbox/' 'crates/nono-cli/src/command_policy.rs' \
  'crates/nono-cli/src/lineage_cgroup.rs' | wc -l
# -> 7
```

| sha | subject | PR# | touched-path summary | windows-touch | disposition |
|-----|---------|:---:|-----------------------|:---:|--------------|
| `691e0f4fa0880ff5392a2f54cd913b1d0e0094ae` | feat(tool-sandbox): simplify self-invocation policy | #1268 | `crates/nono-cli/data/nono-profile.schema.json` (data), `command_policy.rs`, `tool-sandbox/platform/{linux,macos}.rs`, `docs/cli/features/tool-sandbox.mdx` | no | adopt (all module-set + non-production; no other prod path) |
| `7011bc8554d9b34117969f239a7ca9883a195184` | feat(tool-sandbox): add @git:common-dir dynamic token | #1271 | `CHANGELOG.md`, `tool-sandbox/dynamic_providers.rs`, `docs/cli/features/tool-sandbox.mdx` | no | adopt (module-set + non-production only) |
| `c808f000db582ad69a10c1a2a1b8221b020fff94` | chore: migrate GitHub org references from always-further to nolabs-ai | #1235 | 29 paths: CI workflows, `Cargo.toml`, `README.md`, `SECURITY.md`, `command_policy.rs` (module set) + `migration.rs`, `profile/mod.rs`, `proxy_runtime.rs`, `setup.rs`, `test_env.rs`, `update_check.rs`, `crates/nono-proxy/src/route.rs` (production, outside module set) + docs/packaging/scripts noise | no | **split** — structurally split per the rule above; content is a trivial org-URL string rename (verified: `command_policy.rs`'s only hunk changes a test-fixture URL path string), zero behavioral substance |
| `d2252225c6b03bf1bedd17a8b5ec798584e6bada` | fix(tool-sandbox): skip missing fs_read/fs_write dirs instead of erroring | #1253 | `CHANGELOG.md`, `tool-sandbox/platform/{linux,macos}.rs` | no | adopt (module-set + non-production only) |
| `853d52366bc278fb3a6de4632f15003a867b8265` | fix(tool-sandbox): pass TLS trust bundle env vars to tool-sandbox children | #1249 | `CHANGELOG.md`, `tool-sandbox/env.rs`, `tool-sandbox/platform/macos.rs` | no | adopt (module-set + non-production only) |
| `137bb15c569d85341e7dcc9cd38ef0c969e5777f` | fix(sandbox): use syscall() for execveat to avoid glibc 2.34 linker dependency | #1239 | `tool-sandbox/platform/linux.rs` only | no | adopt (single module-set file, Linux-specific driver internals) |
| `11fd10e0c88e747b6d751fec9b38f207276b747c` | feat(sandbox): tool sandbox | **#1105** | 90 paths — the subsystem's own introduction commit. Module set (all `tool-sandbox/*.rs` + `command_policy.rs`) + ~35 `nono-cli/src/*.rs` production files (`approval_runtime.rs`, `audit_commands.rs`, `cli.rs`, `exec_strategy.rs`, `execution_runtime.rs`, `launch_runtime.rs`, `main.rs`, `policy.rs`, `profile/mod.rs`, `sandbox_prepare.rs`, `supervised_runtime.rs`, `wiring.rs`, ...) + **`crates/nono/src/{audit.rs, keystore.rs, lib.rs, sandbox/linux.rs, sandbox/mod.rs, scrub.rs, supervisor/mod.rs, supervisor/types.rs}`** (core library) + `crates/nono-proxy/src/*` (7 files) + tests/data/docs | no | **split** — the largest and highest-risk split commit in this window; the only pre-fence commit touching the core library (`crates/nono/src/...`), directly relevant to the ADR-86 policy-free-library-boundary criterion (D-09) |

**Note on `#1105`'s core-library touches:** `crates/nono/src/{audit.rs, keystore.rs, lib.rs,
sandbox/linux.rs, sandbox/mod.rs, scrub.rs, supervisor/mod.rs, supervisor/types.rs,
undo/types.rs}` are touched by this commit — this is the introduction PR for the entire subsystem
and the one pre-fence commit whose non-module residue reaches into the core library tier, not
just `nono-cli`. Full per-path absorb/defer/noise bucketing for this commit is Plan 116-03's job
(D-04); flagged here as `split` with the core-library touch named explicitly so it is not lost
between plans.

---

## Post-Fence Commits (v0.69.0..v0.71.0)

11 commits (live-measured; matches D-01's hypothesis of 11 — no delta).

```bash
git log --no-merges --oneline 59bdace7e905c05c127f480dc6d2a8c3a3331392..0055bf3c686de6fa665decd9831dc5686b282cb0 -- \
  'crates/nono-cli/src/tool-sandbox/' 'crates/nono-cli/src/command_policy.rs' \
  'crates/nono-cli/src/lineage_cgroup.rs' | wc -l
# -> 11
```

| sha | subject | PR# | touched-path summary | windows-touch | disposition |
|-----|---------|:---:|-----------------------|:---:|--------------|
| `c2cb40619e26c5ff987fb9a8dbd1b6a80f1f4294` | fix(tool-sandbox): collapse command-not-found warnings into one summary line | **#1550** | `command_policy.rs`, `tool-sandbox/platform/{linux,macos}.rs` | no | adopt (module-set only) |
| `ce3e510146b0b603970ff087700e25448987fbc1` | fix: allow non-UTF-8 command line arguments | **#1521** | `cli.rs`, `cli_bootstrap.rs`, `command_display.rs`, `command_runtime.rs`, `exec_strategy.rs`, `execution_runtime.rs`, `learn.rs`, `learn_runtime.rs`, `main.rs`, `profile_save_runtime.rs` (production, outside module set) + `tool-sandbox/url_shim.rs` (module set) + `why_runtime.rs` (production, outside) | no | **split** — general CLI-wide non-UTF-8 arg-handling fix touching 10 production files outside the module set; the tool-sandbox touch (`url_shim.rs`) is incidental to the broader fix |
| `65163c6a4e7ee4eef61d737b1e179a02d1ac257c` | feat: mediate vault login -method=oidc (custom inject header + per-command open_port) | **#1476** | `command_policy.rs`, `tool-sandbox/platform/{linux,macos}.rs` (module set) + `profile/credential_provider.rs`, `profile_runtime.rs`, `proxy_runtime.rs` (production, outside) + docs | no | **split** — credential-provider/proxy plumbing outside the module set |
| `35af34175f603a4860b22666dc8a8fc075fe23c6` | fix(cli): add explicit intercept match predicates | #1364 | `crates/nono-cli/data/nono-profile.schema.json` (data), `command_policy.rs`, `tool-sandbox/platform/{linux,macos}.rs`, `tool-sandbox/policy.rs` (module set) + `profile_save_runtime.rs` (production, outside) + tests/docs | no | **split** — one production file (`profile_save_runtime.rs`) outside the module set |
| `6dfcf2772bf84bc1c53af6b7e4a6d5defdf0a3bd` | fix(tool-sandbox): clean up runtime dir with sealed shims | **#1492** | `tool-sandbox/mod.rs`, `tool-sandbox/platform/{linux,macos}.rs` | no | adopt (module-set only) |
| `b4dbd4f6bd465423482d7f0d33b1b89c2744aaf8` | fix(tool-sandbox): grant command interpreter read of its script | **#1467** | `command_policy.rs`, `tool-sandbox/platform/linux.rs` | no | adopt (module-set only) |
| `6a63b42412999b1389a5d59bdca85b8ab2587105` | feat(tool-sandbox): add jwt-shaped nonce option for capture intercepts | **#1453** | `command_policy.rs`, `tool-sandbox/platform/{linux,macos}.rs` (module set) + `crates/nono-proxy/src/{jwt_phantom.rs, lib.rs, oauth_capture/jwt.rs, oauth_capture/mod.rs, oauth_capture/rewrite.rs}` (production, `nono-proxy` crate, outside module set) + docs | no | **split** — substantial `nono-proxy` OAuth-capture work (5 files) outside the module set |
| `82aa2bc95cabc3297e3dcc0aef08190a81e02ac8` | fix(tool-sandbox): let allow_launch_services reach the open shim on macOS | #1464 | `tool-sandbox/env.rs`, `tool-sandbox/platform/macos.rs` | no | adopt (module-set only) |
| `697be7863e2e133474d9f819431dfe47447bbd3d` | fix(tool-sandbox): skip missing fs_write_file grants instead of denying | #1452 | `tool-sandbox/mod.rs`, `tool-sandbox/platform/{linux,macos}.rs` | no | adopt (module-set only) |
| `53cd0580254f925faa32bb8a3a24200b6215c970` | fix(tool-sandbox): drop trailing newline from captured credential phantoms | #1475 | `tool-sandbox/platform/{linux,macos}.rs` | no | adopt (module-set only) |
| `f76733f6f5fea49871b696bff19fb66ef6ab9ff9` | test(nono-cli): hermetic git test commit.gpgsign | #1470 | `capability_ext.rs` (production, outside module set — but diff-verified as `#[cfg(test)] mod tests` block only) + `tool-sandbox/dynamic_providers.rs` (module set) | no | **split** — structurally split (touches `capability_ext.rs`, a production path), but diff-verified test-only content in both files |

**All 6 named post-fence PR numbers from D-01's hypothesis table (#1476, #1492, #1467, #1453,
#1521, #1550) are confirmed present** in the live post-fence table above:
`#1550` -> `c2cb4061`, `#1521` -> `ce3e5101`, `#1476` -> `65163c6a`, `#1492` -> `6dfcf277`,
`#1467` -> `b4dbd4f6`, `#1453` -> `6a63b424`. **Zero of the 6 are absent** — D-01's hypothesis
list is fully confirmed, not merely partially matched.

---

## Fenced-Window Cross-Reference (v0.66.0..v0.69.0, D-13)

**This ledger does not reproduce Phase 108's per-commit tables** — the 20-commit fenced-window
surface (9 pure / 11 split, with the 7 named refinement PRs) is already fully dispositioned in
`108-DIVERGENCE-LEDGER.md` and cross-referenced here, per D-13.

**Spot-verification** (not re-derivation) that the fenced window still yields 20 commits under
this ledger's finalized pathspec (unchanged from D-06's — see D-03 re-derivation above, which
confirmed no change):

```bash
git log --no-merges --oneline d817ed53663c6bba4669ee7a5bfb41b35971fd1b..59bdace7e905c05c127f480dc6d2a8c3a3331392 -- \
  'crates/nono-cli/src/tool-sandbox/' 'crates/nono-cli/src/command_policy.rs' \
  'crates/nono-cli/src/lineage_cgroup.rs' | wc -l
# -> 20
```

**Matches Phase 108's recorded total exactly — no delta.**

**Fenced-window totals:** 20 commits (9 pure / 11 split) — see
`108-DIVERGENCE-LEDGER.md` § "tool-sandbox-pure and tool-sandbox-split".

**7 named refinement PR-to-SHA mappings** (carried forward from `116-CONTEXT.md`'s interfaces
block, cross-referenced not re-derived):

| PR# | sha | pointer |
|-----|-----|---------|
| #1280 | `199bb26699661e8a36de141adc4767617e0a1036` | see `108-DIVERGENCE-LEDGER.md` § tool-sandbox-pure |
| #1322 | `eb2d61a7abc5a354b8f8085b782cd11afa6babb8` | see `108-DIVERGENCE-LEDGER.md` § tool-sandbox-split |
| #1325 | `676a042f3c1e3877f1d1770abd19b0fea0345750` | see `108-DIVERGENCE-LEDGER.md` § tool-sandbox-pure |
| #1384 | `72a988309974f426f0da8f507a4f05df9738ce92` | see `108-DIVERGENCE-LEDGER.md` § tool-sandbox-split |
| #1394 | `bf7ea3cb7faa241de13fe549152e28333b1f0068` | see `108-DIVERGENCE-LEDGER.md` § tool-sandbox-pure |
| #1413 | `ab93cf44e5706b49b1aa5f8ecc1b3b59f29596f8` | see `108-DIVERGENCE-LEDGER.md` § tool-sandbox-pure |
| #1417 | `a519ee62e6d571330caa6a9a7d033fb10bff5939` | see `108-DIVERGENCE-LEDGER.md` § tool-sandbox-split |

---

## Arithmetic Check

**Pre-fence + fenced + post-fence == Task 1's full-window union total:**

```
7 (pre-fence, this ledger) + 20 (fenced, 108-DIVERGENCE-LEDGER.md) + 11 (post-fence, this ledger)
= 38
```

Matches the live 3-path-union measurement in Reproduction (**38**) exactly. **No discrepancy —
no SHA-level reconciliation needed.**

**Split-flag summary (D-02 classification only, no residue accounting yet — Plan 116-03):**

| window | adopt | split | commits |
|--------|------:|------:|--------:|
| pre-fence | 5 | 2 (`c808f000`, `11fd10e0`) | 7 |
| post-fence | 6 | 5 (`ce3e5101`, `65163c6a`, `35af3417`, `6a63b424`, `f76733f6`) | 11 |
| **Total (new, this ledger)** | **11** | **7** | **18** |
| fenced (108, cross-referenced) | — | — | 9 pure + 11 split = 20 |
| **Full 38-commit surface** | 11 adopt (new) + 9 pure (108) = 20 | 7 split (new) + 11 split (108) = 18 | 38 |

No `adapt` or `skip` rows were assigned in this plan's classification pass — every commit in both
new windows either landed cleanly inside the module set (`adopt`) or touched at least one
production path outside it (`split`, per the structural rule). Content-level `adapt`/`skip`
distinctions (e.g. `c808f000`'s trivial org-rename content, `f76733f6`'s test-only content) are
recorded as notes on the `split` rows above rather than reclassifying them, consistent with the
Disposition Vocabulary's structural-first rule.

---

## Discrepancy vs. CONTEXT.md Hypothesis — Summary

Every D-01-hypothesized figure this task re-measured is reconciled here, in one place, per D-17:

| measure | D-01 hypothesis | this session's live measurement | delta |
|---------|----------------:|---------------------------------:|-------|
| Full-window 3-path-union (`v0.64.1..v0.71.0`) | 38 | 38 | **no delta** |
| Pre-fence (`v0.64.1..v0.66.0`) | 7 | 7 | **no delta** |
| Fenced window (`v0.66.0..v0.69.0`, Phase 108) | 20 | 20 (spot-verified) | **no delta** |
| Post-fence (`v0.69.0..v0.71.0`) | 11 | 11 | **no delta** |
| Named post-fence PRs (#1476/#1492/#1467/#1453/#1521/#1550) | 6 named | 6/6 confirmed present | **no delta** |
| Substring-hazard vs. directory-only (`*tool-sandbox*` vs `tool-sandbox/`) | not stated in D-01 (from `116-RESEARCH.md` §2d: 37 vs 35) | 37 vs 35 | **no delta vs. research** |
| `tool-sandbox/` at v0.71.0 file count / LOC | 12 files, 18,433 LOC | not re-derived this task (out of scope for Task 1/2; D-03 only needed `main.rs`/`mod.rs` mod/use content) | not re-measured |
| `command_policy.rs` LOC | 5,955 LOC | not re-derived this task | not re-measured |
| `upstream/main` boundary | `4ede9ccc` (2026-08-05) | `6bd8a393` (2026-08-09) | **moved — expected drift, named explicitly, does not affect the pinned window** |
| D-03 candidate module set | 4 named candidates, in-or-out undetermined | all 4 **OUT**; re-derived set == D-06's inherited 3 unchanged | **D-03 resolved: no expansion** |

**Every measurable D-01 figure this task re-ran agrees with the discussion-time hypothesis
exactly.** This is itself recorded per D-17's "including the case where they agree" rule — a
reviewer should not have to infer "no discrepancy" from silence.

---

*Ledger status: Plan 116-01 (Task 1 + Task 2) complete. Full D-04 residue accounting for the 7
`split`-flagged commits above, the carve-out re-touch check (D-19), and the completeness sweep
are Plan 116-03/116-04's job — this ledger is not yet closed.*
