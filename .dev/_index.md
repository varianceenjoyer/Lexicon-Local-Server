---
type: index
updated: 2026-01-22
---

# Lexicon Local Server — Development Hub

> **Framework**: Agentic Project Management v1.0
>
> This directory contains all development artifacts organized for multi-agent collaboration.

---

## Quick Navigation

| Directory | Purpose | Primary Consumer |
|-----------|---------|------------------|
| [design/](design/_index.md) | Architecture, research, decisions | design-lead |
| [work/](work/_index.md) | Active work packages | developer |
| [handoffs/](handoffs/_index.md) | Agent-to-agent task transitions | all agents |
| [status/](status/_index.md) | Point-in-time status snapshots | all agents |
| [archive/](archive/_index.md) | Completed/superseded documents | reference |
| [prompts/](prompts/) | Agent prompt templates | agents |
| [docs/](docs/) | Generated documentation | external |
| [commands/](commands/) | Development commands | developer |

---

## Agent Roles

### design-lead

**Purpose**: Assess project state, identify priorities, create work plans

**Inputs**:
- Status reports (`status/`)
- User requirements (external)
- Research findings (`design/research/`)
- Completed work feedback (`archive/`)

**Outputs**:
- Work packages (`work/WP-*.md`)
- Architecture decisions (`design/decisions/ADR-*.md`)
- Handoffs to developers (`handoffs/`)
- Roadmap updates (`design/roadmap.md`)

**Monitors**:
- `status/` — Project health
- `design/research/` — Technical findings
- `work/_index.md` — Work package status

---

### developer

**Purpose**: Implement work packages, write code, create tests

**Inputs**:
- Work packages (`work/WP-*.md`)
- Handoff documents (`handoffs/` where `to: developer`)
- Architecture docs (`design/architecture.md`)
- Research docs (`design/research/`)

**Outputs**:
- Code changes (in source tree)
- Implementation notes (appended to work packages)
- Completion handoffs back to design-lead
- Test results

**Monitors**:
- `work/_index.md` — Active work
- `handoffs/_index.md` — Pending tasks

---

### qa

**Purpose**: Validate implementations, write test plans, sign off releases

**Inputs**:
- Completed work packages (`work/` with status: `review`)
- Test requirements from handoffs
- Implementation code

**Outputs**:
- Test plans
- Bug reports
- Sign-off documents
- Quality metrics

**Monitors**:
- `work/_index.md` — Items ready for review
- `handoffs/_index.md` — QA assignments

---

### archive-manager

**Purpose**: Maintain archive integrity, move completed documents, generate archive summaries

**Inputs**:
- Completed work packages (`work/` with status: `complete`)
- Completed handoffs (`handoffs/` with status: `complete`)
- Historical status reports (`status/`)
- Archive index (`archive/_index.md`)

**Outputs**:
- Archived documents (`archive/{type}/`)
- Updated index files
- Archive summary reports
- Integrity reports

**Monitors**:
- `work/_index.md` — Completed items ready to archive
- `handoffs/_index.md` — Completed handoffs
- `status/_index.md` — Status reports for archival
- `archive/_index.md` — Archive health

---

## Document Standards

### Frontmatter (Required)

Every document must include YAML frontmatter:

```yaml
---
id: WP-EXAMPLE           # Unique identifier
type: work-plan          # work-plan | research | decision | handoff | status
status: in-progress      # draft | ready | in-progress | blocked | review | complete | archived
phase: beta              # mvp | beta | release
created: 2026-01-22
updated: 2026-01-22
author: design-lead      # Agent role that created this
consumers: [developer]   # Agent roles that should read this
depends-on: [WP-CORE]    # Dependencies (optional)
blocks: [WP-MOBILE]      # What this blocks (optional)
priority: high           # critical | high | medium | low
tags: [audio, streaming] # Searchable tags
---
```

### Status Values

| Status | Meaning |
|--------|---------|
| `draft` | Work in progress, not ready for consumption |
| `ready` | Specification complete, ready for implementation |
| `in-progress` | Active development underway |
| `blocked` | Waiting on dependency or decision |
| `review` | Implementation complete, awaiting review |
| `complete` | Fully done, may be archived |
| `archived` | Moved to archive, kept for reference |

### File Naming

- Work packages: `WP-{NAME}.md` (e.g., `WP-AUDIO.md`)
- Research: `R-{CAT}-{NN}-{slug}.md` (e.g., `R-AUD-01-cpal-mobile-audio.md`)
- Decisions: `ADR-{NNN}-{slug}.md` (e.g., `ADR-001-rust-over-go.md`)
- Handoffs: `HO-{DATE}-{slug}.md` (e.g., `HO-2026-01-22-config-implementation.md`)
- Status: `STATUS-{DATE}.md` (e.g., `STATUS-2026-01-22.md`)

---

## Workflow Patterns

### Design Lead → Developer Handoff

1. **Design Lead** creates/updates work package in `work/WP-*.md`
2. **Design Lead** creates handoff in `handoffs/HO-*.md` with:
   - Context and rationale
   - Links to relevant inputs
   - Expected deliverables
   - Acceptance criteria
3. **Developer** acknowledges handoff, begins work
4. **Developer** updates work package status to `in-progress`
5. **Developer** completes work, updates status to `review`
6. **Developer** creates handoff back to design-lead
7. **Design Lead** reviews, marks `complete`, archives if done

### Archival Process

When work is complete:

1. Update document `status: archived` in frontmatter
2. Move to `archive/{type}/` directory
3. Update source `_index.md` to remove entry
4. Update `archive/_index.md` to add entry

---

## Tools Integration

This framework integrates with:

- **MCP Agent Mail** — Async agent communication (see [.tools.md](.tools.md))
- **Beads** — Task dependency tracking (`bd` CLI)
- **bv** — Graph-aware triage engine

Use Beads issue IDs as `thread_id` in Agent Mail for traceability.

---

## Getting Started (Agents)

1. **Identify your role** from the Agent Roles section above
2. **Check your monitors** — Start with the directories listed for your role
3. **Read active handoffs** — Check `handoffs/_index.md` for tasks assigned to you
4. **Update status** — Always update document status as you work
5. **Create handoffs** — When passing work to another role, create a handoff document
