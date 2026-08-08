# Deferred Items — Phase 115

Out-of-scope discoveries surfaced during plan execution, logged per the executor's
scope-boundary rule (fix only what the current task's changes directly caused).

## 115-05

- **`../nono-py/src/override.rs` fails `cargo clippy -p nono-py --all-targets -- -D
  warnings`** (test-target build only; `cargo clippy -p nono-py --lib -- -D warnings`
  is clean). Three `clippy::unnecessary_map_or` findings (`stripped.first().map_or(false,
  ...)` / `b.first().map_or(false, ...)` at `override.rs:1276`, `:1573`, `:2290`) and
  three `unfulfilled_lint_expectations` findings on `#[expect(dead_code, ...)]`
  attributes at `override.rs:76`, `:102`, `:121` whose named construction sites
  (Phase 93 Plan 02/03) have apparently since landed, making the `#[expect]` stale.
  Pre-existing — unrelated to any file this plan (115-05) modified (`src/proxy.rs`,
  `src/undo.rs`). Not fixed here per the scope-boundary rule. Worth a follow-up
  quick-task or the next `../nono-py` touch to run `cargo clippy --fix` on the
  `map_or` findings and either construct the named variants or replace `#[expect]`
  with `#[allow]` on the three dead_code sites.

## 115-06

- **`../nono-py/python/nono_py/_nono_py.pyi` had pre-existing drift from the Rust
  API before this plan touched it**: `RouteConfig.__init__`/properties documented
  `tls_client_cert: str | None = None` and `tls_client_key: str | None = None`
  parameters that do not exist on `RouteConfig::new` in `src/proxy.rs` (verified
  by symbol — no such fields on `nono_proxy::config::RouteConfig` either). Not
  introduced by this plan; left untouched per the scope-boundary rule (this plan's
  own additions were inserted after `tls_ca` and before these two stale entries,
  so the new params are correctly positioned relative to the real signature but
  sit adjacent to the stale ones in the stub). Worth a follow-up quick-task to
  either remove the two stale stub entries or confirm whether a TLS
  client-cert/key mechanism was planned and never landed.

- **`cargo fmt --all -- --check` in `../nono-py` shows ~85 pre-existing diffs
  across `src/override.rs`, `src/override_trust.rs`, `src/windows_confined_run.rs`,
  `src/lib.rs`, and `src/undo.rs` (2 spots) — none touch a line this plan (115-06)
  authored. Verified directly: `src/override.rs`, `src/override_trust.rs`,
  `src/windows_confined_run.rs`, and `src/undo.rs` are entirely untouched by this
  plan's `git diff --stat` (0 insertions/deletions in each). `src/lib.rs` was
  touched (the `m.add_class::<proxy::*>()` registrations, ~line 739-750), but the
  fmt diffs in that file land at lines 14/22 (a pre-existing `mod` declaration
  ordering swap between `override_mod` and `override_trust`/`policy`/`proxy`) and
  line 781 (`verify_override_production` line-wrapping, an unrelated pre-existing
  call site) — nowhere near this plan's insertion point. `src/proxy.rs` (the file
  this plan wrote the most content into) shows zero fmt diffs after the one
  self-introduced formatting issue was fixed inline (an `assert!` line-length
  violation in the D-16 test, corrected during this plan's own execution).
  Likely a `rustfmt` edition/version mismatch between this dev host's toolchain
  and the pinned `rust-version = 1.95` / `edition = "2024"` in
  `../nono-py/Cargo.toml`. Not fixed here (out of scope, large surface, no line
  this plan touched). Worth a follow-up quick-task to align the local `rustfmt`
  toolchain and run a single crate-wide `cargo fmt --all`.
