---
id: TASK-38
title: Generate jointly satisfiable output goals and per-terminal witness vectors
status: To Do
assignee: []
created_date: '2026-10-09'
updated_date: '2026-10-09'
labels: []
milestone: m-3
dependencies: 
  - TASK-37
  - TASK-43
references:
  - 'sql-semantic-protocol TASK-68'
  - 'sql-semantic-protocol TASK-69'
  - 'sql-semantic-protocol TASK-85'
  - 'sql-semantic-protocol TASK-86'
  - 'TASK-30'
  - 'TASK-35'
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generator-side implementation for the single SQL Semantic Protocol 3.0.0 contract. This task must not reinterpret SQL, guess missing semantics or claim partial/local operator witnesses imply complete physical-source/output correctness. Validate with the pinned unpublished protocol release candidate before publication, then with the published crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 Map user output row/group/histogram/distribution targets and per-terminal rejected classification to canonical protocol plans; distinguish --matching/--rejected source rows from output results.
- [ ] #2 Find globally feasible assignments across shared physical sources and divergent terminal branches, with deterministic scenario separation only where necessary.
- [ ] #3 Handle duplicate multiplicity, NULL, joined GROUP/HAVING/window/set row shaping and exact absence for rejected rows without violating generated relation constraints.
- [ ] #4 Emit metadata documenting per-row/per-terminal positive and rejected status, output goals, witnesses and proof reason; reject impossible and residual plans explicitly.
- [ ] #5 Execute entire dbt fixture terminal results plus impossible goal and conflict cases with reproducible data and outcomes.
- [ ] #6 Document supported versus residual variants, add full-result differential tests and update docs/dialect-conformance.md for every changed feature.
<!-- AC:END -->
