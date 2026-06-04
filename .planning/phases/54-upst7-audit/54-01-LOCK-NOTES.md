---
phase: 54-upst7-audit
plan: 01
doc_type: lock-notes
upstream_head_at_audit: 48d39f3635f339e439d43869f8c98bc1db9b6dc1
refetch_date: 2026-06-04
range: v0.57.0..v0.59.0
v0.57.0_sha: 10cec9845e14db24a50bf8e4a0fdda30c8395359
v0.58.0_sha: 54c4deb6fbc14ea751b65f73d697d2d6aa191873
v0.59.0_sha: e61814f8a70a53346a1e9d0bcf7ba4f52e0e4d1d
v0.59.x_patch: none
v0.60.0: 9a05a4ff1a4cc8944ccd1da880432b3efe86a051
plan_base_sha: eb8c9b82ef2be45644080ac68e6d68a58d1169fb
---

# Plan 54-01 Lock Notes — SC3 upstream re-fetch + reproducibility pins

Source-of-truth holding file for the `54-DIVERGENCE-LEDGER.md` frontmatter
(`upstream_head_at_audit` + `refetch_date`) and the Task 9 close-gate base ref (`plan_base_sha`).

## SC3 re-fetch (2026-06-04)

`git fetch upstream --tags` run at audit-open. Before the fetch, `v0.58.0` / `v0.59.0` did NOT
resolve locally (upstream/main was stale at the v0.57.0 era); after the fetch they resolve.

- **upstream_head_at_audit:** `48d39f3635f339e439d43869f8c98bc1db9b6dc1` (post-fetch `git rev-parse upstream/main`)
- **refetch_date:** `2026-06-04` (UTC)

## Anchor-tag asserts (all PASS)

| tag | resolved sha | status |
|-----|--------------|--------|
| v0.57.0 | `10cec9845e14db24a50bf8e4a0fdda30c8395359` | in-range (fork baseline; Phase 48 UPST6 sync point) |
| v0.58.0 | `54c4deb6fbc14ea751b65f73d697d2d6aa191873` | in-range |
| v0.59.0 | `e61814f8a70a53346a1e9d0bcf7ba4f52e0e4d1d` | in-range (range cap) |
| v0.60.0 | `9a05a4ff1a4cc8944ccd1da880432b3efe86a051` | **OUT OF RANGE** — scope decision Task 4 / human (default: defer to UPST8) |

- **v0.59.x patch:** none (`git tag -l 'v0.59.*'` → only `v0.59.0`).
- **plan_base_sha:** `eb8c9b82ef2be45644080ac68e6d68a58d1169fb` (HEAD immediately before Plan 54-01's
  first commit; the Task 9 zero-source-edits close-gate diffs `${plan_base_sha}..HEAD`).

## ⚠ Deviation from plan — upstream moved further than anticipated

This re-fetch also brought **v0.61.0 and v0.61.1** (NOT just v0.60.0, which is all the plan + the
260527-sgo gap analysis knew about). All three are **OUT OF RANGE** for the locked
`v0.57.0..v0.59.0` audit and are the **deferred-to-UPST8** set:

- v0.60.0 = `9a05a4ff1a4cc8944ccd1da880432b3efe86a051`
- v0.61.0 / v0.61.1 = present (newly cut since the 2026-05-27 gap analysis)

The Task 4 v0.60.0 scope decision and the Task 8 UPST8 stub must reflect that the deferred set is
now **v0.60.0..v0.61.1**, not v0.60.0 alone. These are NOT the unrelated Feb-2026 v0.6.x tag line.

Do NOT confuse v0.60.0/v0.61.x with the unrelated Feb-2026 `v0.6.0`/`v0.6.1` tags.
