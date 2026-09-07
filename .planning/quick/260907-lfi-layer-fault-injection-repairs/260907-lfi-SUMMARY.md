---
quick_id: 260907-lfi
slug: layer-fault-injection-repairs
date: 2026-09-07
status: complete
commits:
  - 19c8ccc8  # protected_paths pre-resolved roots
  - 48be5c99  # hook CLR-regression timeout
milestone: v3.7
covers_triage_items: [5]
unblocks_jobs:
  - Windows Layer Fault Injection
---

# Quick Task 260907-lfi — Summary

**Status:** complete. Item 5 of the CI run `34068950597` triage.

`windows-layer-fault-injection` reported `1746 passed; 5 failed`. All five are now
addressed — `receipt_sink` under `260906-q7n`, the other four here.

## `protected_paths` x3 (`19c8ccc8`) — a guard whose positive tests never ran

**Correcting my own first read:** I framed these from the CI log as
elevation-specific. They are not. Run locally they fail identically —
**3 passed / 3 failed, the same three names**. The local reproduction is what
turned this from speculation into a fix.

`validate_requested_path_against_protected_roots` resolves the REQUESTED path
through `resolve_path`, which strips the Windows `\?\` verbatim prefix, then
compares against each `protected_root` **as given** — the roots are
contractually pre-resolved (`ProtectedRoots::from_defaults` → `resolve_path`,
the upstream `be384ee4` perf note at `:83-86`).

The tests passed a raw `tmp.path().canonicalize()`, and on Windows canonicalize
**returns** a `\?\` path. The two sides disagreed at the first component
(`\?\C:` vs `C:`), `path_starts_with` reported no overlap, and the guard
returned `Ok` — **failing open**. So the three positive tests had never verified
blocking on Windows at all.

**Production is unaffected:** its roots go through `resolve_path`, so both sides
are normalized. This is a test-side contract violation.

Verified: 6/6 pass (was 3/6) — changing exactly one thing, normalizing the root,
made all three failures vanish, which is the causal confirmation. **Perturbation:**
forcing `inside_protected`/`parent_of_protected` to `false` makes exactly those
three FAIL, so they now catch a real guard regression rather than being dead
weight. Reverted, re-confirmed 6/6.

## Hook CLR-regression timeout (`48be5c99`)

`Before-hook timed out after 30s (fail-closed)` on CI; the same test completes in
**~0.7s** locally. A cold PowerShell start (CLR load + module init) on a loaded
runner exceeded the bound.

The 30s is a **test fixture** value, not the production hook timeout — verified
before touching it (both occurrences are inside `#[cfg(test)] mod tests`;
production reads `timeout_secs` from the profile), so widening cannot loosen any
shipped fail-closed behaviour.

Widened to 120s because 30s was not measuring the thing under test. The subject
is *"does the CLR fail to start"* — a FAILED launch, not a slow one. A bound
tight enough to fire on a slow-but-successful start makes the test **report the
opposite of the truth**: it claimed "timed out, fail-closed" on a host where the
CLR was loading fine. The only cost of the wider bound is that a genuine hang
takes longer to surface.

The test's own diagnostic enumerates two known modes (`-65536` = `env_clear`
stripped `SystemRoot`; `code 1`/`not digitally signed` = unstripped `\?\`). A
timeout is a third it did not anticipate, which is why CI's output named neither.

## Follow-up recorded, not taken

The pre-resolved-roots contract is a **caller obligation with no type-level
enforcement**, and violating it makes a security guard fail open silently.
Normalizing inside the loop would close that, at a small cost against the
upstream perf note. Flagged for the operator rather than changed unilaterally —
it is a production-semantics change to a security guard.

## Finding worth carrying

**A positive security test can be red for years while looking like an
environment problem.** These three asserted "this MUST be blocked", failed on
every Windows host, and were plausibly filed under the known Windows baseline —
so the signal that the guard's blocking path was untested on Windows never got
read as such. A failing *positive* assertion in a security guard deserves a
different triage default than a failing environment-sensitive test.
