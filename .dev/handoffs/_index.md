---
type: index
updated: 2026-01-23
---

# Handoffs Index

> Agent-to-agent task transitions. Check here for work assigned to your role.

---

## Active Handoffs (Awaiting Human/Hardware)

| ID | Work Package | Priority | Status | Created |
|----|--------------|----------|--------|---------|
| [HO-TUI-001](HO-TUI-001.md) | WP-TUI | HIGH | Feature complete - awaiting hardware test | 2026-01-23 |
| [HO-HASH-001](HO-HASH-001.md) | WP-HASH-VERIFY | HIGH | Scaffolding complete - awaiting hashes | 2026-01-23 |
| [HO-BLE-001](HO-BLE-001.md) | WP-BLE | MEDIUM | Scaffolding complete - awaiting hardware | 2026-01-23 |
| [HO-TTS-QUALITY-001](HO-TTS-QUALITY-001.md) | WP-TTS-QUALITY | MEDIUM | Infrastructure complete - awaiting human eval | 2026-01-23 |

---

## Human-Blocked Work

The following handoffs require human intervention before completion:

| Handoff | Blocker | Action Required |
|---------|---------|-----------------|
| HO-TUI-001 | Hardware test | Test TUI with Whisper model on audio hardware |
| HO-HASH-001 | Model downloads | Download models and compute SHA-256 hashes |
| HO-BLE-001 | Physical hardware | Test with Bluetooth headsets |
| HO-TTS-QUALITY-001 | Subjective evaluation | Listen and rate TTS audio quality |

See [.dev/human.md](../human.md) for detailed instructions.

---

## Completed Handoffs

| ID | Work Package | Completed | Notes |
|----|--------------|-----------|-------|
| [HO-CONFIG-001](HO-CONFIG-001.md) | WP-CONFIG | 2026-01-23 | System profiling + CLI config |

---

## Handoff Workflow

1. **Sender** creates handoff document in `handoffs/`
2. **Sender** updates this index
3. **Recipient** reviews handoff, acknowledges receipt
4. **Recipient** begins work, updates work package status
5. **Recipient** completes work, creates return handoff if needed
6. **Sender/Recipient** archives completed handoff to `archive/handoffs/`

---

*Last updated: 2026-01-23. All Beta work packages feature-complete or scaffolding-complete - awaiting human/hardware actions.*
