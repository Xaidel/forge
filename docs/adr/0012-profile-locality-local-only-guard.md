---
status: accepted
date: 2026-11-05
---

# Profiles declare locality, and a local primary pins auxiliary calls

## Context

FR-MODEL-002 says the `local` slot is used only when explicitly selected or configured as the repo default, and that selecting `local` means no repository content, including auxiliary calls, leaves the machine for model calls unless the config explicitly names a cloud profile. NFR-PRIV-001 requires session state, reasoning, keys, and artifacts to remain local and committed records to exclude secrets and raw context.

A profile currently has a base URL, protocol, auth, and model defaults, but no locality. A loopback URL usually means local; a LAN endpoint may be local; a corporate gateway on loopback may be cloud. Inferring from the host is therefore wrong in both directions: it can send repository content to a cloud gateway that happens to be on localhost, or refuse a legitimate local model on a private LAN.

## Decision

Every profile declares its locality:

- `[profiles.<name>] locality = "local" | "cloud"`.
- The default is `cloud`. A loopback host (`localhost`, `127.0.0.0/8`, `::1`) defaults to `local` only when the field is absent. An explicit value always wins.
- Validation warns when `locality = "local"` but the host is public. It does not hard-fail, because a VPN or a private DNS name may legitimately resolve to a local service.

When a task's `primary` slot resolves to a local profile, the task is **local-only**:

- Every model call, including titles, summaries, compaction, and classification, must use a profile whose locality is `local`.
- A routing entry that names a cloud profile is refused at task start and before each call.
- The only exception is an explicit `[privacy] allow_cloud_auxiliary = true` in `.forge/config`, which is shown in the task header and review packet.
- An explicit `/model` switch from a local profile to a cloud profile requires typed confirmation, is audited, and updates the task model hash. It is never automatic and never a fallback.

## Consequences

- The privacy claim is enforceable and visible, not inferred from a URL.
- A custom profile must declare `locality = "local"` to join a local-only task; this is one extra config line, and the init template fills it for loopback endpoints.
- Selecting `local` for `primary` with a cloud `fast` slot is a configuration error unless the human opts in; the task refuses before sending any content.
- A mid-task local-to-cloud switch is possible but deliberate and recorded.
- The review packet and attestation can state the effective privacy boundary of the task.
- The `locality` field and the `[privacy]` table amend TDD-0001; the task guard amends the model routing contract in TDD-0007.

## Considered options

- **Infer locality from the URL only.** Rejected: misclassifies loopback gateways and private LAN services.
- **No locality field; trust the developer's config.** Rejected: FR-MODEL-002 E3 makes a privacy promise that routing alone cannot keep when auxiliary calls use another slot.
- **Hard-fail any local profile with a non-loopback host.** Rejected: private DNS, VPNs, and LAN hosts are legitimate; a warning plus explicit declaration is honest.
- **Make the whole task local-only only when the `local` slot is active, not when `primary` is local.** Rejected: a task can use `primary` for consequential calls and `fast` for summaries; if `primary` is local, the task's privacy intent is local.
- **Explicit `locality` plus a local-only task guard and an audited opt-in.** Chosen: precise, enforceable, and reviewable.

## Links

- [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
- [Define the model adapters, provider profiles, and cost contract (TDD-0007)](https://github.com/Xaidel/forge/issues/27)
- `docs/PRD.md` §7.9 FR-MODEL-002, FR-MODEL-006; NFR-PRIV-001; D-16
- [TDD-0001](../tdd/0001-cli-config-init-repo-map.md) §6.9–§6.11
- [TDD-0007](../tdd/0007-model-adapters-providers-cost.md) §5.1, §8.1, §13.5
