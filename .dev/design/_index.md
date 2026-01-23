---
type: index
updated: 2026-01-22
---

# Design Index

> Architecture, research, and design decisions. Primary reference for design-lead role.

---

## Quick Navigation

| Document | Purpose |
|----------|---------|
| [architecture.md](architecture.md) | System architecture (authoritative) |
| [roadmap.md](roadmap.md) | High-level phases and milestones |
| [research/](research/_index.md) | Technical research findings |
| [decisions/](decisions/_index.md) | Architecture Decision Records |

---

## Architecture

The authoritative system architecture document:
- **[architecture.md](architecture.md)** — Component design, interfaces, data flow

---

## Roadmap

High-level project phases and delivery strategy:
- **[roadmap.md](roadmap.md)** — MVP → Beta → Release phases

---

## Research

Technical research organized by category:

| Category | Prefix | Count |
|----------|--------|-------|
| Audio | R-AUD | 1 |
| Bluetooth | R-BLE | 2 |
| Database | R-DB | 1 |
| Diarization | R-DIA | 1 |
| Error Handling | R-ERR | 1 |
| Licensing | R-LIC | 1 |
| ORT/ONNX | R-ORT | 1 |
| Security | R-SEC | 1 |
| System | R-SYS | 1 |
| Thermal | R-THR | 1 |
| TTS | R-TTS | 1 |
| UX | R-UX | 1 |
| Whisper | R-WRS | 1 |

See [research/_index.md](research/_index.md) for full index.

---

## Decisions

Architecture Decision Records (ADRs) track significant technical decisions:

| ID | Decision | Status |
|----|----------|--------|
| *Migrate ADRs from design-plan.md* | | |

Key decisions currently embedded in design-plan.md:
- **D1**: Immediate model download on demand
- **D2**: Bluetooth HFP 1.6 support
- **D3**: Speaker diarization configuration
- **D4**: Multi-language TTS fallback

See [decisions/_index.md](decisions/_index.md) for ADR index.

---

## Document Relationships

```
architecture.md (authoritative)
       │
       ├──▶ roadmap.md (delivery phases)
       │
       ├──▶ research/*.md (technical findings)
       │         │
       │         └──▶ inform decisions
       │
       └──▶ decisions/*.md (ADRs)
                 │
                 └──▶ work packages in ../work/
```
