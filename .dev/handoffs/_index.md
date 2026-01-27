---
type: index
updated: 2026-01-27
---

# Handoffs Index

> Agent-to-agent task transitions. Check here for work assigned to your role.

---

## Active Handoffs (Awaiting Human/Hardware)

| ID | Work Package | Priority | Status | Created |
|----|--------------|----------|--------|---------|
| [HO-BLE-001](HO-BLE-001.md) | WP-BLE | MEDIUM | Scaffolding complete - awaiting hardware | 2026-01-23 |

---

## Human-Blocked Work

The following handoffs require human intervention before completion:

| Handoff | Blocker | Action Required |
|---------|---------|-----------------|
| HO-BLE-001 | Physical hardware | Test with Bluetooth headsets |

See [.dev/human.md](../human.md) for detailed instructions.

---

## Completed Handoffs

| ID | Work Package | Completed | Notes |
|----|--------------|-----------|-------|
| [HO-CONFIG-001](HO-CONFIG-001.md) | WP-CONFIG | 2026-01-23 | System profiling + CLI config |
| [HO-TUI-001](HO-TUI-001.md) | WP-TUI | 2026-01-25 | Hardware verification passed |
| [HO-HASH-001](HO-HASH-001.md) | WP-HASH-VERIFY | 2026-01-25 | All hashes acquired and verified |
| [HO-TTS-QUALITY-001](HO-TTS-QUALITY-001.md) | WP-TTS-QUALITY | 2026-01-25 | MOS 3.0, shipped as known limitation |

---

## Handoff Workflow

1. **Sender** creates handoff document in `handoffs/`
2. **Sender** updates this index
3. **Recipient** reviews handoff, acknowledges receipt
4. **Recipient** begins work, updates work package status
5. **Recipient** completes work, creates return handoff if needed
6. **Sender/Recipient** archives completed handoff to `archive/handoffs/`

---

*Last updated: 2026-01-27. Four of four human-blocked items complete (hashes acquired and integrated). Only Bluetooth hardware testing remains for WP-BLE. See [.dev/human.md](../human.md) for details.*
