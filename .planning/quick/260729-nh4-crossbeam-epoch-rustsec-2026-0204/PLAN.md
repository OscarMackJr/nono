---
quick_id: 260729-nh4
slug: crossbeam-epoch-rustsec-2026-0204
title: "Fix live RUSTSEC-2026-0204 — bump crossbeam-epoch 0.9.18 -> 0.9.20"
type: security-dependency-bump
date: 2026-07-29
status: complete
origin: Phase 108 UPST12 divergence audit (DEPS cluster cargo-audit cross-reference)
---

# Fix live RUSTSEC-2026-0204 (`crossbeam-epoch` 0.9.18 → 0.9.20)

## Why this exists

Phase 108's divergence audit ran `cargo audit` against the fork's actual `Cargo.lock` while
classifying the DEPS cluster and found **RUSTSEC-2026-0204 live in the fork today** — with its
exact fix sitting unabsorbed in the very upstream window being audited (`373a67ae`, #1369).

This is the concrete payoff of CONTEXT decision **D-16**, which ruled that dependency-only
commits form a security-relevant cluster rather than noise. Had they been skipped as noise, this
would have gone unnoticed for another sync cycle.

Phase 112 SC3 prioritizes this fix but explicitly permits it being closed out-of-band by a quick
task — which is what this is. Fixing it here means the exposure closes now rather than waiting for
Phase 112 to be planned and executed.

## The advisory

```
Crate:     crossbeam-epoch
Version:   0.9.18
Title:     Invalid pointer dereference in `fmt::Pointer` impl for `Atomic` and `Shared`
           when the underlying pointer is invalid
Date:      2026-07-06
ID:        RUSTSEC-2026-0204
URL:       https://rustsec.org/advisories/RUSTSEC-2026-0204
Solution:  Upgrade to >=0.9.20
```

It reaches the shipped binaries, not just dev tooling:

```
crossbeam-epoch 0.9.18
└── crossbeam-deque 0.8.6
    └── ignore 0.4.26
        ├── nono-sandbox-cli 0.66.1
        └── nono-sandbox 0.66.1
            ├── nono-shell-broker 0.66.1
            ├── nono-sandbox-proxy 0.66.1
            └── nono-ffi 0.66.1
```

`crossbeam-epoch` is **transitive only** — it appears in no workspace `Cargo.toml`, so no manifest
edit is needed or appropriate.

## Scope

- **In scope:** `Cargo.lock` version + checksum bump, mirroring upstream `373a67ae` exactly.
- **Out of scope:** any source change; any other DEPS-cluster commit; the remaining pre-existing
  `cargo audit` *warnings* (async-std, fxhash, paste, rustls-pemfile, anyhow — unmaintained/unsound,
  not vulnerabilities, and untouched by this bump).

## Tasks

### Task 1 — bump the lockfile
`cargo update -p crossbeam-epoch --precise 0.9.20`

**Acceptance criteria**
- `git diff Cargo.lock` shows exactly 2 insertions / 2 deletions, `Cargo.lock` only
- The version and checksum lines match upstream `373a67ae`'s diff byte-for-byte
- No `Cargo.toml` is modified

### Task 2 — verify
**Acceptance criteria**
- `cargo audit` output contains zero matches for `RUSTSEC-2026-0204` or `crossbeam-epoch`
- `grep -A2 'name = "crossbeam-epoch"' Cargo.lock` reports `version = "0.9.20"`
- `cargo build --workspace --all-targets` finishes successfully
- No new advisories introduced relative to the pre-change baseline
