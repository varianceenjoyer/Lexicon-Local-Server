<!-- AGENTIC_FRAMEWORK_SNIPPET_START -->
## agentic-framework v2.0

This project uses an agentic project management framework optimized for multi-agent collaboration.

### Quick Start

1. **Identify your role**: Check [.dev/_index.md](.dev/_index.md) for role definitions
2. **Find your work**: Check the directories your role monitors
3. **Track status**: Update document frontmatter as you work
4. **Create handoffs**: Use handoff documents when passing work between roles

### Directory Structure

| Directory | Purpose | Who Monitors |
|-----------|---------|--------------|
| [.dev/_index.md](.dev/_index.md) | Development hub, role definitions | All agents |
| [.dev/work/](.dev/work/_index.md) | Active work packages | developer |
| [.dev/handoffs/](.dev/handoffs/_index.md) | Agent-to-agent transitions | All agents |
| [.dev/design/](.dev/design/_index.md) | Architecture, research, decisions | design-lead |
| [.dev/status/](.dev/status/_index.md) | Project status snapshots | All agents |
| [.dev/archive/](.dev/archive/_index.md) | Completed documents | archive-manager |

### Agent Roles

- **design-lead**: Assess project state, create work plans, make architecture decisions
- **developer**: Implement work packages, write code and tests
- **qa**: Validate implementations, create test plans
- **archive-manager**: Move completed documents to archive, maintain archive integrity

### Document Standards

All documents use YAML frontmatter for filtering:
```yaml
---
id: WP-EXAMPLE
type: work-plan          # work-plan | research | decision | handoff | status
status: ready            # draft | ready | in-progress | blocked | review | complete
consumers: [developer]   # Who should read this
priority: high           # critical | high | medium | low
---
```

### Tools

- [.dev/.tools.md](.dev/.tools.md) — MCP Agent Mail, Beads, bv integration
- Do not access .dev/prompts unless instructed

<!-- AGENTIC_FRAMEWORK_SNIPPET_END -->
