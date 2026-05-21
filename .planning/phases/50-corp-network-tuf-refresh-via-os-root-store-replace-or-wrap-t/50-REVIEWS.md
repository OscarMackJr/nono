---
phase: 50
reviewers: [codex]
reviewed_at: 2026-05-21T23:30:57Z
plans_reviewed:
  - 50-01-PLAN.md
  - 50-02-PLAN.md
  - 50-03-PLAN.md
  - 50-04-PLAN.md
  - 50-05-PLAN.md
runtime: claude-code-cli (self-reviewer skipped)
unavailable_reviewers:
  - gemini (GEMINI_API_KEY not set in user environment)
---

# Cross-AI Plan Review — Phase 50

> Two external CLIs were attempted (`gemini`, `codex`). Gemini failed authentication (no `GEMINI_API_KEY`). Codex returned a substantive review with **4 HIGH-severity findings the gsd-plan-checker missed**.

## Gemini Review

Gemini review failed: `GEMINI_API_KEY` environment variable not set on this machine. Skipped.

To enable Gemini reviews, set `GEMINI_API_KEY` in the user environment and re-run `/gsd-review --phase 50 --gemini`.

---

## Codex Review

**Model:** gpt-5.4 (Codex v0.124.0)
**Tokens used:** 108,355
**Workaround applied:** invoked with `-c 'hooks=[]'` to bypass an unrelated config.toml schema error in `~/.codex/config.toml` (the `[[hooks]]` entry triggers an "invalid type: map, expected a sequence" load error).

### Summary

The Phase 50 plan set is thoughtful and unusually well-researched, especially around the `tough` async correction and the requirement to keep TUF verification inside upstream code. But it still has several plan-quality gaps that could cause execution churn or, worse, give false confidence that SPEC-50-REQ-1..6 were met when they were only partially proven. The biggest issues are dependency/interface mistakes in the proposed implementation surface, an acceptance check that is currently impossible to satisfy as written, a weakened byte-identity proof, and a contradiction between the "HARD pass" cross-target policy and the actual fallback path in Plan 50-05.

### Strengths

- The plan correctly identifies the central technical correction: `tough` is async, so the `tokio` runtime in `setup.rs` must stay.
- The security boundary is mostly sound: the intended design keeps TUF signature math inside `tough` and avoids hand-rolled verification.
- The call-site swap in `50-03-PLAN.md` is appropriately minimal, which lowers regression risk for the cache contract.
- The plan explicitly preserves `serde_json::to_string_pretty(&trusted_root)` and `std::fs::write(&cache_path, &json)`, which is the right instinct for preserving the offline cache reader contract.
- The test strategy is hermetic and avoids socket-based flake by preferring an in-memory transport seam.
- Keeping Phase 49's `--from-file` path as a backstop is the right operational posture.

### Concerns

- `severity: HIGH` The implementation plans use transitive crates as if they were direct deps. `50-02-PLAN.md` imports `sigstore_trust_root::{DEFAULT_TUF_URL, PRODUCTION_TUF_ROOT, TRUSTED_ROOT_TARGET}` and `async_trait`, `bytes`, `futures`, but `50-01-PLAN.md` only promotes `tough`. Rust will not let `nono-cli` import `sigstore_trust_root` unless it is also a direct dependency. This is a plan bug, not a coding nit.
- `severity: HIGH` The Req 1 acceptance grep is currently self-defeating. `50-03-PLAN.md` requires `grep -rn 'TrustedRoot::production()' crates/nono-cli/src/` to return zero, but `50-02-PLAN.md` explicitly adds comments/docs in `trust_refresh.rs` containing that exact string. The executor can satisfy the code change and still fail the gate forever.
- `severity: HIGH` The byte-identity proof in `50-04-PLAN.md` is weaker than the spec. The proposed `cache_bytes_match_baseline` test compares against `TrustedRoot::from_json(fixture_bytes)` reserialized with `serde_json::to_string_pretty`, not against output captured from the upstream `TrustedRoot::production()` path against the same hermetic repo. That proves serde round-trip determinism, not equivalence to the upstream behavior.
- `severity: HIGH` `50-05-PLAN.md` contradicts D-50-13. The phase context says cross-target clippy is a hard requirement, but Task 3 still allows Outcome B deferral to CI/checklist. That is not a hard pass. Either the policy or the plan is wrong.
- `severity: MEDIUM` The production cleanup semantics do not match the stated intent. `50-02-PLAN.md` only removes `tuf-cache` if `RepositoryLoader::load()` fails. If `read_target`, UTF-8 decode, or `TrustedRoot::from_json` fails, partial datastore state remains even though D-50-07 says cleanup should occur on any failure after directory creation.
- `severity: MEDIUM` The plan proves trust-store selection more than proxy-path correctness. `ureq + platform-verifier` may fix CA validation, but the phase goal is success behind a corporate proxy. If the environment depends on system proxy discovery/auth rather than transparent interception, the current plan may still fail even with correct root-store use. The HUMAN-UAT catches this late, but the plans do not surface it as an explicit residual risk.
- `severity: MEDIUM` The hermetic tests exercise the extracted helper, not the full public wrapper. That means URL composition, production agent construction, and production datastore resolution are mostly validated by grep/compile, not behavior tests.
- `severity: MEDIUM` `50-04-PLAN.md` inserts an interactive fixture-decision checkpoint into an execution plan. That is a process smell: either decide the fixture strategy in planning, or make the fallback automatic without blocking human input.
- `severity: MEDIUM` The fixture naming/story is inconsistent. Several tests are named as `v14 -> v15`, but the proposed checked-in fixtures appear to use `1.root.json` style local test metadata. That is fine technically, but the naming can mislead reviewers into thinking the tests mirror production root versions when they do not.
- `severity: LOW` Mapping HTTP `403` to `FileNotFound` is faithful to `tough`'s HTTP transport behavior, but it is worth calling out that a corp proxy returning `403` for policy reasons could be normalized into a TUF "not found" path. That may make some failures less diagnosable.

### Suggestions

- In `50-01-PLAN.md` or `50-02-PLAN.md`, either add direct dependencies on `sigstore-trust-root` and any macro/helper crates actually used, or simplify the surface:
  - Prefer `nono::trust::TrustedRoot` for the return type.
  - Add `sigstore-trust-root = "0.7.0"` explicitly if you need `PRODUCTION_TUF_ROOT`, `DEFAULT_TUF_URL`, and `TRUSTED_ROOT_TARGET`.
- Fix the Req 1 gate in `50-03-PLAN.md`. Don't grep the whole source tree for `TrustedRoot::production()`. Restrict the check to executable code, or just assert that `crates/nono-cli/src/setup.rs` no longer calls it.
- Strengthen the Req 4 proof in `50-04-PLAN.md`:
  - Add a direct comparison against a captured upstream baseline generated from the same hermetic repo.
  - Add a separate test that writes the produced bytes to a temp file and calls `TrustedRoot::from_file` on that exact file.
- Tighten `50-02-PLAN.md` cleanup so every failure after datastore creation goes through one cleanup path, not just `.load()` failures.
- Resolve the cross-target policy contradiction in `50-05-PLAN.md`. If D-50-13 is truly hard, Outcome B cannot be considered acceptable for phase close.
- Make proxy behavior an explicit residual-risk line in `50-HUMAN-UAT.md`: distinguish "transparent TLS interception" from "explicit proxy discovery/auth required." Right now the phase may be over-claiming what is fixed.
- Collapse the fixture decision now. My recommendation: use checked-in fixtures plus a regeneration script committed to the repo. That is more stable than a human checkpoint and less API-fragile than `tough::editor` in test code.
- Add one small behavior test around the public wrapper path in `crates/nono-cli/src/trust_refresh.rs` so the production composition is not validated only by grep.

### Risk Assessment

**Overall risk: MEDIUM-HIGH**

The design direction is good, and the phase probably can succeed with modest corrections. But in its current form the plans contain multiple execution-level defects: missing direct deps, an impossible acceptance grep, a softened proof for the byte-identity requirement, and a hard-policy contradiction on cross-target verification. Those are not theoretical risks; they are the kind that derail implementation or let a phase close with weaker evidence than the spec demands.

---

## Consensus Summary

> Only one external reviewer returned (Codex). The "consensus" below is a synthesis of Codex's findings against the gsd-plan-checker's earlier `VERIFICATION PASSED` verdict — i.e., where the external reviewer disagrees with the internal checker.

### Agreed Strengths

- **Async/sync correction is properly encoded.** Both Codex and the gsd-plan-checker agree the tokio runtime must stay and `spawn_blocking` is the right bridge.
- **Cache-write preservation is correct** (verbatim `serde_json::to_string_pretty` + `std::fs::write`).
- **Hermetic test strategy via in-memory Transport seam is sound.**
- **Phase 49 `--from-file` operational backstop posture is right.**

### Agreed Concerns (= external reviewer flagged, internal checker missed or rationalized)

| ID | Severity | Title | Where | Internal-checker status |
|----|----------|-------|-------|--------------------------|
| **R-50-01** | HIGH | Transitive crate imports used as if direct deps | 50-01-PLAN.md (deps) + 50-02-PLAN.md (imports). Imports `sigstore_trust_root::*` + `async_trait` + `bytes` + `futures` but only `tough` is added to Cargo.toml. | **Missed.** Plan-checker's dependency check did not catch the missing direct-dep promotion for `sigstore-trust-root` / `async-trait` / `bytes` / `futures`. |
| **R-50-02** | HIGH | Self-defeating Req 1 acceptance grep | 50-03-PLAN.md acceptance `grep -rn 'TrustedRoot::production()' crates/nono-cli/src/` returns 0, but 50-02-PLAN.md adds doc-comments referencing that string. | **Missed.** Plan-checker accepted the grep as valid; didn't notice the cross-plan conflict between comment text and the grep pattern. |
| **R-50-03** | HIGH | Weak byte-identity proof | 50-04-PLAN.md `cache_bytes_match_baseline` compares fixture-round-tripped bytes, not upstream-captured baseline. Req 4 says "byte-identical to `TrustedRoot::production()` output", not "serde round-trips deterministically". | **Missed.** Plan-checker treated the snapshot test as adequate Req 4 proof. |
| **R-50-04** | HIGH | Cross-target clippy policy contradiction | D-50-13 says HARD pass; 50-05-PLAN.md Task 3 still permits "Outcome B" (deferral to CI/checklist). | **Rationalized.** Plan-checker accepted Outcome B as "structurally preventing close until CI confirms — acceptable per cross-target-verify-checklist.md escape hatch". Codex disagrees: HARD means no deferral. |
| **R-50-05** | MEDIUM | Cleanup semantics narrower than D-50-07 | 50-02-PLAN.md only removes `tuf-cache` on `RepositoryLoader::load()` failure; not on subsequent `read_target` / UTF-8 / `from_json` failures. D-50-07 says "any failure after directory creation". | **Missed.** Plan-checker noted cleanup is present but did not audit failure-path coverage. |
| **R-50-06** | MEDIUM | Trust-store success ≠ proxy-path success | Phase goal is "succeed behind corp proxy". `platform-verifier` fixes CA validation but not proxy discovery/auth. HUMAN-UAT catches this; plans don't surface as residual risk. | **Missed.** Plan-checker focused on TLS trust-store; did not interrogate proxy discovery/PAC/auth path. |
| **R-50-07** | MEDIUM | Behavior tests cover helper, not public wrapper | URL composition + production agent + production datastore resolution validated only by grep/compile, not behavior. | **Missed.** Plan-checker accepted the test-via-injection-seam pattern at face value. |
| **R-50-08** | MEDIUM | Interactive checkpoint in execution plan is a process smell | 50-04 Task 2 `checkpoint:decision` for fixture strategy. Either decide now in planning, or make fallback automatic. | **Partial.** Plan-checker noted the checkpoint as "well-justified"; Codex disagrees and recommends collapsing the decision before execute-phase. |
| **R-50-09** | MEDIUM | Fixture naming inconsistency (`v14 -> v15` vs `1.root.json`) | Test names imply production semantics; fixture files don't mirror them. | Flagged as W3 by plan-checker (cosmetic warning). Codex aligns. |
| **R-50-10** | LOW | 403 → FileNotFound diagnostic obscurity | tough's HTTP transport normalizes corp-proxy 403 (policy-deny) into TUF "not found", which may misdirect debugging. | **Missed.** Plan-checker did not interrogate HTTP status-code mapping. |

### Divergent Views

- **Internal plan-checker:** `VERIFICATION PASSED` with 3 non-blocking warnings (W1/W2/W3 — all cosmetic).
- **External Codex:** `MEDIUM-HIGH risk` with 4 HIGH + 5 MEDIUM + 1 LOW findings — including 4 issues the internal checker missed entirely and 1 (cross-target) it explicitly rationalized.
- **Recommendation:** Treat Codex's 4 HIGH findings (R-50-01 through R-50-04) as blockers for execute-phase. R-50-05 through R-50-10 are addressable in the replan or deferrable to executor judgment.

### Next Action

Run `/gsd-plan-phase 50 --reviews` to incorporate this feedback into a revised plan set. Expected fixes:

1. **50-01-PLAN.md** — add direct deps `sigstore-trust-root`, `async-trait` (if used), `bytes` (if used). Verify against actual import list in 50-02 task actions.
2. **50-02-PLAN.md** — broaden cleanup to ALL post-datastore-creation failures (single cleanup path via `let _ = || { ... }()` or a guard struct).
3. **50-03-PLAN.md** — scope the Req 1 grep to executable code only (`crates/nono-cli/src/setup.rs` specifically, or use a stricter pattern that excludes doc-comments / strings).
4. **50-04-PLAN.md** — strengthen Req 4 proof: add a baseline-captured snapshot test (one-time generation from real `TrustedRoot::production()` against the hermetic repo, then assert byte-identical match). Collapse the fixture checkpoint to a planning-time decision (recommended: checked-in fixtures + regen script per Codex). Add at least one behavior test of the public `refresh_production_trusted_root()` wrapper, not just the helper.
5. **50-05-PLAN.md** — resolve D-50-13 contradiction: either remove Outcome B entirely (HARD = HARD), or relax D-50-13 to PARTIAL-allowed with explicit user-acknowledged sign-off. Add proxy-path residual-risk note to HUMAN-UAT (transparent interception vs proxy-discovery/auth).
