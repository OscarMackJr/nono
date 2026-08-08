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
