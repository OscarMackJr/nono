---
quick_id: 260603-i31
slug: cosign-sigstore-signing-authority
type: research-determination
date: 2026-06-03
---

# Determine: can sigstore/cosign be the signing authority for the v2.9 Windows release?

The v2.9 / v0.58.0 release is BLOCKED on D-02 (no production Authenticode code-signing
material — the repo secrets hold the self-signed POC cert / expired). The operator has the
`cosign` utility available and proposes adding a sigstore signing step:

```yaml
- name: Install Cosign
  uses: sigstore/cosign-installer@v3
- name: Sign the publishing artifacts
  run: cosign sign-blob --yes my-built-binary.tar.gz
```

Question: can sigstore/cosign serve as the **signing authority** for this release (i.e. does it
resolve D-02), or only as a complementary layer?

Deliverable: a grounded determination (SUMMARY.md) — verdict + reasoning + the real D-02
remediation paths. No code change in this task.
