# Deferred Items — Phase 110

## Plan 02

- **`crates/nono-cli/src/profile/mod.rs` has 4 pre-existing `cargo fmt --check` diffs**
  (lines ~9164, ~9254, ~9402, ~9560), all inside Plan 110-01's `platform_overrides` test
  additions. Discovered while running `cargo fmt -p nono-sandbox-cli` for Plan 02's own
  changes (`cargo fmt` reformats the whole package, not just touched files). Out of scope
  for Plan 02 (file not modified by this plan's tasks) — reverted via
  `git checkout -- crates/nono-cli/src/profile/mod.rs` to avoid an unrelated diff.
  Left for whichever plan/task next touches `profile/mod.rs`, or a dedicated `cargo fmt`
  pass at the phase gate.
