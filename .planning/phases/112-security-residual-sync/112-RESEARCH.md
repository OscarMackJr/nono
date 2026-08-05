# Phase 112: Security + Residual Sync - Research

**Researched:** 2026-08-05
**Domain:** Upstream-absorb diff-verification (not greenfield) — 18-commit `security-residual-and-misc` cluster from `nolabs-ai/nono` `v0.66.0..v0.69.0`
**Confidence:** HIGH (all 18 SHAs resolved locally via `git show`; every disposition below is cited to a `git show`/`file:line` excerpt against the live tree, not inferred from commit subjects or the ledger's prose alone)

## Summary

This is a diff-verification pass, not a stack survey. All 18 SHAs resolve locally (`upstream` remote already fetched); every disposition in the table below was produced by running `git show --stat`/`git show <sha> -- <file>` against the commit and cross-checking the named target files against the live fork tree with `Read`/`Grep`/`Bash ls`.

**Headline finding (answers decision-critical question 1): the heavyweights do NOT behave symmetrically, and D-04's collapse permission should be exercised for two of the three, not zero.**

- **SEC-01 (`0ecc476b`, AWS SigV4 proxy auth)** is essentially **0% adoptable**. Of 11 files, the two subsystems it creates/modifies (`nono-proxy/src/aws/*` — new, 846 of ~1,185 non-lockfile insertion lines; `nono-proxy/src/tls_intercept/*` — modified, 213 lines) are **both entirely absent from the fork**. The fork's own `credential.rs` carries an explicit placeholder (`aws_routes: HashMap<String, ()>`, "placeholder until full SigV4 signing is implemented") and `reverse.rs` returns a hand-authored `501 Not Implemented` guard citing a prior fork decision (D-15) to skip this exact subsystem. This is the origin commit of the same absent subsystem that made Phase 109's `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` disposition `6fb7ecbf`/`23d93fc9` (later bug-fixes to these same files) `won't-sync (target subsystem absent)`. **Recommend: won't-sync, with a finding document mirroring `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md`.**
- **SEC-02a (`9b692e07`, declarative sandboxed OAuth capture)** is **partially adoptable but heavily entangled** with the same absent `tls_intercept/` subsystem plus a second absent file, `nono-proxy/src/forward.rs` (not created by this commit — it *modifies* a pre-existing upstream `forward.rs` the fork never absorbed, from ancestor commit `149abde0`). Roughly 9 of 26 files (the core `oauth_capture/` module, `tls_intercept/{h2_forward,handle}.rs`, `forward.rs`, plus the not-yet-existing `proxy_command.rs`) depend on subsystems the fork lacks; the remaining ~17 files (schema, docs, `profile/mod.rs`, `profile_runtime.rs`, `proxy_runtime.rs`, `nono-proxy/config.rs`, `filter.rs`, `server.rs`) are present and could host an adapted version of the feature that bypasses the response-rewrite-via-`forward.rs` hook. **This does not collapse to won't-sync outright — it is the same architectural shape as NET-02/SPIFFE (moved to its own Phase 113 for entanglement with fork-divergent `tls_intercept.rs`). Recommend: adapt-down significantly (author the schema/credential_provider layer against `server.rs` directly, skip the `forward.rs`-dependent response-rewrite hook) OR split to a future dedicated phase if the adapt proves as large as SPIFFE's.**
- **SEC-07 (`2663e990`, standalone `nono proxy` command)** is the one heavyweight that is a **genuine, clean absorb**. All 12 touched files (`app_runtime.rs`, `cli.rs`, `cli_bootstrap.rs`, `main.rs`, `proxy_runtime.rs`, and 7 `nono-proxy` files) exist in the fork; the fork has the proxy modules but zero `Proxy` variant in the `Commands` enum (confirmed: `grep -n "enum Commands" crates/nono-cli/src/cli.rs` → line 646; `grep -n "Proxy" crates/nono-cli/src/cli.rs` returns no enum-variant hit). **Recommend: adopt, own plan, own wave — as D-04 originally sized it.**

**Sequencing dependency the planner must respect:** upstream authored SEC-07 (`2663e990`, 2026-07-04) *before* SEC-02a (`9b692e07`, 2026-07-05). `2663e990` is the commit that **creates** `crates/nono-cli/src/proxy_command.rs` (696 lines, new file); `9b692e07` then makes a 6-line edit to that same file. If SEC-02a is absorbed in any form, SEC-07 must land first.

**A load-bearing negative finding not anticipated by CONTEXT.md: SEC-08 (`a5a441c2`) would relax an existing fork security guard if adopted verbatim.** The fork already implements fail-closed semantics for empty `allow_vars` — landed in a prior phase ("Plan 34-08a Task 3/5, D-20 manual replay of upstream `1b412a7`/`780965d7`"), predating this sync window. Upstream's `a5a441c2` changes the *meaning* of an omitted-vs-explicit-empty `allow_vars` in a way that is incompatible with the fork's own fail-closed default (see Threat Model input, below, and the SEC-08 table row). **This is the `adopt-that-loosens-a-guard` case D-05 names as the one contingent ADR-112 trigger.**

D-01's premise for SEC-09 (target subsystem absent) is **confirmed true** by direct grep: zero hits for `command_policies`, `tool_sandbox_runtime`, `tool_sandbox_initial_shim`, `env_credentials` in `crates/nono-cli/src/execution_runtime.rs`. D-07's premise (crossbeam-epoch already 0.9.20, `cargo audit` clean) is confirmed true directly in `Cargo.lock`.

**Primary recommendation:** run D-02's reality-check pass largely pre-computed by this research — SEC-07/SEC-03/SEC-04/SEC-05/SEC-06/SEC-08(as won't-sync-or-careful-adapt)/SEC-09 are settled with HIGH confidence; SEC-01 is settled won't-sync with HIGH confidence; SEC-02a/b/c need one focused reality-check task to decide adapt-down-scope vs. defer-to-future-phase before any absorb plan is written. RES-01/RES-02 are individually assessed below per D-03's skip bias.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| AWS SigV4 proxy authentication (SEC-01) | API/Backend (`nono-proxy` crate) | — | Credential-injection/signing logic lives entirely in the proxy process, never the sandboxed child; N/A pending won't-sync disposition |
| Declarative OAuth capture (SEC-02) | API/Backend (`nono-proxy`) + CLI policy (`nono-cli` profile schema) | — | Token capture/rewrite is a proxy-layer interception concern; the declarative provider config is CLI-owned profile schema, per the existing `network_policy.rs`/`profile/mod.rs` split |
| NVIDIA procfs mediation (SEC-03) | Library primitive (`crates/nono/src/sandbox/linux.rs`) | CLI wiring (`sandbox_prepare.rs`, `capability_ext.rs`) | Landlock/seccomp mechanism is core-library; the `--allow-gpu` flag and policy decision of *when* to apply it is CLI-side, matching the existing split |
| Trust-policy predicate discriminator (SEC-04) | Library primitive (`crates/nono/src/trust/types.rs`) | CLI UX (`trust_cmd.rs`) | Adds a data field + parse-time check to `TrustPolicy` — observability/format-detection, not policy |
| Landlock `Refer` execute-restriction grant (SEC-05) | Library primitive (`crates/nono/src/sandbox/linux.rs`) | — | Pure Landlock ruleset-construction bugfix inside `restrict_execute()`; no CLI involvement |
| Seccomp supervisor-ancestry / orphan reaping (SEC-06) | CLI (`crates/nono-cli/src/exec_strategy.rs`, supervisor loop) | — | `PR_SET_CHILD_SUBREAPER` + `waitpid` orphan-reap is process-supervision logic, which CLAUDE.md's boundary table places CLI-side (`ExecStrategy` selection/supervision) |
| Standalone `nono proxy` command (SEC-07) | CLI (`cli.rs` Commands enum, `proxy_command.rs`) | API/Backend (`nono-proxy` server/route/connect/external/reverse/token) | New CLI subcommand wiring plus proxy-crate support for standalone (non-sandboxed-child) operation |
| `allow_vars` empty-list semantics (SEC-08) | CLI (`crates/nono-cli/src/profile/mod.rs`, `profile_runtime.rs`) | — | Profile-schema field semantics and the runtime resolution that turns it into an allow-list are both CLI-owned policy |
| Non-shim credential/policy guard (SEC-09) | CLI (`execution_runtime.rs`) | — | N/A — target guard confirmed absent from the fork; no code to place |
| Registry namespace / update-check headers (RES-01) | CLI (`registry_client.rs`, `update_check.rs`, `profile/*`) | OS-registered state (HKLM machine-policy spine, v3.0) | Update-check behavior interacts with the fork's own `HKLM\SOFTWARE\Policies\nono` spine, which upstream has no equivalent of |
| PTY teardown late-reply drain (RES-02) | CLI (`pty_proxy.rs`) | — | Terminal I/O teardown is CLI-side process supervision, same tier as SEC-06 |

## Standard Stack

Not applicable in the conventional sense — this phase absorbs existing upstream commits rather than selecting new libraries. The only net-new dependency surface across all 18 commits is SEC-01's (a full AWS SigV4 signing implementation would need an HMAC-SHA256/canonical-request crate; `Cargo.lock`'s 688-line diff and `nono-proxy/Cargo.toml`'s +6 lines are almost entirely this). Given the won't-sync recommendation for SEC-01, **no new external dependency is expected to be introduced by this phase** if that recommendation is followed. If the planner instead attempts a partial SEC-01 adopt, budget a full Package Legitimacy Audit for whatever SigV4 crate is chosen — none is verified here because none is recommended for install.

**Version verification:** N/A (no new packages recommended).

## Package Legitimacy Audit

**Not applicable.** This phase's primary path (per the findings above) installs zero new external packages: SEC-01 is won't-sync, SEC-02a's recommended adapt-down path re-uses existing fork dependencies (`nono-proxy`'s current `tokio`/`hyper`/`rustls` stack), and every other commit is a pure code change with no new `Cargo.toml` entries beyond what upstream's own diffs already show landing in files the fork does not have. If the planner elects a different SEC-01/SEC-02a disposition that requires a new crate, re-run the Package Legitimacy Gate protocol against that specific crate before finalizing the plan — this research did not identify one to check.

## Per-SHA Disposition Table (Primary Deliverable)

All 18 SHAs confirmed to resolve locally: `git cat-file -t <sha>` returned `commit` for all 18 (verified live 2026-08-05, no network fetch required).

| SHA (short) | Req | Upstream diffstat | Fork target path(s) | Already present in fork? | Provisional disposition | Cited evidence | Confidence |
|---|---|---|---|---|---|---|---|
| `0ecc476b` | SEC-01 | 11 files, +1,873/-412 | `nono-proxy/src/aws/*` (new: mod.rs, sign.rs, route.rs, endpoints.rs — 846 lines), `nono-proxy/src/tls_intercept/{h2_forward,handle}.rs` (213 lines), `credential.rs`, `lib.rs`, `test_env.rs`, `Cargo.toml`/`Cargo.lock` | **NO** — `aws/` and `tls_intercept/` directories do not exist anywhere in `crates/nono-proxy/src/` (`ls crates/nono-proxy/src/` lists 15 files, neither dir present) | **won't-sync (target subsystem absent)** | `crates/nono-proxy/src/credential.rs:131-134`: `aws_routes: HashMap<String, ()>, /* placeholder until full SigV4 signing is implemented */`; `crates/nono-proxy/src/reverse.rs:260-268`: hand-authored `501 Not Implemented` guard citing prior fork decision D-15 ("upstream's 501 is in tls_intercept/handle.rs which the fork does not have"); `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` independently confirms both dirs absent as of Phase 109 (2026-07-29) — this commit is the origin of that same absent subsystem | HIGH |
| `9b692e07` | SEC-02a | 26 files, +4,425/-163 | `oauth_capture/*` (new, 5 files, 1,156 lines — absent), `tls_intercept/{h2_forward,handle}.rs` (absent), `forward.rs` (absent — created by ancestor `149abde0`, never absorbed), `proxy_command.rs` (absent until SEC-07 lands), `credential_provider.rs` (new, 391 lines, freestanding — buildable), 17 other files (schema.json, docs, `profile/mod.rs`, `profile_runtime.rs`, `proxy_runtime.rs`, `launch_runtime.rs`, `main.rs`, `policy.rs`, `sandbox_prepare.rs`, `nono-proxy/{config,filter,server,lib}.rs`) — all present | **PARTIAL** — ~17/26 files present; the feature's core mechanism (response-rewrite hook via `forward.rs::ResponseRewrite`) targets an absent file | **adapt-down significantly, OR defer to a future phase (SPIFFE/#1272 precedent)** — not a clean adopt | `git log --oneline --all --diff-filter=A -- crates/nono-proxy/src/forward.rs` → only `149abde0` (not in this 18-commit list, never absorbed); `git show 9b692e07 -- crates/nono-proxy/src/forward.rs` shows a *modify* diff (`--- a/...` not `/dev/null`) against a file this fork's `ls` confirms absent; fork's `oauth2.rs` (573 lines, present) is a **different** feature (client_credentials token exchange for credential injection), confirmed by reading its module doc comment — no naming collision but no reusable code either | HIGH |
| `3c59c62e` | SEC-02b | 4 files, +262/-12 | `profile_runtime.rs` (present), `forward.rs` (absent), `oauth_capture/mod.rs` (absent), `oauth_capture/persist.rs` (absent) | **NO** (3/4 files target SEC-02a's not-yet-existing subsystem) | **rides with SEC-02a's disposition** — this is a security-hardening follow-up to `9b692e07`; cannot be absorbed independently | `git show --numstat 3c59c62e`: 3 of 4 touched files are inside the SEC-02a subsystem that does not exist pre-absorb | HIGH |
| `d033c631` | SEC-02c | 1 file, +4/-1 | `proxy_runtime.rs` (present) | Trivial — a stdin-consumption fix in a test fixture for the SEC-02a OAuth capture flow | **rides with SEC-02a's disposition** — the fixture it patches only exists if SEC-02a's provider stdin-handling code lands | `git show d033c631`: single-hunk change inside a test helper introduced by `9b692e07` | HIGH |
| `a3243907` | SEC-03 | 15 files, +611/-201 | `command_runtime.rs`, `exec_strategy.rs`, `exec_strategy/supervisor_linux.rs`, `execution_runtime.rs`, `launch_runtime.rs`, `main.rs`, `profile/mod.rs`, `profile_runtime.rs`, `proxy_runtime.rs`, `sandbox_prepare.rs`, `nono/lib.rs`, `nono/sandbox/linux.rs`, `nono/sandbox/mod.rs` — all present; `issues/sandbox-policy-split.md` (deleted by upstream, doc-only, absent from fork — non-blocking) | **YES** — 14/15 target files present | **adopt** | `crates/nono/src/sandbox/mod.rs` diff adds `apply_seccomp`/`apply_seccomp_with_abi` and reshapes `apply_external()`'s doc comment from "No-op: nono installs no Landlock or seccomp rules" to "TCP network enforcement is handled externally... filesystem/process sandboxing is applied separately" — this is mechanism reorganization (primitive), not new policy; existing GPU test surface at `crates/nono/src/sandbox/linux.rs:5059` (`test_is_nvidia_compute_device_accepts_upstream_list`) confirms the fork already carries a prior partial port ("D-13 upstream parity port") this commit extends, not creates from scratch | HIGH |
| `f943fb5a` | SEC-04 | 7 files, +182/-65 | `trust_cmd.rs`, `trust_scan.rs`, `nono/trust/{mod,policy,types}.rs`, `docs/cli/features/trust.mdx`, `tests/integration/test_trust_cli.sh` — all present | **YES** — 7/7 | **adopt** | `crates/nono/src/trust/types.rs` diff adds a `#[deprecated]` marker on `TRUST_POLICY_VERSION` plus a new `TRUST_POLICY_PREDICATE` const — additive discriminator field, no removal of existing fork-specific trust types; pure data/format-detection mechanism, no policy embedding | HIGH |
| `d84b4818` | SEC-05 | 1 file, +89/-0 | `nono/sandbox/linux.rs` — present | **YES**, and the specific gap is confirmed real | **adopt** | Fork's `restrict_execute()` (`crates/nono/src/sandbox/linux.rs:1105-1157`) is **byte-for-byte the pre-patch state** of upstream's function (matches even the double-space `"Tool Sandbox  execute restriction"` string) — it has **no** `.handle_access(AccessFs::Refer)` and **no** `if abi.has_refer() { ... add_rule for / ... }` block. This is a **separate, stacked Landlock ruleset** from the main `apply_landlock`/`access_to_landlock` ruleset (which already grants `Refer` under `AccessMode::Write` at `linux.rs:365` — confirmed via `grep -n Refer`), so the general grant does NOT cover this layer; Landlock requires each stacked layer to independently grant `Refer` or cross-directory renames break under the execute-restriction layer specifically. This resolves decision-critical question 2 with certainty: the general grant and this fix are genuinely non-overlapping. | HIGH |
| `ac5ccd70` | SEC-06 | 2 files, +180/-20 | `exec_strategy.rs` (present), `tests/socket_access_run.rs` (present) | **YES**, but the exact predicate upstream's diff references does not exist in the fork | **adapt** | Upstream's diff guards on `config.tool_sandbox_runtime.is_some() \|\| config.seccomp_policy.child_requires_dumpable()` — the fork has neither `tool_sandbox_runtime` (confirmed absent, D-01) nor a `seccomp_policy.child_requires_dumpable()` method; the fork's equivalent predicate is the free function `linux_child_requires_dumpable(capability_elevation, network_notify)` at `crates/nono-cli/src/exec_strategy.rs:445`, called at `:1524` around the existing `PR_SET_DUMPABLE(0)` logic. The underlying bug (Yama `ptrace_scope=1` reparenting a daemonizing descendant out of supervisor ancestry, breaking seccomp-notify `/proc/<pid>/mem` classification reads) is real and applicable — the fork's own AF_UNIX/network seccomp-notify mediation (Phase 87 SEC-01 memory) is exactly the mechanism this would protect. Absorb requires re-wiring the `prctl(PR_SET_CHILD_SUBREAPER)` call to `linux_child_requires_dumpable(config.capability_elevation, config.seccomp_proxy_fallback \|\| config.af_unix_mediation.is_pathname())` (dropping the `tool_sandbox_runtime` disjunct entirely) and porting `reap_reparented_orphans()` + its call site in `run_supervisor_loop` (fork has two `run_supervisor_loop` definitions at `:2897`/`:3111`, presumably cfg-split — plan must confirm which needs the reap-call). | HIGH |
| `2663e990` | SEC-07 | 15 files, +1,927/-140 | `app_runtime.rs`, `cli.rs`, `cli_bootstrap.rs`, `main.rs`, `proxy_command.rs` (new), `proxy_runtime.rs`, `nono-proxy/{config,connect,external,reverse,route,server,token}.rs` — all present except `proxy_command.rs`, which this commit itself creates | **YES** — 11/12 pre-existing, 1 new-file-creation (the point of the commit) | **adopt** | `crates/nono-cli/src/cli.rs` confirmed: `enum Commands` at line 646, zero `Proxy` variant anywhere in the file (grep for `Proxy` returns only doc-string/unrelated hits, not an enum arm); all 7 `nono-proxy` support files this commit modifies already exist. Zero touches to `crates/nono/src/` — stays entirely in `nono-cli`/`nono-proxy`, no ADR-86 boundary question. Confirmed sequencing dependency: upstream authored this **before** SEC-02a (`2026-07-04` vs `2026-07-05`), and SEC-02a's diff makes a 6-line edit to the `proxy_command.rs` this commit creates. | HIGH |
| `a5a441c2` | SEC-08 | 5 files, +124/-24 | `profile-authoring-guide.md`, `profile/mod.rs`, `profile_runtime.rs`, `environment.mdx`, `profile-authoring.mdx` — all present | **YES**, but adopting verbatim **relaxes an existing fork guard** | **won't-sync, OR adapt with an explicit fail-closed-preserving redesign — flag for ADR-112 escalation per D-05** | Fork's `profile_runtime.rs:840-870` already implements fail-closed semantics for empty `allow_vars`, landed in a prior phase ("Plan 34-08a Task 3/5 — D-20 manual replay of upstream `1b412a7`/`780965d7`"): whenever `profile.environment` is `Some(_)` at all, `allowed_env_vars` resolves to `Some(env_config.allow_vars.clone())` (never collapsed to `None`), so an `environment` block with `deny_vars` set but `allow_vars` omitted **already strips all inherited vars** (confirmed at `exec_strategy.rs:581`/`:796`, `if let Some(ref allowed) = config.allowed_env_vars`, activating the filter). Upstream's `a5a441c2` changes `EnvironmentConfig.allow_vars` from `Vec<String>` to `Option<Vec<String>>` specifically so that *omitting* `allow_vars` means "allow everything" (`None`) rather than "block everything" — the **opposite default** from what the fork already ships and documents as deliberate ("Profiles that set env_credentials but omit allow_vars would otherwise silently inherit every parent env var"). Adopting verbatim would reintroduce exactly the leak the fork's Phase-34 fix closed. | HIGH |
| `f6f02751` | SEC-09 | 3 files, +99/-26 | `execution_runtime.rs` (present), `profile/mod.rs` (present), `tests/execution_strategy_run.rs` (absent — new test file for the deleted guard) | Guard target **confirmed absent** | **won't-sync (target subsystem absent) — LOCKED per D-01, premise verified true** | `grep -rn "command_policies" crates/nono-cli/src/` → 0 hits; `grep -rn "tool_sandbox_runtime\|tool_sandbox_initial_shim" crates/nono-cli/src/` → 0 hits; `grep -n "env_credentials\|non-shim\|is_shim" crates/nono-cli/src/execution_runtime.rs` → 0 hits. D-01's evidence is independently reproduced, not merely trusted. | HIGH |
| `f0506434` | RES-01 | 24 files, +105/-102 | `.github/ISSUE_TEMPLATE/onboarding_issue.yml`, `README.md`, `nono-cli/README.md`, `policy.json`, `cli.rs`, `legacy_cleanup.rs` (absent), `main.rs`, `migration.rs` (absent), `pack_update_hint.rs`, `package_cmd.rs`, `package_status.rs`, `policy.rs`, `profile/builtin.rs`, `profile/mod.rs`, `profile_cmd.rs`, `profile_runtime.rs`, `profile_save_runtime.rs`, `pull_ui.rs` (absent), `query_ext.rs`, `setup.rs`, `wiring.rs`, `tests/deprecated_policy.rs`, `tests/profile_cli.rs`, `nono/trust/bundle.rs` | **PARTIAL** — 21/24 present, 3 absent (fork has no legacy-pack-cleanup migration UI) | **skip-biased default, but a real staleness finding exists — see evidence** | Top-level `README.md` and `setup.rs`/`test_env.rs`/`update_check.rs` **already** reference `nolabs-ai/nono` (a prior phase's own GitHub-org-URL update — this commit does NOT introduce that). But `crates/nono-cli/README.md`, `cli.rs` help-text examples, `policy.json:1164`'s alias map (`"always-further/claude": "claude-code"`), `policy.rs`, and `profile/builtin.rs` **still say `always-further/*`** for the *pack registry namespace* prefix (a different string than the GitHub org URL) — `grep -rn "always-further"` returns 15+ hits across these files. This is a genuine, provable staleness (help text tells users to run `nono pull always-further/claude`, which may not resolve against upstream's actual current `nolabs-ai/*` pack registry) but is a UX-consistency issue, not security-relevant, so D-03's skip bias still applies as the default; flag as a low-priority adapt candidate for the planner, not a security absorb. | MEDIUM |
| `7fe0c828` | RES-01 | 5 files, +81/-32 | `app_runtime.rs`, `migration.rs` (absent), `package_cmd.rs`, `profile/mod.rs`, `registry_client.rs` — all present except `migration.rs` | **PARTIAL** — 4/5 | **skip, with recorded reasoning** | Adds `X-Nono-Pull-Reason` header to registry pull requests — pure telemetry/diagnostics for upstream's own registry service; fork's registry/update-check behavior is already divergent (interacts with the fork's own `HKLM\SOFTWARE\Policies\nono` machine-policy spine, confirmed present via `grep -rln HKLM crates/nono-cli/src/*.rs` → 5 files), per D-03's stated rationale. No security or correctness gap identified. | MEDIUM |
| `0158d52f` | RES-01 | 2 files, +175/-2 | `registry_client.rs` (679 lines, present), `update_check.rs` (614 lines, present) | **YES** | **skip, with recorded reasoning** | Attaches installation-context headers to registry requests — additive telemetry to an upstream-service-facing client; no fork-side bug or security gap this closes. Matches D-03's rationale verbatim (registry/update-check collides with the fork's divergent HKLM-governed update behavior). | MEDIUM |
| `762eb05b` | RES-01 | 3 files, +87/-6 | `.github/workflows/release.yml`, `build.rs`, `update_check.rs` — all present | **YES** | **skip, with recorded reasoning** | Emits `install_source` on update-check requests — same class as `0158d52f`, pure telemetry; also touches `release.yml`, which the fork has already heavily customized for Trusted Signing (v3.5) — absorbing an upstream `release.yml` hunk verbatim carries out-of-proportion regression risk for a telemetry field. | MEDIUM |
| `503045801a` | RES-02 | 2 files, +242/-4 | `pty_proxy.rs`, `timeouts.rs` — both present | **YES** — genuinely new, additive mechanism | **adopt (real fix, not skip-biased away)** | Adds `discard_late_terminal_input()`/`CprReplyParse` to drain a late cursor-position-report reply (`ESC[<row>;<col>R`) left in the input queue when a TUI child exits mid-query, which otherwise gets pasted into the next shell prompt. Confirmed additive-only (new fn + one call site in the final-teardown path, explicitly NOT touching `restore_terminal()`'s non-draining behavior used by suspend/resume). Does not overlap the fork's existing `drain_terminal_output`/`drain_socket_replay`/`drain_attach_resize_pipe` mechanisms (different problem: those handle output flushing and IPC pipe draining, not a stray terminal *reply*). `pty_proxy.rs` is `#[cfg(unix)]`-gated in part, not purely `target_os = "linux"`/`"macos"`, so it falls outside the strict D-06 Linux-only list, but the planner should still run both cross-target clippy gates on it per the checklist's broader Unix-file scope. | MEDIUM-HIGH |
| `4cc0af2c` | RES-02 | 3 files, +10/-16 | `open_url_runtime.rs`, `tests/url_open_integration.rs` (absent), `nono/supervisor/socket.rs` — 2/3 present | **PARTIAL**, and applying it risks reintroducing a problem the fork's current code deliberately avoids | **adapt with caution, or skip** | Test-only: switches `socket_test_dir()` from a `target/`-relative tempdir to `/tmp` to stay under the Unix-domain-socket `SUN_LEN` path-length limit. But the fork's **current** `crates/nono/src/supervisor/socket.rs` comment (unchanged by this commit) explicitly documents choosing `target/` **specifically to avoid macOS Seatbelt denials when running tests inside a sandbox** (`deny network*` blocks Unix-socket connect on `/var/folders`, the macOS `TMPDIR` default) — switching unconditionally to `/tmp` could reintroduce that failure mode on macOS CI, which this project validates only via cross-target clippy + CI, not a live macOS host (established posture, per ROADMAP v3.6 Out of Scope). Needs either a macOS-conditional path choice or explicit confirmation that `/tmp` (unlike `/var/folders`) is Seatbelt-safe before absorbing. Test-only, so risk is CI-flake, not production security — but not a blind adopt. | MEDIUM |
| `9840a16f` | RES-02 | 1 file, +9/-9 | `tests/socket_access_run.rs` — present | **YES** | **adopt** | Test-quality fix only: suppresses the post-run denied-path review UI prompt in test runs (`NONO_NO_SAVE_PROMPT=1`) and tightens an assertion from a broad string-contains check to a specific denial-marker match. Zero production code touched, zero risk. | HIGH |

**Disposition tally: 5 clean adopt (SEC-03, SEC-04, SEC-05, SEC-07, RES-02/`9840a16f`) + 1 conditional/likely adopt (RES-02/`503045801a`) + 2 adapt (SEC-06, RES-02/`4cc0af2c` adapt-with-caution) + 2 won't-sync (SEC-01, SEC-09) + 1 won't-sync-or-careful-adapt with ADR-112 flag (SEC-08) + 3 rides-with-SEC-02a (SEC-02b, SEC-02c, and SEC-02a itself pending its own adapt-down-vs-defer decision) + 3 skip-with-reasoning (RES-01 × `7fe0c828`/`0158d52f`/`762eb05b`) + 1 partial-adopt-low-priority (RES-01/`f0506434`) = 18.**

## Architecture Patterns

### Recommended plan shape (informs D-04's collapse question)

Given the findings above, the three-heavyweight structure D-04 anticipated does not hold uniformly:

- **SEC-07**: keep as its own plan, own wave, exactly as D-04 specifies — this is real, self-contained absorb work.
- **SEC-01**: does NOT need its own absorb-plan wave. It needs a **finding-document plan** (mirroring `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md`'s shape and the SEC-09/D-01 carry-forward pattern) — small, not a 1,873-line-diff-sized wave.
- **SEC-02a/b/c**: needs a **reality-check task before plan-sizing** — the planner should decide adapt-down-scope (author a minimal profile-driven credential-provider path against `server.rs`, skip the `forward.rs`-dependent rewrite hook) vs. defer-to-a-future-phase (SPIFFE/#1272 precedent) as an explicit first task, then size the resulting plan(s) accordingly. This could still be "its own wave," but the wave's content is materially smaller than the raw 4,425-line diff suggests once the absent-subsystem 1,564 lines (oauth_capture core: 1,156 + tls_intercept: 195 + forward.rs: 194 − schema/docs counted separately) are subtracted.

**Net effect on D-04's "expect 8-10 plans, this is accepted, not a defect" framing:** the reality-check pass legitimately reduces total plan count versus the raw diffstat-based estimate, without contradicting D-04 (which explicitly anticipates and permits this collapse).

### System flow — SEC-07's proxy subcommand (representative of the one clean heavyweight)

```
CLI invocation (`nono proxy ...`)
        |
        v
crates/nono-cli/src/cli.rs         <- NEW: Commands::Proxy variant + subcommand args (currently absent)
        |
        v
crates/nono-cli/src/proxy_command.rs   <- NEW FILE (696 lines): standalone proxy lifecycle,
        |                                  independent of a sandboxed child process
        v
crates/nono-cli/src/proxy_runtime.rs   <- extended (110 lines) to support standalone-mode dispatch
        |
        v
crates/nono-proxy/src/{server,connect,external,reverse,route,token}.rs
        |                                  <- extended (existing, present) to serve requests
        v                                  without a supervised child on the other end
Network egress (subject to existing HostFilter/deny_domain/allow_domain policy)
```

### Anti-Patterns to Avoid

- **Treating "target file present" as "clean adopt."** Several SHAs (SEC-08 specifically) have every target file present in the fork yet still carry a real semantic conflict with existing fork code. File presence is necessary, not sufficient, evidence for `adopt`.
- **Absorbing SEC-01/SEC-02a as diff-apply exercises.** Both touch files/subsystems (`aws/`, `tls_intercept/`, `forward.rs`) that don't exist; a mechanical attempt to "make the diff apply" would require authoring an entire missing subsystem under the banner of a security-residual sync, which is out of this phase's declared scope (mirrors Phase 109's explicit refusal to build `aws/sign.rs`/`tls_intercept/handle.rs` from scratch for #1430/#1437).
- **Applying RES-02's `4cc0af2c` /tmp switch uncritically.** It is test-only and low production risk, but silently reintroducing a documented-and-deliberate macOS Seatbelt test-avoidance choice is exactly the kind of "silent regression via `#[allow]`-adjacent shortcut" CLAUDE.md warns against in spirit, even though this isn't a clippy allow.

## Don't Hand-Roll

Not applicable in the conventional library-selection sense — no new external-library decisions are in scope. The closest analogue: **do not hand-roll a partial AWS SigV4 signer or TLS-intercept dispatcher to force SEC-01/SEC-02a's diffs to apply.** If either subsystem is ever wanted, it needs its own future phase with its own design review and threat model — precisely the discipline `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` already established for the sibling commits to this same subsystem.

## Common Pitfalls

### Pitfall 1: Mistaking upstream's own predecessor-commit dependency for "this commit doesn't apply"
**What goes wrong:** SEC-02a's diff to `forward.rs` looks like a modify (not a new-file diff), so a shallow read might assume the file exists somewhere in the fork under a different name.
**Why it happens:** upstream's own commit history has an earlier commit (`149abde0`, outside this 18-commit list) that created `forward.rs`; the fork never absorbed that ancestor.
**How to avoid:** always run `git log --oneline --all --diff-filter=A -- <path>` for any "modified" file in an upstream diff that doesn't exist in the fork tree, to find the true ancestor commit and confirm it's out of scope.
**Warning signs:** a diff hunk with `--- a/<path>` / `+++ b/<path>` (not `/dev/null`) for a file `ls` shows absent from the fork.

### Pitfall 2: Trusting a commit's own bug-description framing over the fork's actual prior decisions
**What goes wrong:** SEC-08's commit message frames itself unambiguously as a bug fix ("empty allow_vars no longer strips all env vars"); a planner reading only the message would absorb it as an obvious correctness fix.
**Why it happens:** the message describes upstream's own prior bug, not the fork's current behavior — the fork already fixed an equivalent problem differently, in the opposite direction from what this commit assumes as the "correct" default.
**How to avoid:** for every SEC/RES commit touching security-adjacent defaults (env filtering, network filtering, path grants), read the fork's *current* implementation and its own doc comments/tests before accepting the upstream commit's self-description.
**Warning signs:** the target function/field already has non-trivial doc comments citing a prior phase number or "D-NN" decision reference — that's a signal the fork has its own considered position, not upstream's baseline.

### Pitfall 3: Cross-target clippy gate scope under-inclusion
**What goes wrong:** `.planning/templates/cross-target-verify-checklist.md`'s named scope list (files with `target_os = "linux"`/`"macos"` cfg blocks, `exec_strategy/` dir, `bindings/c/src/`) doesn't explicitly name `pty_proxy.rs` (which is `#[cfg(unix)]`-gated, not `target_os`-specific).
**Why it happens:** the checklist's literal scope list predates this phase's specific files.
**How to avoid:** treat "any file with platform-conditional compilation touching Unix-specific syscalls" as in-scope in spirit even if not literally named, per the checklist's own stated purpose.
**Warning signs:** `grep -n "cfg(unix)\|cfg(target_os" <file>` returns any hit.

## Code Examples

### The fork's existing fail-closed `allow_vars` resolution (SEC-08's real starting point — do not overwrite blindly)

```rust
// Source: crates/nono-cli/src/profile_runtime.rs:840-872 (live fork, pre-Phase-112)
// Plan 34-08a Task 3 (D-20 manual replay of upstream `1b412a7`): surface
// `profile.environment.allow_vars` as a runtime allow-list.
// Plan 34-08a Task 5 (D-20 replay of v0.52.0 `780965d7`): preserve
// fail-closed semantics for empty allow_vars. An empty `allow_vars`
// list returns `Some([])` (strip all inherited vars) rather than
// `None` (no filtering).
allowed_env_vars: loaded_profile.as_ref().and_then(|profile| {
    profile.environment.as_ref().map(|env_config| {
        // ... validate_env_var_patterns warning ...
        env_config.allow_vars.clone()
    })
}),
```

### SEC-05's exact gap in the execute-restriction layer (confirmed byte-for-byte pre-patch)

```rust
// Source: crates/nono/src/sandbox/linux.rs:1105-1128 (live fork) — matches
// upstream's PRE-`d84b4818` state exactly, including the ruleset's
// `.handle_access(AccessFs::Execute)` with NO `.handle_access(AccessFs::Refer)`,
// and no `if abi.has_refer() { ... }` block granting bare Refer on `/`.
pub fn restrict_execute(paths: &[impl AsRef<Path>]) -> Result<()> {
    let abi = detect_abi()?;
    // ...
    let mut ruleset = Ruleset::default()
        .set_compatibility(CompatLevel::HardRequirement)
        .handle_access(AccessFs::Execute)   // <- upstream adds .handle_access(AccessFs::Refer) here too
        // ...
}
```

## State of the Art

| Old Approach (fork, pre-112) | Current Approach (upstream, this window) | When Changed | Impact |
|--------------------------|------------------------------------------|---------------|--------|
| `restrict_execute()` grants only `Execute` in the stacked execute-restriction Landlock layer | Also grants bare `Refer` on `/` in that layer | `d84b4818`, 2026-07-10 | Cross-directory renames into freshly-created subdirs break under the execute-restriction layer without this; adopt |
| Fork's `EnvironmentConfig.allow_vars: Vec<String>` — empty/omitted both mean "strip all" (fork's own fail-closed default, Phase 34) | Upstream's `Option<Vec<String>>` — omitted means "allow all," explicit `[]` means "strip all" | `a5a441c2`, 2026-06-30 | Opposite default; adopting verbatim is a security regression for the fork specifically |
| Fork lacks a standalone `nono proxy` subcommand (proxy only runs attached to a sandboxed child) | Upstream adds `nono proxy` for standalone operation | `2663e990`, 2026-07-04 | New user-facing capability, no fork conflict |
| Fork has no `tls_intercept/` or `forward.rs` module (skipped an ancestor upstream refactor) | Upstream's OAuth-capture and AWS-SigV4 features both build on top of those modules | `149abde0` (ancestor, pre-window) + `0ecc476b`/`9b692e07` (this window) | Both SEC-01 and much of SEC-02a inherit this pre-existing architectural gap; not new to this sync |

**Deprecated/outdated:** `TRUST_POLICY_VERSION` (SEC-04) is marked `#[deprecated(since = "0.66.0")]` by upstream in favor of the new `TRUST_POLICY_PREDICATE` URI-based scheme; the fork should carry this deprecation marker forward if SEC-04 is adopted.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | SEC-02a's adapt-down path (author credential_provider/schema against `server.rs`, skip the `forward.rs`-dependent rewrite hook) is technically feasible without the `forward.rs` abstraction | Summary, Architecture Patterns | If false, SEC-02a should be a straight defer-to-future-phase (SPIFFE precedent) rather than an in-phase adapt; this was not verified by writing code, only by reading the diff's integration points |
| A2 | `/tmp` is Seatbelt-safe on macOS in a way `/var/folders`(the macOS `TMPDIR` default) is not, making RES-02's `4cc0af2c` a safe adapt rather than a real regression risk | Per-SHA table (`4cc0af2c` row), Common Pitfalls | This project validates macOS only via cross-target clippy + CI, never a live macOS host (established posture) — this claim is unverified against a live macOS Seatbelt profile in this research pass |
| A3 | The fork's two `run_supervisor_loop` definitions (`exec_strategy.rs:2897`/`:3111`) are cfg-split (e.g. with/without PTY, or Linux/other) such that SEC-06's `reap_reparented_orphans()` call site needs to be added to only the Linux-relevant one(s) | Per-SHA table (`ac5ccd70` row) | Not confirmed by reading both function bodies in full; if both are reachable on Linux, the orphan-reap call may need duplicating or refactoring into a shared point |

## Open Questions

1. **Should SEC-02a be adapted-down within Phase 112, or deferred to a dedicated future phase (mirroring NET-02/SPIFFE's move to Phase 113)?**
   - What we know: ~9 of 26 files depend on absent subsystems (`oauth_capture/` core, `tls_intercept/*`, `forward.rs`); ~17 files are present and could host a reduced-scope adapt.
   - What's unclear: whether a reduced-scope OAuth-capture feature (without the response-rewrite-via-`forward.rs` hook) is still a coherent, shippable feature, or whether the rewrite hook is load-bearing for the whole capture flow's security model (real tokens never reach the sandboxed client — if the rewrite hook is how that's enforced, a reduced version could ship an unsafe half-feature).
   - Recommendation: make this the very first D-02 reality-check task, with an explicit go/no-go before any absorb work is written for SEC-02a/b/c.

2. **Does the fork's `sha2` workspace dependency (already present in `nono-proxy/Cargo.toml`) get the SEC-01 won't-sync finding any closer to a future cheap implementation, or is a full SigV4 signer materially more work regardless?**
   - What we know: only `sha2` is present; no HMAC or AWS-canonical-request crate.
   - What's unclear: whether a minimal SigV4 implementation is small enough to be a future quick task rather than a full phase.
   - Recommendation: out of scope for Phase 112; note for whoever eventually revisits this (mirrors the `109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` "either would need its own future phase" framing).

## Environment Availability

Not applicable — this phase's dependencies (Rust toolchain, `cross`, `cargo-zigbuild`, Docker) are the same fork-invariant verification toolchain already confirmed local-runnable per `.planning/templates/cross-target-verify-checklist.md` and exercised successfully in Phase 111 (`111-04-VERIFICATION-NOTES.md`, both gates GREEN). No new external tool/service dependency is introduced by any of the 18 commits' recommended dispositions.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test runner (`cargo test`) + `cross test` for Linux-only-compiled modules |
| Config file | none — no dedicated test-framework config; workspace `Cargo.toml` |
| Quick run command | `cargo test -p nono-sandbox --lib <module>::` (native, for cross-platform code) or `cross test --target x86_64-unknown-linux-gnu -p nono-sandbox <test_name>` (for `#[cfg(target_os = "linux")]`-gated code) |
| Full suite command | `cargo test --workspace --no-fail-fast` (RED on this host for pre-existing, phase-unrelated reasons — see Phase 111 baseline below; diff against the documented baseline, don't expect a clean exit code) |

### D-06 verification contract (Linux-only enforcement items)

Per `112-CONTEXT.md` D-06 (LOCKED): SEC-03, SEC-05, SEC-06 are verified via **both** mandatory cross-target clippy gates **plus** `cross test --target x86_64-unknown-linux-gnu` on the affected modules — **no live-kernel UAT checkpoint**. Precedent: Phase 111-01 proved `max_crypto_threads_raised_to_12` live via `cross test` because `exec_strategy.rs` is `#[cfg(not(target_os = "windows"))]` and reports "0 tests" under a native Windows `cargo test` filter (a silent-pass trap the live cross-target run avoids). Cite `.planning/templates/cross-target-verify-checklist.md` for exact invocation syntax — do not duplicate the runbook here.

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SEC-03 | NVIDIA procfs mediation predicate | unit | `cross test --target x86_64-unknown-linux-gnu -p nono-sandbox -- test_is_nvidia_compute_device_accepts_upstream_list` | ✅ (`crates/nono/src/sandbox/linux.rs:5059`, pre-existing "D-13 upstream parity port" test to extend, not create) |
| SEC-03 | `--allow-gpu` capability wiring | unit | `cargo test -p nono-sandbox-cli --lib -- test_from_args_allow_gpu_sets_capability_on_unix test_from_args_windows_sandbox_state_invariant_with_vs_without_allow_gpu` | ✅ (`crates/nono-cli/src/capability_ext.rs:1468`/`:1512`) |
| SEC-05 | Landlock `Refer` in execute-restriction layer | unit (regression) | `cross test --target x86_64-unknown-linux-gnu -p nono-sandbox -- test_restrict_execute_does_not_break_rename_into_new_subdir` | ❌ Wave 0 — this exact test is what `d84b4818` itself adds; port it verbatim as part of the absorb, then run it via `cross test` |
| SEC-06 | Orphan-reaping under seccomp-notify mediation | unit | `cross test --target x86_64-unknown-linux-gnu -p nono-sandbox-cli -- test_linux_child_requires_dumpable_only_for_seccomp_driven_features` (existing predicate test to extend) plus any new orphan-reap unit test the plan authors | ✅ predicate test exists (`crates/nono-cli/src/exec_strategy.rs:4452`); orphan-reap itself needs a new Wave-0 test |
| SEC-07 | `nono proxy` subcommand dispatch | integration | `cargo test -p nono-sandbox-cli --bin nono -- proxy` (new tests, name TBD by plan) | ❌ Wave 0 — no existing `Proxy` command tests since the variant doesn't exist yet |
| SEC-08 | `allow_vars` fail-closed semantics preserved (regression guard) | unit | `cargo test -p nono-sandbox-cli --lib -- empty_allow_vars_fails_closed` | ✅ (`crates/nono-cli/src/profile_runtime.rs:989`) — this is the exact test that would catch a verbatim-adopt regression; run it BEFORE and AFTER any SEC-08 change as the primary guard |
| SEC-09 | Carry-forward note only (no code) | manual-only | N/A — documentation deliverable, not testable | N/A |
| RES-02 (`503045801a`) | Late CPR-reply teardown drain | unit (new) | Plan must author a new unit test around `discard_late_terminal_input()`/`CprReplyParse` | ❌ Wave 0 |
| RES-02 (`9840a16f`) | Denial-marker test assertion tightening | integration | `cargo test -p nono-sandbox-cli --test socket_access_run -- af_unix_mediation_pathname_allows_connect_to_listed_socket` | ✅ |

### Sampling Rate

- **Per task commit:** targeted `cargo test`/`cross test` filter for the touched module (see table above)
- **Per wave merge:** `make ci` (clippy + fmt + tests) plus both cross-target clippy gates for any wave touching `crates/nono/src/sandbox/linux.rs`, `crates/nono-cli/src/exec_strategy.rs`, or `crates/nono-cli/src/exec_strategy/`
- **Phase gate:** full `cargo test --workspace --no-fail-fast` diffed against the Phase 111 baseline (below) before `/gsd:verify-work`

### Phase 111 workspace-test baseline (inherited, do not mis-read as a new regression)

Per `111-04-VERIFICATION-NOTES.md`: `cargo test --workspace --no-fail-fast` exits 101 (not 0) on this host for pre-existing, phase-unrelated reasons. The **documented 24-name failing-test baseline** (11 `nono` --bin names + 13 across `audit_attestation.rs`/`env_vars.rs`/`resl_nix_async_signal_safety.rs`) is host-state/flake-driven (mandatory-label ACE contamination, hardcoded Unix path literals in Windows-run tests, a stale signature-match panic message) — **not a code regression**, confirmed by `git log --oneline --all -- <file>` returning no Phase 108-111 SHA for any of the affected files. A documented orchestrator addendum records **3 additional** `profile_cmd.rs` names (`test_init_creates_valid_profile`, `test_init_rejects_existing_file_without_force`, `test_schema_output_to_file`) as a **shared-fixture flake** (fixed `%TEMP%\nono-test-profile-init` path racing between parallel test-runner processes, not `tempfile::TempDir` — passes 10/10 in isolation), also confirmed unrelated to Phase 108-111 changes. **Phase 112's baseline is Phase 111's 24-name set plus this documented 3-name addendum** — any NEW name outside this 27-total set is a real regression signal; any name already in this list reappearing is expected host noise, not a new finding.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V2 Authentication | yes (SEC-01/SEC-02 scope) | N/A pending won't-sync/defer disposition for SEC-01; OAuth token capture (SEC-02a) if adapted must ensure real tokens never cross the sandbox boundary — this is the exact property the missing `forward.rs` rewrite hook currently enforces upstream (see Open Question 1) |
| V4 Access Control | yes (SEC-05, SEC-06, SEC-08) | Landlock ruleset construction (library primitive, SEC-05); seccomp-notify supervisor-ancestry (CLI process supervision, SEC-06); environment-variable allow-list resolution (CLI profile policy, SEC-08) |
| V5 Input Validation | yes (SEC-04) | Trust-policy predicate-field discriminator — prevents a full-parse crash on foreign `trust-policy.json` files; pure format validation, no policy embedding |
| V6 Cryptography | yes (SEC-01, if ever revisited) | AWS SigV4 (HMAC-SHA256 canonical-request signing) — explicitly NOT implemented by this phase per the won't-sync recommendation; if ever built, must use a vetted signing crate, never hand-rolled HMAC |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|----------------------|
| Reparenting a descendant out of supervisor ancestry to bypass ptrace-gated seccomp-notify mediation (SEC-06) | Elevation of Privilege | `PR_SET_CHILD_SUBREAPER` + orphan-reap, adapted to the fork's `linux_child_requires_dumpable` predicate (per the SEC-06 table row) |
| Silent default-widening of an environment-variable allow-list when a field's absence is reinterpreted (SEC-08) | Information Disclosure | Preserve the fork's existing fail-closed `Some(vec![])`-on-empty-environment-block default; do not adopt upstream's `Option`-based reinterpretation verbatim |
| Cross-directory rename bypass under a stacked Landlock execute-restriction layer missing `Refer` (SEC-05) | Tampering | Grant bare `Refer` on `/` in the execute-restriction layer specifically, matching upstream's fix — CLAUDE.md's "Landlock is strictly allow-list, cannot express deny-within-allow" constraint is satisfied because this is an *additional* grant in a *stacking* layer, not a deny-within-allow expression |
| Real OAuth tokens reaching the sandboxed client if a capture-and-rewrite feature is adapted without its rewrite-enforcement hook (SEC-02a, if adapted) | Information Disclosure | Do not ship a reduced SEC-02a that drops the `forward.rs`-style response-rewrite hook without an equivalent enforcement point; resolve Open Question 1 before implementation |

## Sources

### Primary (HIGH confidence — live repository inspection, 2026-08-05)
- `git show`/`git show --stat --numstat` for all 18 SHAs (`0ecc476b`, `9b692e07`, `3c59c62e`, `d033c631`, `a3243907`, `f943fb5a`, `d84b4818`, `ac5ccd70`, `2663e990`, `a5a441c2`, `f6f02751`, `f0506434`, `7fe0c828`, `0158d52f`, `762eb05b`, `503045801a`, `4cc0af2c`, `9840a16f`) — no network fetch required, all confirmed via `git cat-file -t`
- `Read`/`Grep`/`Bash ls` against the live fork tree at `C:\Users\OMack\nono` for every named target file (aws/, tls_intercept/, oauth_capture/, restrict_execute(), profile_runtime.rs's allow_vars resolution, exec_strategy.rs's linux_child_requires_dumpable, cli.rs's Commands enum, HKLM references, Cargo.lock's crossbeam-epoch pin)
- `.planning/phases/108-upst12-divergence-audit/108-DIVERGENCE-LEDGER.md` — Cluster Summary, security-residual-and-misc per-commit table (line ~626), Requirement Coverage Gap (D-18/D-19, line ~63), Phase 111 Standing Divergence Addendum (line ~1648), "Ledger closed" declaration (line ~1642)
- `.planning/phases/109-proxy-network-absorb/109-AWS-SIGV4-TLS-INTERCEPT-FINDING.md` — precedent finding for the same absent `aws/`/`tls_intercept/` subsystem
- `.planning/phases/111-core-carry-resource-cli-verify-release-leapfrog/111-04-VERIFICATION-NOTES.md` — 24-name workspace-test baseline, `cross test` precedent
- `proj/ADR-86-library-boundary-convergence.md`, `proj/ADR-111-resource-limits-boundary.md`, `proj/ADR-108-deny-domain-posture.md`
- `.planning/templates/cross-target-verify-checklist.md`
- `.planning/phases/112-security-residual-sync/112-CONTEXT.md`, `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md` (Phase 112 section), `.planning/STATE.md`
- `./CLAUDE.md`

### Secondary / Tertiary
None — every claim in this document is either a direct `git show`/repository-read citation (HIGH) or explicitly logged in the Assumptions Log (A1-A3) as unverified in this pass.

## Metadata

**Confidence breakdown:**
- Per-SHA dispositions (SEC-01, SEC-03 through SEC-09, RES-02): HIGH — every disposition backed by a direct `git show`/`grep`/`Read` citation against the live tree, not inference from commit subjects
- SEC-02a/b/c adapt-scope: HIGH on the entanglement finding, MEDIUM on the specific recommended scope (adapt-down vs. defer) — genuinely a judgment call flagged as Open Question 1
- RES-01 dispositions: MEDIUM — skip-bias rationale confirmed (HKLM spine exists), but the `f0506434` partial-staleness finding is a new observation not previously flagged in CONTEXT.md/the ledger, worth a second look
- Validation Architecture: HIGH for D-06-covered items (SEC-03/05/06), MEDIUM for RES-02 (no existing test scaffolding to extend, Wave 0 gap)

**Research date:** 2026-08-05
**Valid until:** until the fork's HEAD moves materially past the current tree state referenced above (this is a live-tree diff-verification pass, not a stable external-library survey — re-verify any finding if significant unrelated commits land on the target branch before Phase 112 executes)
