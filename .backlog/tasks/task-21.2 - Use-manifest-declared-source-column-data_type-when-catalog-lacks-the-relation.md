---
id: TASK-21.2
title: Use manifest-declared source column data_type when catalog lacks the relation
status: To Do
assignee: []
created_date: '2026-10-06 09:18'
updated_date: '2026-10-06 13:26'
labels: []
milestone: m-2
dependencies: []
references:
  - >-
    sql-semantic-protocol TASK-34 (Use manifest-declared column data_type when
    catalog lacks a relation, milestone 1.1.0)
parent_task_id: TASK-21
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
dbt sources and models may declare `data_type` per column in YAML; the value appears in manifest.json. When catalog.json has no schema for a physical dependency (source tables not yet created in the warehouse), these declared types are legitimate type metadata and should be usable for generation. Per AGENTS.md, dbt artifact interpretation belongs in `sql-semantic-protocol` (`analyze_dbt_artifacts` in `src/dbt.rs` currently builds schemas only from the catalog). This requires a protocol change and release, then sql-tdg bumps its `sql-semantic-protocol` dependency and adds coverage. A relation with any column lacking both catalog and declared types must still fail explicitly. Decision (maintainer, 2026-10-06).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 sql-semantic-protocol accepts manifest-declared column data_type for physical dependencies absent from catalog.json
- [ ] #2 Catalog types take precedence over declared types when both exist
- [ ] #3 A relation missing types for any referenced column still fails with an explicit error
- [ ] #4 sql-tdg depends on the protocol release containing the fallback and an end-to-end test generates data for a dbt project whose sources exist only in YAML with declared types
<!-- AC:END -->
