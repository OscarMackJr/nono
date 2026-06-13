# 69-01 Lock Notes — UPST8 Audit Upstream Re-fetch

upstream_head_at_audit: 849cda42c0541f18915708cd3ff31d61c12d136d
refetch_date: 2026-06-13
v0.60.0_sha: 9a05a4ff1a4cc8944ccd1da880432b3efe86a051
v0.61.2_sha: 3e605f2716483a326fed49784a6c70412af62f35
v0.62.0_upstream_sha: 52809dda3b9ec5d7a237c26ac5e90840052993d9
v0.62.0_local_fork_tag: 3c5e9025 (FORK RELEASE — divergent history; MUST NOT use as --to bound)
upstream_newer_than_v0.62.0: none
plan_base_sha: fee511b7af74ec79f1b850d4164312eb114980f4

## SHA Collision Guard Details

The local tag `v0.62.0` was created by the fork's v2.8/v2.9 leapfrog strategy (fork needed to
clear upstream's tag line). It resolves to `3c5e902570c394483e360102268e53911d93d139` — a fork
release commit on divergent history. Upstream's real `v0.62.0` is `52809dda3b9ec5d7a237c26ac5e90840052993d9`,
confirmed via `git ls-remote upstream refs/tags/v0.62.0`.

A bare `git rev-list --count v0.61.2..v0.62.0` returns ~1889 (garbage) because it compares
upstream-v0.61.2 against fork-v0.62.0 across divergent history lines. The drift-tool `--to`
bound MUST be the literal SHA `52809dda`, never the tag.

## Upstream tag inventory (v0.6x line)

Upstream tags confirmed via `git ls-remote upstream 'refs/tags/v0.6*'`:
- v0.60.0 = 9a05a4ff1a4cc8944ccd1da880432b3efe86a051 (range start; confirmed)
- v0.61.0 = 658e40f89165453b33a2f334c5a1b158d54988b7
- v0.61.1 = b37198c03dde892bb422705b43288596aa210f0b
- v0.61.2 = 3e605f2716483a326fed49784a6c70412af62f35
- v0.62.0 = 52809dda3b9ec5d7a237c26ac5e90840052993d9 (range end; the --to bound)

No upstream v0.62.1 or v0.62.2 tag exists. The `v0.7.0` upstream tag (`0ef6a7f1da19dc4dd29fe1b10bce61fea5a41843`)
belongs to the old pre-v0.60 numbering series and is NOT a newer release in the v0.6x line.
`upstream_newer_than_v0.62.0: none` — D-03 UPST9 deferral gate does NOT fire.

## Re-fetch method note

`git fetch upstream --tags` was rejected due to the tag collision (`v0.62.0` would clobber the
local fork tag). This is the expected behavior confirming the D-02 landmine is live. The upstream
state was obtained via:
1. `git fetch upstream` (branches only; upstream/main advanced)
2. `git ls-remote upstream refs/tags/v0.60.0 refs/tags/v0.61.2 refs/tags/v0.62.0` (SHA verification)
3. `git ls-remote upstream 'refs/tags/v0.6*'` (full v0.6x inventory)

This approach is safe and equivalent: `upstream/main` is at `849cda42` and the upstream tag SHAs
are verified independently. The drift tool uses explicit `--from`/`--to` SHA bounds, not tag names.
