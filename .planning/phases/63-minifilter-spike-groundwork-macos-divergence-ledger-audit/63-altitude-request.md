# Microsoft Minifilter Altitude Request

**Phase:** 63 (nono v2.10 Kernel-Driver Spike)
**Requirement:** DRV-03 (partial — altitude clock)
**Maintainer/Contact:** Oscar Mack Jr `<oscar.mack.jr@gmail.com>`

---

## Submission Channel

**Recipient:** `fsfcomm@microsoft.com`
**Subject:** `Filter altitude request`

> Note: As of Phase 63 research (2026-06-06), Microsoft's official altitude-request page at
> `learn.microsoft.com/windows-hardware/drivers/ifs/minifilter-altitude-request` still lists
> `fsfcomm@microsoft.com` as the submission address with no web-form replacement. The prior
> agent verified this directly. If Microsoft has since moved to a web form, record the
> verified URL here and note that the email address was superseded.

---

## Ready-to-Send Email

Copy the text below exactly (or paste into your mail client) and send it to `fsfcomm@microsoft.com`.

```
To: fsfcomm@microsoft.com
Subject: Filter altitude request

Hello,

I am the maintainer of nono (https://github.com/OscarMackJr/nono), an open-source
capability-based sandboxing system for running untrusted AI agents with OS-enforced
isolation.

I am requesting a minifilter driver altitude assignment for the following driver:

  Driver name: nono-fltmgr
  Driver purpose: nono Gap 6b minifilter feasibility spike — observe/intercept
    file-open (pre-create IRP_MJ_CREATE) events for a capability-based sandbox.
    The driver acts as an Activity Monitor: it reports pre-create events to a
    user-mode supervisor (nono-cli) via FltSendMessage so the supervisor can
    enforce capability policy. The driver does NOT block I/O itself during the
    spike phase; deny decisions are made in user mode.

  Requested altitude band: FSFilter Activity Monitor (360000–389999)
  Requested altitude: a non-colliding number within the FSFilter Activity Monitor
    band (360000–389999). We explicitly do NOT request an altitude in the AV/EDR
    range (320000–329998), as registering there would risk colliding with or
    disabling installed security products.

  Company/Organization: nono project (open-source, individual maintainer)
  Contact name: Oscar Mack Jr
  Contact email: oscar.mack.jr@gmail.com

Please let me know if additional information is needed.

Thank you,
Oscar Mack Jr
oscar.mack.jr@gmail.com
```

---

## Requested Band Details

| Parameter | Value | Notes |
|-----------|-------|-------|
| Band | FSFilter Activity Monitor | Matches the driver's observe/intercept role (not blocking) |
| Band range | 360000–389999 | Official Microsoft load-order group for activity-monitor filters |
| AV range to avoid | 320000–329998 | Collision here can blind or break installed EDR/AV products (Pitfall 5 / D-08) — this band is explicitly NOT requested |
| Placeholder (Phase 63) | 370020 (nullFilter default) | Used in the INF during the spike only; replaced with the assigned altitude before any non-disposable deployment |

---

## Send-Status

| Field | Value |
|-------|-------|
| Status | `pending` — sent 2026-06-07; awaiting Microsoft altitude assignment (~30 business days) |
| Send-date | `2026-06-07` |
| Microsoft response | `[to be filled when Microsoft assigns an altitude]` |
| Assigned altitude | `[to be filled when Microsoft responds]` |

---

## Lifecycle

1. **Drafted** (Plan 63-02, 2026-06-07) — email body drafted by executor.
2. **Sent** (2026-06-07) — user sent the email to `fsfcomm@microsoft.com`; Send-date recorded above.
3. **Pending** — Microsoft processes the request (~30 business days).
4. **Assigned** — Microsoft replies with an official altitude number; record in this file and in `drivers/nono-fltmgr/DESIGN.md` Altitude Configuration table and the INF.

---

## Files to Update After Assignment

- `drivers/nono-fltmgr/DESIGN.md` — Altitude Configuration table, Microsoft assignment row
- `drivers/nono-fltmgr/nono-fltmgr.inf` — `Altitude` value under `[nono-fltmgr.Instances.Defaults]`
- This file — assigned altitude + response date
