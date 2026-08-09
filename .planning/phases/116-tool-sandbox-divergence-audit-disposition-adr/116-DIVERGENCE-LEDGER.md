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

---

## Split-Commit Residue Accounting (Pre-Fence + Post-Fence)

**Plan 116-03, Task 1.** Every commit flagged `split` in the Pre-Fence and Post-Fence Commits
tables above gets its own per-path residue table here, following the exact D-04/Phase-108-D-07
convention (`absorb` / `defer` / `noise`, spot-checked row-count-equals-touched-path-count). The
definitive touched-path list per commit is `git show --name-only --format='' <sha>` — **not**
the summary column in the tables above, which is prose shorthand written for readability, not an
authoritative count.

**Reconciliation note (D-17 discipline — recorded in place, not silently corrected):** re-running
`git show --name-only --format='' <sha> | wc -l` against the two pre-fence split commits finds
**31** paths for `c808f000` (the Pre-Fence table's prose said "29 paths") and **91** paths for
`11fd10e0` (the Pre-Fence table's prose said "90 paths"). Both deltas are explained by the prose
summary's comma-separated file lists omitting a few genuinely-touched noise-bucket paths (e.g.
`cliff.toml`, `crates/nono-cli/README.md`, and `crates/nono-cli/data/profile-authoring-guide.md`
for `c808f000`; `Cargo.lock` and `crates/nono-cli/Cargo.toml` for `11fd10e0`) rather than any
disagreement about which commits are `split` or a miscounted `git log`. The two full residue
tables below are built from the live `git show --name-only` output (31 and 91 respectively), not
the prose figures, and each table's own row count is spot-checked against that live count
directly. All five post-fence split commits' live-measured touched-path counts (12/8/8/9/2) match
the Post-Fence table's prose exactly — no discrepancy there.

### Pre-Fence split commits

#### `c808f000db582ad69a10c1a2a1b8221b020fff94` — #1235 chore: migrate GitHub org references from always-further to nolabs-ai (31 paths)

**Finding:** structurally `split` (touches 7 production paths outside the module set:
`migration.rs`, `profile/mod.rs`, `proxy_runtime.rs`, `setup.rs`, `test_env.rs`,
`update_check.rs`, `crates/nono-proxy/src/route.rs`), but every one of those 7 files' diff is a
single-line-or-few-line `always-further` → `nolabs-ai` GitHub-org URL/string substitution
(diff-verified for all 7 — e.g. `migration.rs`'s only hunk is
`- "https://github.com/always-further/nono/discussions/780"` →
`+ "https://github.com/nolabs-ai/nono/discussions/780"`). Zero behavioral content, no requirement
ID — the same "structurally split, cosmetic content" shape 108's own `a519ee62`/`72a98830` rows
already established.

| path | marker | note |
|------|--------|------|
| `.github/workflows/attest-release.yml` | noise | CI workflow config, non-source |
| `.github/workflows/aur-publish.yml` | noise | CI workflow config, non-source |
| `.github/workflows/docs-dispatch.yml` | noise | CI workflow config, non-source |
| `.github/workflows/homebrew-bump.yml` | noise | CI workflow config, non-source |
| `.github/workflows/image-build.yml` | noise | CI workflow config, non-source |
| `.github/workflows/release.yml` | noise | CI workflow config, non-source |
| `.github/workflows/sign-instruction-files.yml` | noise | CI workflow config, non-source |
| `Cargo.toml` | noise | workspace manifest, not source under `crates/*/src/` |
| `README.md` | noise | project readme |
| `SECURITY.md` | noise | project doc |
| `cliff.toml` | noise | changelog-generator config |
| `crates/nono-cli/README.md` | noise | crate readme |
| `crates/nono-cli/data/profile-authoring-guide.md` | noise | data/ |
| `crates/nono-cli/src/command_policy.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/migration.rs` | absorb | production, outside module set — org-URL rename only (verified via diff: one doc-comment link, `always-further`→`nolabs-ai`); would land as a trivial fork-wide org-reference string substitution if ever absorbed, not routed to any feature area — no requirement ID |
| `crates/nono-cli/src/profile/mod.rs` | absorb | production, outside module set — org-URL rename only (verified via diff: one embedded JSON-example string); same trivial-substitution note as above |
| `crates/nono-cli/src/proxy_runtime.rs` | absorb | production, outside module set — org-URL rename only (verified via diff: two test-fixture request-path strings); same trivial-substitution note as above |
| `crates/nono-cli/src/setup.rs` | absorb | production, outside module set — org-URL rename only (verified via diff: two doc/help-text URLs); same trivial-substitution note as above |
| `crates/nono-cli/src/test_env.rs` | absorb | production, outside module set — org-URL rename only (verified via diff: one doc-comment link); same trivial-substitution note as above |
| `crates/nono-cli/src/update_check.rs` | absorb | production, outside module set — org-URL rename only (verified via diff: one test-fixture JSON string); same trivial-substitution note as above |
| `crates/nono-proxy/src/route.rs` | absorb | production, outside module set (different crate) — org-URL rename only (verified via diff: one comment + three test-fixture path strings); same trivial-substitution note as above |
| `docs/docs.json` | noise | docs/ |
| `docs/plans/2026-04-24-issue-594-phase-2-schema-design.md` | noise | docs/ |
| `packaging/aur/README.md` | noise | packaging doc |
| `packaging/rpm/README.md` | noise | packaging doc |
| `scripts/build-rpm.sh` | noise | build script |
| `scripts/downstream-workflows/bump-nono-go.yml` | noise | CI workflow config |
| `scripts/downstream-workflows/bump-nono-py.yml` | noise | CI workflow config |
| `scripts/downstream-workflows/bump-nono-registry.yml` | noise | CI workflow config |
| `scripts/downstream-workflows/bump-nono-ts.yml` | noise | CI workflow config |
| `scripts/push-downstream-workflows.sh` | noise | build/release script |

Row count: 23 noise + 1 defer + 7 absorb = 31 = live-measured touched-path count (31). ✓

---

#### `11fd10e0c88e747b6d751fec9b38f207276b747c` — #1105 feat(sandbox): tool sandbox (91 paths)

**Finding:** the subsystem's own introduction commit — the largest and highest-risk residue
table in this ledger. 13 paths are module set (`command_policy.rs` + the 12
`tool-sandbox/*.rs` files), 22 are non-production noise, and **56** are production paths outside
the module set: 34 general `nono-cli` runtime files, 13 `nono-proxy` crate files, and **9 core
library files** (`crates/nono/src/{audit.rs, keystore.rs, lib.rs, sandbox/linux.rs,
sandbox/mod.rs, scrub.rs, supervisor/mod.rs, supervisor/types.rs, undo/types.rs}`) — the only
pre-fence commit whose non-module residue reaches into the core library tier, directly relevant
to ADR-86's policy-free-library-boundary criterion. Per-file sizes below are from
`git show --stat`; core-library `pub` additions are from a targeted diff grep, both live-measured
this task (D-16).

| path | marker | note |
|------|--------|------|
| `Cargo.lock` | noise | lockfile, not source |
| `crates/nono-cli/Cargo.toml` | noise | crate manifest, not source under `src/` |
| `crates/nono-cli/data/nono-profile.schema.json` | noise | data/ |
| `crates/nono-cli/data/profile-authoring-guide.md` | noise | data/ |
| `crates/nono-cli/src/approval_runtime.rs` | absorb | production, outside module set — new file (+411 lines, `git show --stat`); terminal/webhook/chain approval-backend runtime for the new profile Approval Backends config surface; would land as general CLI approval-runtime infra if ever absorbed, not tool-sandbox-specific |
| `crates/nono-cli/src/audit_commands.rs` | absorb | production, outside module set — new file (+109 lines); CLI-side audit command surface for the new sandbox-runtime/command-policy audit event types |
| `crates/nono-cli/src/audit_event_reader.rs` | absorb | production, outside module set — new file (+83 lines); audit-log event-reader plumbing for the new event types |
| `crates/nono-cli/src/audit_integrity.rs` | absorb | production, outside module set (+79 lines net); audit-ledger integrity checks extended for the new event types |
| `crates/nono-cli/src/audit_ledger.rs` | absorb | production, outside module set (+17 lines); small audit-ledger wiring for the new event types |
| `crates/nono-cli/src/cli.rs` | absorb | production, outside module set (+75 lines net); CLI arg surface extended for tool-sandbox-adjacent dispatch |
| `crates/nono-cli/src/cli_bootstrap.rs` | absorb | production, outside module set (+41 lines); CLI bootstrap wiring |
| `crates/nono-cli/src/command_policy.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/command_runtime.rs` | absorb | production, outside module set (+11 lines net); small command-dispatch wiring |
| `crates/nono-cli/src/config/user.rs` | absorb | production, outside module set (+10 lines); small config-surface addition |
| `crates/nono-cli/src/exec_strategy.rs` | absorb | production, outside module set — substantial (+574 lines net, largest `exec_strategy` delta in this commit); startup-timeout consolidation (`startup_timeout_should_terminate` helper replacing `wait_for_child_with_startup_timeout`/`CHILD_POLL_INTERVAL` per the commit message) plus tool-sandbox launch wiring; would land as general exec-strategy runtime infra if ever absorbed |
| `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` | absorb | production, outside module set (+31 lines net); Linux supervisor-loop wiring for the same startup-timeout consolidation |
| `crates/nono-cli/src/execution_runtime.rs` | absorb | production, outside module set — substantial (+211 lines net); command-execution runtime wiring for tool-sandbox child dispatch |
| `crates/nono-cli/src/launch_runtime.rs` | absorb | production, outside module set (+42 lines net); launch-runtime wiring for the brokered tool-sandbox child |
| `crates/nono-cli/src/main.rs` | absorb | production, outside module set (+23 lines net); crate-root `mod`/dispatch registration for the new `tool_sandbox` module and siblings |
| `crates/nono-cli/src/network_policy.rs` | absorb | production, outside module set (+17 lines); small network-policy addition — proxy env-var passthrough per the commit message's "allow proxy environment variables" entry |
| `crates/nono-cli/src/package_cmd.rs` | absorb | production, outside module set (+15 lines net); small package-command wiring |
| `crates/nono-cli/src/policy.rs` | absorb | production, outside module set (+4 lines); trivial general-policy addition |
| `crates/nono-cli/src/profile/builtin.rs` | absorb | production, outside module set (+2 lines); trivial built-in-profile registration touch |
| `crates/nono-cli/src/profile/mod.rs` | absorb | production, outside module set — substantial (+762 lines net, the single largest profile-schema delta in this commit); adds the Invocation Policies / Endpoint Policies / Approval Backends / Resource Limits / Stdio Limits / mTLS / `allow_all` network-control profile-schema fields the commit message describes |
| `crates/nono-cli/src/profile_cmd.rs` | absorb | production, outside module set — new content (+123 lines); profile-command CLI surface for the new schema fields |
| `crates/nono-cli/src/profile_runtime.rs` | absorb | production, outside module set (+204 lines net); profile-runtime wiring (validation, merge) for the new schema fields |
| `crates/nono-cli/src/proxy_runtime.rs` | absorb | production, outside module set — **largest single-file delta in this entire commit** (+2,506 lines net, `git show --stat`); credential-capture command execution, endpoint-policy enforcement, and mTLS credential wiring per the commit message; would land as a substantial CLI proxy-orchestration feature if ever absorbed |
| `crates/nono-cli/src/pty_proxy.rs` | absorb | production, outside module set (+11 lines net); small PTY-proxy wiring |
| `crates/nono-cli/src/query_ext.rs` | absorb | production, outside module set (+23 lines); small query-extension addition |
| `crates/nono-cli/src/rollback_runtime.rs` | absorb | production, outside module set (+53 lines net); rollback-runtime wiring touched by the shared-struct-field reconciliation the commit message's "reconcile fixtures" entry describes |
| `crates/nono-cli/src/sandbox_prepare.rs` | absorb | production, outside module set (+25 lines net); sandbox-prepare wiring for tool-sandbox child launch |
| `crates/nono-cli/src/supervised_runtime.rs` | absorb | production, outside module set (+41 lines net); supervised-runtime wiring for the brokered child |
| `crates/nono-cli/src/terminal_approval.rs` | absorb | production, outside module set — substantial (+267 lines net); terminal approval-backend implementation for the new Approval Backends profile surface |
| `crates/nono-cli/src/timeouts.rs` | absorb | production, outside module set (−3 lines, a pure removal); trivial cleanup tied to the startup-timeout consolidation |
| `crates/nono-cli/src/tool-sandbox/audit_context.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/credentials.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/dynamic_providers.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/env.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/launch.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/mod.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/platform/linux.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/platform/macos.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/policy.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/protocol.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/token_broker.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/url_shim.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/trust_intercept.rs` | absorb | production, outside module set (+16 lines net); trust-intercept wiring touched by the shared-struct-field reconciliation |
| `crates/nono-cli/src/trust_keystore.rs` | absorb | production, outside module set (+12 lines net); trust-keystore wiring, same reconciliation |
| `crates/nono-cli/src/url_open.rs` | absorb | production, outside module set — new file (+190 lines); "the single source of truth shared by the supervisor and tool-sandbox paths" per the commit message, for URL validation + browser launch (`open_urls`/`allow_launch_services` policy); general-purpose CLI helper despite being introduced alongside tool-sandbox |
| `crates/nono-cli/src/why_runtime.rs` | absorb | production, outside module set — substantial (+335 lines net); CLI "why" diagnostic runtime extended to explain the new invocation-policy/approval/resource-limit denial reasons |
| `crates/nono-cli/src/wiring.rs` | absorb | production, outside module set (+106 lines net); CLI command-wiring registration for the new `profile_cmd`/`audit_commands`/etc. surfaces |
| `crates/nono-cli/tests/profile_cmd.rs` | noise | tests/ |
| `crates/nono-cli/tests/schema_shape.rs` | noise | tests/ |
| `crates/nono-proxy/src/approval.rs` | absorb | production, outside module set (different crate) — new file (+105 lines); proxy-side approval-request plumbing for endpoint policies |
| `crates/nono-proxy/src/audit.rs` | absorb | production, outside module set — substantial (+261 lines); proxy audit record extended with endpoint-policy/credential-capture audit fields |
| `crates/nono-proxy/src/capture.rs` | absorb | production, outside module set — new file (+91 lines); credential-capture command execution on the proxy side |
| `crates/nono-proxy/src/config.rs` | absorb | production, outside module set — new file (+341 lines); proxy config for endpoint policies, approval backends, resource limits, credential capture |
| `crates/nono-proxy/src/credential.rs` | absorb | production, outside module set (+179 lines net); proxy credential resolution extended for command-backed (`cmd://`) credentials |
| `crates/nono-proxy/src/lib.rs` | absorb | production, outside module set (+3 lines); trivial module registration for the new proxy files above |
| `crates/nono-proxy/src/reverse.rs` | absorb | production, outside module set — second-largest delta in the `nono-proxy` crate (+617 lines net); reverse-proxy path extended for endpoint-policy enforcement and credential-capture injection |
| `crates/nono-proxy/src/route.rs` | absorb | production, outside module set (+44 lines net); route matching extended for the `endpoint_policy` field |
| `crates/nono-proxy/src/server.rs` | absorb | production, outside module set (+106 lines net); proxy server wiring for the new approval/capture/config plumbing |
| `crates/nono-proxy/src/tls_intercept/ca.rs` | absorb | production, outside module set (+2 lines net); trivial |
| `crates/nono-proxy/src/tls_intercept/cert_cache.rs` | absorb | production, outside module set (+47 lines net); cert-cache extension for the new mTLS wiring |
| `crates/nono-proxy/src/tls_intercept/handle.rs` | absorb | production, outside module set — third-largest delta in the `nono-proxy` crate (+593 lines net); TLS-intercept handle extended substantially for mTLS client-cert/key wiring |
| `crates/nono-proxy/src/token.rs` | absorb | production, outside module set (+21 lines net); token handling extended |
| `crates/nono/src/audit.rs` | absorb | **core library**, outside module set (+177/−4 lines); new `SandboxRuntimeAuditEvent`/`CommandPolicyAuditEvent`/`CommandPolicyEnvAuditEntry`/`CommandPolicyStdioAudit`/`CommandPolicyStdioStreamAudit` public structs + `record_sandbox_runtime_event()`/`record_command_policy_event()` methods (verified via targeted diff grep of the `pub` surface); ADR-86-boundary-relevant — genuine core-library audit-primitive additions, not CLI plumbing |
| `crates/nono/src/keystore.rs` | absorb | **core library**, outside module set (+36/−1 lines); new `is_cmd_uri()`/`validate_cmd_uri()` public functions (verified via diff grep) recognizing `cmd://` credential-capture URIs; ADR-86-boundary-relevant |
| `crates/nono/src/lib.rs` | absorb | **core library**, outside module set (+7/−3 lines); trivial re-export wiring for the two files above |
| `crates/nono/src/sandbox/linux.rs` | absorb | **core library**, outside module set — new content (+104 lines); new `restrict_execute()` public function on the Landlock driver (verified via diff grep) — an execute-only capability restriction; ADR-86-boundary-relevant, genuine core sandbox primitive |
| `crates/nono/src/sandbox/mod.rs` | absorb | **core library**, outside module set (+16/−1 lines); facade-level `restrict_execute()` plumbing matching the `linux.rs` addition above |
| `crates/nono/src/scrub.rs` | absorb | **core library**, outside module set — substantial (+102/−2 lines); new `add_env_var()`/`remove_env_var()`/`scrub_env_name[_with_policy]()`/`scrub_env_value[_with_policy]()` public functions (verified via diff grep); ADR-86-boundary-relevant |
| `crates/nono/src/supervisor/mod.rs` | absorb | **core library**, outside module set (+86/−20 lines); supervisor IPC trait extended with an `ApprovalRequest`-mediated `request_approval()` method (verified via diff grep) |
| `crates/nono/src/supervisor/types.rs` | absorb | **core library**, outside module set — substantial (+132/−4 lines); new `ApprovalRequest` public enum + accessors (verified via diff grep) |
| `crates/nono/src/undo/types.rs` | absorb | **core library**, outside module set — new content (+63 lines); new enum variants (`ApproveRequested`/`ApproveGranted`/`ApproveDenied`/`ApproveTimeout`/`ApproveError`) and struct fields (`endpoint_policy_action`, `endpoint_policy_rule`, `approval_backend`, `credential_capture_*`) on the audit-record types (verified via diff) |
| `docs/cli/features/credential-injection.mdx` | noise | docs/ |
| `docs/cli/features/dangerous-command-blocking.mdx` | noise | docs/ |
| `docs/cli/features/tool-sandbox.mdx` | noise | docs/ |
| `docs/cli/getting_started/quickstart.mdx` | noise | docs/ |
| `docs/cli/usage/flags.mdx` | noise | docs/ |
| `docs/docs.json` | noise | docs/ |
| `qa-profiles/01-credential-with-rules.json` | noise | non-production fixture data |
| `qa-profiles/02-rules-without-credential.json` | noise | non-production fixture data |
| `qa-profiles/03-mixed-routes.json` | noise | non-production fixture data |
| `tests/integration/test_audit.sh` | noise | integration test script |
| `tests/integration/test_child_tool_boundaries.sh` | noise | integration test script |
| `tests/integration/test_learn.sh` | noise | integration test script |
| `tests/integration/test_network.sh` | noise | integration test script |
| `tests/integration/test_pack_resolution.sh` | noise | integration test script |
| `tests/integration/test_rollback.sh` | noise | integration test script |
| `tests/run_integration_tests.sh` | noise | integration test runner script |

Row count: 22 noise + 13 defer + 56 absorb = 91 = live-measured touched-path count (91). ✓

---

### Post-Fence split commits

#### `ce3e510146b0b603970ff087700e25448987fbc1` — #1521 fix: allow non-UTF-8 command line arguments (12 paths)

**Finding:** a general CLI-wide non-UTF-8 argument-handling fix; the tool-sandbox touch
(`url_shim.rs`, +4/−2 lines) is incidental — the fix's substance is spread across 11 CLI-dispatch
files (`git show --stat`: `cli_bootstrap.rs` +65/−, `exec_strategy.rs` +95/−, `command_display.rs`
+51/−, others smaller).

| path | marker | note |
|------|--------|------|
| `crates/nono-cli/src/cli.rs` | absorb | production, outside module set — part of the general non-UTF-8 arg-handling fix (#1521); would land as a CLI-argument-handling fix if ever absorbed, not tool-sandbox-specific |
| `crates/nono-cli/src/cli_bootstrap.rs` | absorb | production, outside module set — largest single-file delta in this commit (+65 lines net); same non-UTF-8 arg-handling fix |
| `crates/nono-cli/src/command_display.rs` | absorb | production, outside module set (+51 lines net); same fix — non-UTF-8-safe command display formatting |
| `crates/nono-cli/src/command_runtime.rs` | absorb | production, outside module set (+21 lines net); same fix |
| `crates/nono-cli/src/exec_strategy.rs` | absorb | production, outside module set (+95 lines net); same fix |
| `crates/nono-cli/src/execution_runtime.rs` | absorb | production, outside module set (+25 lines net); same fix |
| `crates/nono-cli/src/learn.rs` | absorb | production, outside module set (+24 lines net); same fix |
| `crates/nono-cli/src/learn_runtime.rs` | absorb | production, outside module set (+8 lines net); same fix |
| `crates/nono-cli/src/main.rs` | absorb | production, outside module set (+5 lines net); same fix |
| `crates/nono-cli/src/profile_save_runtime.rs` | absorb | production, outside module set (+59 lines net); same fix |
| `crates/nono-cli/src/tool-sandbox/url_shim.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/why_runtime.rs` | absorb | production, outside module set (+19 lines net); same fix |

Row count: 0 noise + 1 defer + 11 absorb = 12 = live-measured touched-path count (12). ✓

---

#### `65163c6a4e7ee4eef61d737b1e179a02d1ac257c` — #1476 feat: mediate vault login -method=oidc (custom inject header + per-command open_port) (8 paths)

**Finding:** credential-provider/proxy plumbing for an OIDC-login OAuth capture flow, split from
3 tool-sandbox platform files that consume the new per-command `open_port` mediation.

| path | marker | note |
|------|--------|------|
| `crates/nono-cli/src/command_policy.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/profile/credential_provider.rs` | absorb | production, outside module set — largest delta in this commit (+163 lines net); credential-provider plumbing for the OIDC vault-login flow |
| `crates/nono-cli/src/profile_runtime.rs` | absorb | production, outside module set (+4 lines); small profile-runtime wiring |
| `crates/nono-cli/src/proxy_runtime.rs` | absorb | production, outside module set (+65 lines net); proxy-runtime wiring for the custom-inject-header mediation |
| `crates/nono-cli/src/tool-sandbox/platform/linux.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/platform/macos.rs` | defer | module set — fork has no host module for this path |
| `docs/cli/features/sandboxed-oauth-logins.mdx` | noise | docs/ — new file |
| `docs/cli/features/tool-sandbox.mdx` | noise | docs/ |

Row count: 2 noise + 3 defer + 3 absorb = 8 = live-measured touched-path count (8). ✓

---

#### `35af34175f603a4860b22666dc8a8fc075fe23c6` — #1364 fix(cli): add explicit intercept match predicates (8 paths)

**Finding:** the module-set files (`command_policy.rs` +499/−, `tool-sandbox/policy.rs` +553/−)
carry the bulk of this commit's substance; only one production file outside the module set
(`profile_save_runtime.rs`, a 3-line delta) is touched.

| path | marker | note |
|------|--------|------|
| `crates/nono-cli/data/nono-profile.schema.json` | noise | data/ |
| `crates/nono-cli/src/command_policy.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/profile_save_runtime.rs` | absorb | production, outside module set — trivial (+3 lines net); would land as a small profile-save-runtime touch if ever absorbed, not independently significant |
| `crates/nono-cli/src/tool-sandbox/platform/linux.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/platform/macos.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/policy.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/tests/schema_shape.rs` | noise | tests/ |
| `docs/cli/features/tool-sandbox.mdx` | noise | docs/ |

Row count: 3 noise + 4 defer + 1 absorb = 8 = live-measured touched-path count (8). ✓

---

#### `6a63b42412999b1389a5d59bdca85b8ab2587105` — #1453 feat(tool-sandbox): add jwt-shaped nonce option for capture intercepts (9 paths)

**Finding:** substantial `nono-proxy` OAuth-capture-phantom work (5 files, `jwt_phantom.rs` new
+73 lines) outside the module set, alongside the module-set `command_policy.rs`/platform-driver
touches.

| path | marker | note |
|------|--------|------|
| `crates/nono-cli/src/command_policy.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/platform/linux.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-cli/src/tool-sandbox/platform/macos.rs` | defer | module set — fork has no host module for this path |
| `crates/nono-proxy/src/jwt_phantom.rs` | absorb | production, outside module set (different crate) — new file (+73 lines); JWT-shaped phantom-token generation for capture intercepts |
| `crates/nono-proxy/src/lib.rs` | absorb | production, outside module set (+1 line); trivial module registration |
| `crates/nono-proxy/src/oauth_capture/jwt.rs` | absorb | production, outside module set (−19 lines, a pure removal — logic relocated into `jwt_phantom.rs`) |
| `crates/nono-proxy/src/oauth_capture/mod.rs` | absorb | production, outside module set (−1 line); trivial |
| `crates/nono-proxy/src/oauth_capture/rewrite.rs` | absorb | production, outside module set (+2/−1 lines); trivial wiring for the relocated JWT-phantom logic |
| `docs/cli/features/tool-sandbox.mdx` | noise | docs/ |

Row count: 1 noise + 3 defer + 5 absorb = 9 = live-measured touched-path count (9). ✓

---

#### `f76733f6f5fea49871b696bff19fb66ef6ab9ff9` — #1470 test(nono-cli): hermetic git test commit.gpgsign (2 paths)

**Finding:** structurally `split` (touches `capability_ext.rs`, a production path outside the
module set), but diff-verified test-only content in both files.

| path | marker | note |
|------|--------|------|
| `crates/nono-cli/src/capability_ext.rs` | absorb | production, outside module set — but diff-verified the entire hunk is inside `#[cfg(test)] mod tests` (confirmed via `git show ... \| grep -n "mod tests"`, matches at the two hunk headers); a git-command test-hermeticity fix (`.output()`→`.status()` + explicit success assertions, `commit.gpgsign=false`), zero production behavior change; would land as a trivial test-infra touch if ever absorbed |
| `crates/nono-cli/src/tool-sandbox/dynamic_providers.rs` | defer | module set — fork has no host module for this path |

Row count: 0 noise + 1 defer + 1 absorb = 2 = live-measured touched-path count (2). ✓

---

### Split-Commit Residue Accounting — arithmetic summary

7 split commits (2 pre-fence + 5 post-fence), 31 + 91 + 12 + 8 + 8 + 9 + 2 = **161** total
touched-path rows across their residue tables, zero unbucketed paths — every commit's table row
count spot-checked equal to its live `git show --name-only --format='' <sha> | wc -l` output
directly above the table. Precise per-commit bucket breakdown:

| commit | absorb | defer | noise | total |
|--------|-------:|------:|------:|------:|
| `c808f000` | 7 | 1 | 23 | 31 |
| `11fd10e0` | 56 | 13 | 22 | 91 |
| `ce3e5101` | 11 | 1 | 0 | 12 |
| `65163c6a` | 3 | 3 | 2 | 8 |
| `35af3417` | 1 | 4 | 3 | 8 |
| `6a63b424` | 5 | 3 | 1 | 9 |
| `f76733f6` | 1 | 1 | 0 | 2 |
| **Total** | **84** | **26** | **51** | **161** |

84 + 26 + 51 = 161. ✓ Matches the sum of the 7 individual row counts above exactly.

Every `defer`-marked row across all 7 tables carries the literal phrase "fork has no host module
for this path" (26 occurrences — one per module-set path across `command_policy.rs` and the
`tool-sandbox/*.rs` files touched by these 7 commits, matching the table's `defer` column sum).
The module-scoped half of every split commit always defers for exactly this reason: the fork has
never absorbed any part of the `tool-sandbox`/`command_policy`/`lineage_cgroup` subsystem
(D-06/108's standing finding, re-confirmed by this ledger's own D-03 re-derivation above).

---

## Fenced-Window Residue Reconciliation (grep-verified against fork HEAD)

**Plan 116-03, Task 2.** Phase 108's `108-DIVERGENCE-LEDGER.md` § "tool-sandbox-split" recorded
11 split commits; **7 of the 11 carry zero `absorb`-marked rows** (`a519ee62`, `72a98830`,
`1f54f4ae`, `7c20dc75`, `eb2d61a7`, `5a7447d3`, `ebd51cbb` — every non-module path in each is
wiring-only, per 108's own finding, quoted here rather than re-derived: *"7 of 11 do not [carry
absorb content] ... every non-module path in those 7 is small wiring"*). Those 7 need no
reconciliation table row (there is nothing to verify a landing for). The remaining **4 split
commits carry all 31 of the fenced window's `absorb`-marked paths** — exactly the 4 D-05-named
worked examples this plan's `interfaces` block names. Every one of the 31 is reconciled below by
a live grep against fork HEAD (not by reading 108's routing-note phase-number citation as
sufficient proof), per D-04/D-16.

### `d5803b994b416ad07a73907143ca169c408917f3` — #1398 port-range support (PROF-03 claim)

108's routing-note claim (quoted verbatim): *"ranges 'expand to individual rules on both
platforms — Seatbelt rules on macOS, Landlock NetPort objects on Linux,' implemented entirely in
`crates/nono/src/sandbox/linux.rs`/`sandbox/macos.rs` + `crates/nono/src/capability.rs` (the
port-range `CapabilitySet` mechanism) + CLI plumbing (`profile/mod.rs`, `profile_cmd.rs`,
`profile_runtime.rs`, `capability_ext.rs`, `exec_strategy.rs`,
`exec_strategy/supervisor_linux.rs`, `output.rs`, `supervised_runtime.rs`, `manifest_convert.rs`,
`capability-manifest.schema.json`)... (**PROF-03**)."

| absorb path | grep command | hit count | verdict |
|--------------|---------------|-----------|---------|
| `crates/nono-cli/src/capability_ext.rs` | `grep -n "port_range" crates/nono-cli/src/capability_ext.rs` | 6 (`open_port_range` field reads + 1 test) | landed |
| `crates/nono-cli/src/exec_strategy.rs` | `grep -n "port_range" crates/nono-cli/src/exec_strategy.rs` | 5 (`proxy_bind_port_ranges` field + uses) | landed |
| `crates/nono-cli/src/exec_strategy/supervisor_linux.rs` | `grep -n "port_range" crates/nono-cli/src/exec_strategy/supervisor_linux.rs` | 5 | landed |
| `crates/nono-cli/src/output.rs` | `grep -n "port_range" crates/nono-cli/src/output.rs` | 2 (`localhost_port_ranges()` display) | landed |
| `crates/nono-cli/src/profile/mod.rs` | `grep -n "port_range" crates/nono-cli/src/profile/mod.rs` | 5+ (`open_port_range`/`listen_port_range` schema fields) | landed |
| `crates/nono-cli/src/profile_cmd.rs` | `grep -n "port_range" crates/nono-cli/src/profile_cmd.rs` | 5+ | landed |
| `crates/nono-cli/src/profile_runtime.rs` | `grep -n "port_range" crates/nono-cli/src/profile_runtime.rs` | 3+ (`validate_port_ranges()`, explicit `PROF-03 (110-04)` doc-comment citation) | landed |
| `crates/nono-cli/src/supervised_runtime.rs` | `grep -n "port_range" crates/nono-cli/src/supervised_runtime.rs` | 1 (`proxy_bind_port_ranges`) | landed |
| `crates/nono/schema/capability-manifest.schema.json` | `grep -n "localhost_range" crates/nono/schema/capability-manifest.schema.json` | 1 field def (`PortConfig.localhost_range`, `[start,end]` array-of-pairs shape) | landed — differently-named (`localhost_range`, not `localhost_port_ranges`), functionally equivalent inclusive-range shape, noted explicitly per Task 2's differently-named-symbol allowance |
| `crates/nono/src/capability.rs` | `grep -n "port" crates/nono/src/capability.rs \| head` | 10+ (`NetworkMode::ProxyOnly{port,bind_ports}`, `MACOS_PORT_RANGE_LIMIT`, `merge_port_ranges`) | landed |
| `crates/nono/src/manifest_convert.rs` | `grep -n "localhost_range" crates/nono/src/manifest_convert.rs` | 4 (reads `ports.localhost_range`, calls `caps.allow_localhost_port_range()`) | landed |
| `crates/nono/src/sandbox/linux.rs` | `grep -n "NetPort" crates/nono/src/sandbox/linux.rs` | 7 (`NetPort::new(*port, AccessNet::ConnectTcp\|BindTcp)` — kernel-enforced Landlock NetPort objects) | landed |
| `crates/nono/src/sandbox/macos.rs` | `grep -n "port_range\|remote tcp" crates/nono/src/sandbox/macos.rs` | 6+ (`(remote tcp "localhost:{}")` Seatbelt-rule emission, `localhost_port_ranges` unrolling, `MACOS_PORT_RANGE_LIMIT` cumulative cap) | landed |

**All 13 of `d5803b99`'s `absorb`-marked paths verdict: `landed`.** Port-range support exists on
both platforms, kernel-enforced on Linux (`NetPort`) and Seatbelt-rule-emitted on macOS, matching
108's routing-note claim symbol-for-symbol (with one benign field-name difference,
`localhost_range` vs. `localhost_port_ranges`, noted above).

### `ea334d2bbdcb332c3a1c4164843667b0d2153cb1` — #1332 musl build fix (fs_type_unsupported u64) claim

108's routing-note claim (quoted verbatim): *"the substantive fix is entirely in
`crates/nono/src/sandbox/linux.rs` (`V9FS_MAGIC`/`fs_type_unsupported` retyped `libc::c_long` →
`u64` to fix an Alpine/musl build break — musl defines `statfs::f_type` as `u64` vs glibc's
`c_long`)."*

| absorb path | grep command | hit count | verdict |
|--------------|---------------|-----------|---------|
| `crates/nono/src/sandbox/linux.rs` | `grep -n "V9FS_MAGIC\|fs_type_unsupported" crates/nono/src/sandbox/linux.rs` | 8 hits — **but** `const V9FS_MAGIC: libc::c_long` and `fn fs_type_unsupported(f_type: libc::c_long)` (types unchanged from the pre-fix upstream shape, not retyped to `u64`) | **not-landed — false-positive grep hit** |

**Finding (this is the genuine Task-2 catch this plan exists to make):** the fork's
`V9FS_MAGIC`/`fs_type_unsupported` symbols are a **coincidentally-identically-named, unrelated,
fork-native feature** — 9P-filesystem (WSL2 host-path) Landlock-enforcement-limitation detection,
introduced independently via `git log --oneline --all -- crates/nono/src/sandbox/linux.rs \|
grep -i "wsl\|9p"` (finds `c786b063 feat(wsl2): add WSL2 detection...`, `5fe9d203
fix(wsl2): security hardening from code review`, `345891fa`/`5b8e94da`/`4b0b6870 fix(sandbox):
warn when capability path is on a 9P filesystem` — none reference the tool-sandbox split commit
or a musl build target). The fork's `V9FS_MAGIC` const still uses `libc::c_long` (unchanged from
the pre-fix upstream shape `ea334d2b` corrected), and `Cross.toml`/`.github/workflows/*.yml`
contain no musl cross-compilation target (`grep -rn "musl" Cross.toml .github/workflows/*.yml`
finds only one unrelated hit — a third-party SPIFFE binary tarball name in `spire.yml`, not a
fork build target). **The musl-specific build-compatibility fix `ea334d2b` actually made has
never landed in the fork — a naive grep for the symbol names alone would have reported `landed`
and been wrong**, exactly the "file/symbol presence ≠ the actual fix" trap D-16 warns against.
Verdict: **not-landed**, with the false-positive risk recorded explicitly so a future reader does
not re-trip on the same symbol-name coincidence.

### `d4927f95a37863cf0ba534b054e28f48f002ad21` — #1298 @git:* dynamic token expansion (PROF-02 claim)

108's routing-note claim (quoted verbatim): *"`capability_ext.rs` is where the top-level
`filesystem.allow`/`read`/`write` fields gain `@git:*` token expansion (**PROF-02**)... the fix
works by calling `crate::tool_sandbox::dynamic_providers::expand_dynamic_tokens` — a function
that lives in `tool-sandbox/dynamic_providers.rs`... PROF-02's absorb work in Phase 110 cannot be
a clean lift of `capability_ext.rs` alone... Phase 110 must either port a minimal standalone
`expand_dynamic_tokens`... or explicitly scope PROF-02 down."*

| absorb path | grep command | hit count | verdict |
|--------------|---------------|-----------|---------|
| `crates/nono-cli/src/capability_ext.rs` | `grep -n "@git:\|expand_dynamic_tokens\|dynamic_providers" crates/nono-cli/src/capability_ext.rs` | 8 (`expand_dynamic_tokens()` called for `allow`/`read`/`write`/`allow_file`/`read_file`/`write_file`/`add_deny_access` fields) | landed |
| — cross-dependency resolution check | `grep -n "@git:" crates/nono-cli/src/dynamic_tokens.rs` (108's flagged blocker: does the fork have its own standalone `expand_dynamic_tokens`, resolving 108's own open item?) | 10+ (`@git:config-files`, `@git:hooks-path`, `@git:common-dir`, `@git:worktree` token parsing/tests) | **resolved** — the fork built its own standalone `crates/nono-cli/src/dynamic_tokens.rs` (not a port of `tool-sandbox/dynamic_providers.rs`) rather than deferring per 108's proposed fallback; `capability_ext.rs:17` carries an explicit `// PROF-02b (Phase 110 Plan 02, D-01/D-02): @git:* dynamic-token expansion is` doc-comment confirming this was a deliberate, already-recorded Phase 110 decision |

**Both of `d4927f95`'s tracked items verdict: `landed`.** 108's own flagged cross-dependency
concern (that `capability_ext.rs` alone would not be a clean lift) was correctly anticipated and
is confirmed resolved — Phase 110 built a standalone `dynamic_tokens.rs`, not a port of the
deferred `tool-sandbox/dynamic_providers.rs` module.

### `8a4237f2ee0dc33bc1e5afdd39c0db8ca6f5ed38` — #1283 SeccompPolicy struct refactor claim (16 absorb paths)

108's routing-note claim (quoted verbatim): *"a general Linux seccomp/sandbox-enforcement-
selection mechanism (`SeccompPolicy` struct, `apply_auto`/`apply_landlock`/`apply_external` entry
points, new `--sandbox-policy` CLI flag)... filed as a CORE-cluster / Phase 111 residual item."*

| absorb path | grep command | hit count | verdict |
|--------------|---------------|-----------|---------|
| `crates/nono/src/sandbox/linux.rs`, `sandbox/mod.rs` | `grep -rn "SeccompPolicy\|apply_auto\|apply_landlock" crates/nono/src/sandbox/mod.rs crates/nono/src/sandbox/linux.rs` | 0 hits for `SeccompPolicy`/`apply_auto`/`apply_landlock` (only `apply_external` exists, see next row) | **not-landed** |
| `crates/nono/src/sandbox/linux.rs::apply_external` | `grep -n "fn apply_external" crates/nono/src/sandbox/linux.rs` + doc-comment read | 1 hit, but the fork's `apply_external()` is a no-op marker (`info!("TCP network enforcement delegated externally"); Ok(())`) documented as "exists only as an explicit marker for callers that have already applied the normal filesystem and process sandbox... It must not be used as the whole `nono run` sandbox on its own" | **not-landed** — same function name, structurally unrelated stub, not the SeccompPolicy dispatch mechanism |
| `crates/nono-cli/src/cli.rs::--sandbox-policy` | `grep -n "sandbox.policy\|sandbox_policy" crates/nono-cli/src/cli.rs` | 0 hits | **not-landed** |
| `bindings/c/src/sandbox.rs::apply_auto` | `grep -n "apply_auto\|Sandbox::apply" bindings/c/src/sandbox.rs` | 1 hit, `nono::Sandbox::apply` (not `apply_auto`) | **not-landed** |
| — direct fork self-documentation (stronger than a fresh grep) | `sed -n '5658,5667p' crates/nono/src/sandbox/linux.rs` (an existing Phase-112 test comment) | literal text: *"this fork never absorbed the antecedent `LinuxSandboxPolicy`/`SeccompPolicy` refactor (fa21a004/8a4237f2, #1283)... the fork's equivalent of upstream's `apply_landlock()` is named `apply()`... no such function named `apply_landlock` exists in this fork"* | **not-landed — independently confirmed by the fork's own prior-recorded code comment, citing this exact SHA** |

**All 16 of `8a4237f2`'s `absorb`-marked paths verdict: `not-landed`.** This is the strongest
finding in this reconciliation: not only does a live grep fail to find the claimed symbols, but
the fork's own code (from an unrelated earlier phase, Phase 112 SEC-03) already explicitly
documents non-absorption of this exact SHA by name — independent corroboration that 108's
`absorb` marking for this commit's 16 paths never executed. 108's own residue table already
filed this as "Phase 111 residual" (a forward-pointing marker, not a landed-absorb claim) — this
finding does not contradict 108, it confirms 108's own residual framing was accurate and the
residual has not since been picked up by any subsequent phase (Phase 111 through 116 inclusive).

### Fenced-Window Residue Reconciliation — summary

| commit | absorb paths | landed | not-landed | partially-landed |
|--------|-------------:|-------:|-----------:|------------------:|
| `d5803b99` (#1398, PROF-03) | 13 | 13 | 0 | 0 |
| `ea334d2b` (#1332, musl fix) | 1 | 0 | 1 | 0 |
| `d4927f95` (#1298, PROF-02) | 1 | 1 | 0 | 0 |
| `8a4237f2` (#1283, SeccompPolicy) | 16 | 0 | 16 | 0 |
| **Total** | **31** | **14** | **17** | **0** |

31 of 31 fenced-window `absorb`-marked paths reconciled by a recorded live grep (or, for
`8a4237f2`, an even stronger existing-code-comment citation) — zero accepted on routing-note
prose alone. **14 landed** (all of PROF-02's and PROF-03's claimed absorb work), **17 not-landed**
(all of SeccompPolicy's claimed work, plus the musl fix's one path — the latter a genuine
false-positive-symbol-collision finding this task's grep-not-prose discipline exists to catch).

---

## Post-Fence Residue Finding

Per Task 1's post-fence split-commit residue tables above, every post-fence commit's
`absorb`-marked path is checked the same way: grep fork HEAD for a matching symbol/feature, per
D-04's requirement that post-fence residue mapping to no existing phase be named as an
operator-gated finding.

| absorb path (post-fence) | commit | grep command | hit count | verdict |
|----------------------------|--------|---------------|-----------|---------|
| `crates/nono-cli/src/cli.rs` (+ 10 sibling files) | `ce3e5101` (#1521, non-UTF-8 args) | `grep -n "OsStr\|from_encoded_bytes\|OsString" crates/nono-cli/src/cli_bootstrap.rs` | 3 (`OsString`/`OsStr` handling present) | landed — fork independently handles non-ASCII/OS-string args in its own CLI bootstrap path (pre-existing, not traced to this specific upstream commit, but the capability class exists) |
| `crates/nono-cli/src/profile/credential_provider.rs` (+ 2 siblings) | `65163c6a` (#1476, OIDC vault-login) | `grep -n "oidc\|vault" crates/nono-cli/src/profile/credential_provider.rs` | 0 | **not-landed** — no OIDC/Vault-specific credential-provider mediation in the fork |
| `crates/nono-cli/src/profile_save_runtime.rs` | `35af3417` (#1364, intercept predicates) | `grep -n "intercept" crates/nono-cli/src/profile_save_runtime.rs` | 0 | **not-landed** — no tool-sandbox intercept concept exists in the fork at all (module absent) |
| `crates/nono-proxy/src/jwt_phantom.rs` (+ 4 siblings) | `6a63b424` (#1453, jwt-shaped nonce) | `grep -rn "jwt_phantom\|jwt.shaped" crates/nono-proxy/src/` | 0 | **not-landed** — no JWT-phantom capture-intercept mechanism in the fork's proxy |
| `crates/nono-cli/src/capability_ext.rs` (test-only) | `f76733f6` (#1470, hermetic git test) | N/A — diff-verified test-only in the source commit itself; no fork-side landing question applies | N/A | not-applicable (test-infra content, not a feature to land) |

**Of the 4 genuinely feature-bearing post-fence absorb paths, 3 map to no existing phase and no
landed fork symbol** (`65163c6a`'s OIDC vault-login mediation, `35af3417`'s intercept-predicate
refinement, `6a63b424`'s JWT-phantom capture nonce) — but **all 3 are module-scoped extensions of
the tool-sandbox subsystem itself** (OIDC credential mediation for a tool-sandbox-brokered login
flow; intercept-match predicates for the tool-sandbox `policy.rs` intercept action; JWT-shaped
nonces for tool-sandbox capture intercepts), not standalone features severable from the deferred
base subsystem. None of the 3 has independent value absent the tool-sandbox module itself landing
first — the same "wiring/refinement for a deferred base" shape Task 1's residue tables already
established for the bulk of the fenced-window's non-absorb split commits.

**This finding: no post-fence residue maps to no phase in a way that needs a *new* successor
phase.** The natural home for all 3 unresolved items (per CONTEXT.md's own framing) is a future
absorb of the tool-sandbox subsystem itself (whatever phase executes this ADR's verdict if it is
Pole A, or UPST13/FUT-08 if a future sync re-examines the window) — not a standalone new phase or
FUT item for these 3 refinement commits individually. **This finding is a proposal; it is not
applied to `.planning/ROADMAP.md` or `.planning/REQUIREMENTS.md` by this phase — successor phase
or FUT-item creation requires operator approval.** `git diff --stat -- .planning/ROADMAP.md
.planning/REQUIREMENTS.md` shows zero changes after this plan's tasks (verified below).

---

*Ledger status: Plan 116-03 (Task 1 + Task 2) complete. Split-commit residue accounting for all 7
pre-fence/post-fence `split` commits is done (161 rows, zero unbucketed). The fenced window's 31
`absorb`-marked paths are grep-verified against fork HEAD (14 landed, 17 not-landed — including
one genuine false-positive-symbol-collision catch). Post-fence residue is disposed as "no new
successor phase needed; 3 unresolved items are module-scoped extensions of the still-deferred
tool-sandbox subsystem itself" — a proposal, not applied to ROADMAP.md/REQUIREMENTS.md. Remaining
work for this ledger (per `116-01-PLAN.md`'s original scope split): the D-03-adjacent module-set
candidate re-touch and any further completeness sweep are Plan 116-04's job, if named in that
plan — this ledger is not yet closed.*
