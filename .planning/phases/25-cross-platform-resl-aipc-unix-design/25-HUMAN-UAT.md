---
status: partial
phase: 25-cross-platform-resl-aipc-unix-design
source: [25-VERIFICATION.md]
started: 2026-05-10T17:50:00Z
updated: 2026-05-10T17:50:00Z
---

## Current Test

[awaiting human testing on Linux/macOS host]

## Tests

### 1. Linux OOM kill via cgroup v2 memory.max
expected: `nono run --memory 256m -- bash -c 'tail -c 1G </dev/urandom'` exits non-zero (OOM-killed by cgroup). Optional: `nono inspect <id>` shows `memory_kill: true` — note this field was scoped as optional follow-up in the plan.
result: [pending]

### 2. Linux fork-bomb mitigation via cgroup v2 pids.max
expected: `nono run --max-processes 10 -- bash -c 'for i in {1..20}; do sleep 60 & done; wait'` exits non-zero (fork failure after 10 processes).
result: [pending]

### 3. Linux supervisor watchdog timeout
expected: `nono run --timeout 5s -- sleep 60` exits non-zero within 3-10s (watchdog fires via `cgroup.kill`).
result: [pending]

### 4. Linux no-warning assertion
expected: Running any of the above commands emits zero stderr lines containing `is not enforced on linux` (the Unix-side stub warnings are gone).
result: [pending]

### 5. macOS RLIMIT_AS enforcement
expected: `nono run --memory 256m -- bash -c '<large alloc>'` exits non-zero (RLIMIT_AS aborts the child during mmap).
result: [pending]

### 6. macOS --cpu-percent clap rejection
expected: `nono run --cpu-percent 50 -- ls` exits non-zero at parse time with error message indicating cpu-percent is not supported on macOS; no child spawned.
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0
blocked: 0

## Gaps
