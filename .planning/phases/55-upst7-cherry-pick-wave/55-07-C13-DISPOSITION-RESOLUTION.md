# C13 Disposition Resolution: sigstore 0.8.0 bump (scrub.rs diff-inspection)

**Produced:** 2026-06-05  
**Phase:** 55-upst7-cherry-pick-wave  
**Plan:** 55-07-SIGSTORE-BUMP  
**Upstream commit:** e581569659d35b50bb64d8086fc240d715f5e049  
**Upstream tag:** v0.58.0  
**Upstream subject:** chore(deps): update sigstore crates to 0.8.0  
**D-55-02 mandate:** Diff-inspection-first before any code lands; compare upstream scrub.rs change against fork's Phase-49 trust-root surface and D-32-15 verify-is-offline invariant.

---

## 1. Upstream scrub.rs Diff (verbatim from `git show e581569`)

```diff
diff --git a/crates/nono/src/scrub.rs b/crates/nono/src/scrub.rs
index 69cddc93..3430445b 100644
--- a/crates/nono/src/scrub.rs
+++ b/crates/nono/src/scrub.rs
@@ -181,7 +181,7 @@ pub fn scrub_value_with_policy<'a>(s: &'a str, policy: &ScrubPolicy) -> Cow<'a,
     let url_scrubbed = scrub_url_userinfo(header_scrubbed.as_ref());
     let query_scrubbed = scrub_query_params(url_scrubbed.as_ref(), policy);
 
-    if query_scrubbed.as_ref() == s {
+    if &*query_scrubbed == s {
         Cow::Borrowed(s)
     } else {
         Cow::Owned(query_scrubbed.into_owned())
@@ -271,7 +271,7 @@ pub fn scrub_header_with_policy<'a>(
 fn scrub_header_arg<'a>(value: &'a str, policy: &ScrubPolicy) -> Cow<'a, str> {
     if let Some((name, header_value)) = value.split_once(':') {
         let scrubbed = scrub_header_with_policy(name.trim(), header_value.trim_start(), policy);
-        if scrubbed.as_ref() != header_value.trim_start() {
+        if &*scrubbed != header_value.trim_start() {
             return Cow::Owned(format!("{}: {}", name, scrubbed));
         }
     }
```

**Change summary:** 2 lines changed, 2 added. Both changes convert `Cow::as_ref()` deref idiom to
the `&*cow` deref idiom. These are semantically identical in Rust: `Cow<'_, str>` implements `Deref<Target = str>`, so both `cow.as_ref()` and `&*cow` produce a `&str`. This is a Rust style/clippy-driven normalization, not a behavioral change.

---

## 2. Fork's Phase-49 Trust-Root Surface

Phase 49 (Plan 49-01) added the `--from-file` flag for `nono setup`, with these trust-root surface files:
- `crates/nono-cli/src/setup.rs` — `from_file` branch; reads `--from-file <PATH>` and calls
  `nono::trust::bundle::load_trusted_root` + `check_trusted_root_freshness` + `std::fs::copy`
  to populate `<nono_home>/.nono/trust-root/trusted_root.json`
- `crates/nono/src/trust/bundle.rs` — `load_trusted_root`, `load_production_trusted_root`,
  `check_trusted_root_freshness` — the verify path (UNCHANGED by Phase 49)

**The Phase-49 changes are entirely in `crates/nono-cli/src/setup.rs` and `crates/nono/src/trust/bundle.rs`.**

`crates/nono/src/scrub.rs` is the diagnostics/audit redaction module. It is **not part of the
Phase-49 trust-root surface** — Phase 49 explicitly excluded touching `crates/nono` beyond the
trust bundle (`49-SPEC.md` out-of-scope: "Touching the verify path in `crates/nono`").

---

## 3. D-32-15 Verify-is-Offline Invariant Description

D-32-15 (from Phase 32 architecture): `nono trust verify` reads `trusted_root.json` from the
cache path via `nono::trust::bundle::load_production_trusted_root()` which calls
`TrustedRoot::from_file` — plain JSON deserialization, NOT a live TUF re-verification fetch.
There is no network call on the verify path.

The D-32-15 invariant is enforced at `crates/nono/src/trust/bundle.rs`. The upstream `e581569`
change touches `crates/nono/src/scrub.rs` — a completely separate module used only for
diagnostic/audit record redaction. These modules have no dependency relationship.

---

## 4. Per-Line Collision Analysis

| Line | Change | Touches Phase-49 surface? | Touches D-32-15 path? | Verdict |
|------|--------|---------------------------|----------------------|---------|
| `scrub.rs:184` | `query_scrubbed.as_ref() == s` → `&*query_scrubbed == s` | NO — Phase-49 surface is in `setup.rs` + `trust/bundle.rs` | NO — D-32-15 path is in `trust/bundle.rs`, not `scrub.rs` | NO COLLISION |
| `scrub.rs:278` | `scrubbed.as_ref() != header_value.trim_start()` → `&*scrubbed != header_value.trim_start()` | NO — same reasoning | NO — same reasoning | NO COLLISION |

**Analysis of each change line:**

**Line 1 (`scrub_value_with_policy`, line 184 in fork's current file):**
- Context: comparing a `Cow<'_, str>` to a `&str` to determine if scrubbing made any change.
- `Cow::as_ref()` on `Cow<'_, str>` returns `&str` (via `AsRef<str>` impl).
- `&*cow` on `Cow<'_, str>` also returns `&str` (via `Deref<Target = str>` impl).
- Both forms compile to the same code. This is a clippy-style suggestion to prefer the Deref form.
- No behavior change. Does not touch any sigstore API call site.
- Does not touch `--from-file` path, `trusted_root.json` deserialization, TUF verification, or
  any code path in `trust/bundle.rs`.

**Line 2 (`scrub_header_arg`, line 278 in fork's current file):**
- Context: comparing a `Cow<'_, str>` returned by `scrub_header_with_policy` to `&str`.
- Same mechanical change: `as_ref()` → `&*` deref notation.
- No behavior change. Does not touch any sigstore API call site.
- Does not touch `--from-file` path, `trusted_root.json` deserialization, TUF verification, or
  any code path in `trust/bundle.rs`.

---

## 5. Cargo Bump Diff (crates/nono/Cargo.toml)

```diff
-sigstore-verify = { version = "0.6", default-features = false }
-sigstore-trust-root = "=0.6.3"
+sigstore-verify = { version = "0.8.0", default-features = false }
+sigstore-trust-root = "=0.8.0"
```

**Fork's current state (Phase 37 + Phase 50 divergence):**
- `crates/nono/Cargo.toml` has `sigstore-verify = { version = "0.7.0", default-features = false, features = ["tuf"] }` (Phase 37 bumped 0.6.5→0.7.0)
- `crates/nono/Cargo.toml` does NOT have a `sigstore-trust-root` entry (removed; dependency via transitive graph at 0.7.0)
- `crates/nono-cli/Cargo.toml` has `sigstore-sign = "0.7.0"` + `sigstore-trust-root = "0.7.0"` (Phase 37 + Phase 50 additions)

The fork is at 0.7.0 for all sigstore deps. The upstream bump is from 0.6.x → 0.8.0. The fork's equivalent bump is 0.7.0 → 0.8.0. The Plan 2 implementation must apply the bump to the fork's current 0.7.0 baseline.

**Cargo bump verdict: will-sync** (explicit Cargo.toml edits required rather than cherry-pick due to the fork's 0.7.0→0.8.0 delta vs upstream's 0.6.x→0.8.0 format, and Phase 50's restructured dependency layout).

---

## 6. Overall Verdict

**CLEAR — port scrub.rs verbatim AND apply Cargo.toml bumps**

The upstream `e581569` scrub.rs change is a purely mechanical Cow deref notation adjustment
(`as_ref()` → `&*`). It:

1. Does NOT touch any sigstore API call site in scrub.rs (scrub.rs has zero imports from sigstore)
2. Does NOT touch the Phase-49 trust-root surface (`setup.rs` / `trust/bundle.rs`)
3. Does NOT alter the D-32-15 verify-is-offline invariant (the invariant lives in `trust/bundle.rs`)
4. Does NOT introduce any new API call points, network paths, or TUF verification calls
5. Is a style normalization only — identical runtime behavior to the fork's current code

**D-32-15 invariant status:** NOT REGRESSED. The scrub.rs changes have no path to the
`trusted_root.json` deserialization code. The verify path in `trust/bundle.rs` is untouched.
Post-cherry-pick verification: `grep -n "TUF\|tuf\|update_trusted_root\|fetch" crates/nono/src/scrub.rs`
will return zero results (scrub.rs has no sigstore imports at all; this is expected and correct).

**Sub-verdict for scrub.rs:** port-verbatim (apply both Cow deref changes with D-19 trailer)

**Cargo bump sub-verdict:** Apply manual edits (not cherry-pick) because:
- Fork is at 0.7.0 baseline; upstream e581569 was authored from a 0.6.x baseline
- Fork's `crates/nono/Cargo.toml` uses `features = ["tuf"]` which upstream did not have at 0.6.x
- Fork's `crates/nono-cli/Cargo.toml` has additional sigstore deps from Phase 50 (`sigstore-trust-root`, `tough`) that would conflict with a raw cherry-pick
- The correct approach: manually bump all sigstore dep version strings from 0.7.0 → 0.8.0 in both Cargo.toml files, then `cargo update` to regenerate Cargo.lock

---

## 7. Execution Plan (for Plan 55-07 Task 2)

**Branch A (CLEAR):**

1. Apply scrub.rs Cow deref changes verbatim (two-line change)
2. Apply Cargo.toml bumps manually:
   - `crates/nono/Cargo.toml`: bump `sigstore-verify` version "0.7.0" → "0.8.0"
   - `crates/nono-cli/Cargo.toml`: bump `sigstore-sign` "0.7.0" → "0.8.0" and `sigstore-trust-root` "0.7.0" → "0.8.0"
3. Run `cargo update -p sigstore-verify -p sigstore-sign -p sigstore-trust-root` to regenerate Cargo.lock
4. Run `cargo build --workspace` to verify all 5 crates build
5. Run `cargo test --workspace` to verify tests pass
6. Commit with D-19 trailer (Upstream-commit: e581569659d35b50bb64d8086fc240d715f5e049)

**Threat T-55-07-01 status:** MITIGATED — diff-inspection confirms no D-32-15 regression.  
**Threat T-55-07-02 status:** MITIGATED — sigstore-rs and reqwest are canonical upstream deps already in lockfile; Cargo.lock pin prevents phantom version injection.  
**Threat T-55-07-03 status:** MITIGATED — scrub.rs has zero sigstore API imports; the Cow deref change is mechanical and does not introduce new API entry points.  
**Threat T-55-07-SC status:** MITIGATED — sigstore-rs is the fork's own upstream dep (already in lockfile at 0.7.0); reqwest is a canonical Rust HTTP crate. No [ASSUMED]/[SUS] packages.
