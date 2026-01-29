---
type: index
updated: 2026-01-29
---

# Archive Index

> Completed and superseded documents. Kept for reference and audit trail.

---

## Archive Structure

```
archive/
├── work/       # Completed work packages
├── handoffs/   # Completed handoffs
├── status/     # Historical status reports
└── docs/       # Superseded documentation
    └── cuttlefish/  # Archived Cuttlefish emulation docs
```

---

## Completed Work Packages

| ID | Title | Completed | Phase | Outcome |
|----|-------|-----------|-------|---------|
| *None archived yet* | | | | |

---

## Completed Handoffs

| ID | From → To | Work Package | Completed |
|----|-----------|--------------|-----------|
| HO-2026-01-22-config | design-lead → developer | WP-CONFIG | 2026-01-23 |

---

## Historical Status Reports

| Date | Phase | Key Events |
|------|-------|------------|
| *None archived yet* | | |

---

## Archived Documentation

| Directory | Description | Archived | Reason |
|-----------|-------------|----------|--------|
| [docs/cuttlefish/](docs/cuttlefish/_index.md) | Cuttlefish Android emulation | 2026-01-29 | Replaced by Android Studio |

---

## Archival Process

When work is complete:

1. **Update status** in document frontmatter to `archived`
2. **Move file** to appropriate archive subdirectory
3. **Update source index** to remove entry
4. **Update this index** to add entry
5. **Commit changes** with message referencing the completed work

**Example:**
```bash
# Archive completed work package
mv .dev/work/WP-CORE.md .dev/archive/work/
# Update indexes accordingly
```
