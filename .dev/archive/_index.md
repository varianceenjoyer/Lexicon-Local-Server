---
type: index
updated: 2026-01-22
---

# Archive Index

> Completed and superseded documents. Kept for reference and audit trail.

---

## Archive Structure

```
archive/
├── work/       # Completed work packages
├── handoffs/   # Completed handoffs
└── status/     # Historical status reports
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
| *None archived yet* | | | |

---

## Historical Status Reports

| Date | Phase | Key Events |
|------|-------|------------|
| *None archived yet* | | |

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
