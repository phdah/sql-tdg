---
id: TASK-40
title: Generate complete canonical typed and constrained physical rows
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
  - 'sql-semantic-protocol TASK-79'
  - 'sql-semantic-protocol TASK-87'
  - 'TASK-22'
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generator-side implementation for the single SQL Semantic Protocol 3.0.0 contract. This task must not reinterpret SQL, guess missing semantics or claim partial/local operator witnesses imply complete physical-source/output correctness. Validate with the pinned unpublished protocol release candidate before publication, then with the published crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 Map canonical signed/unsigned integer, decimal, float, temporal, string/binary, enum, array/map/struct/JSON and nullable types to lossless Arrow values; fail on unsupported or lossy types.
- [ ] #2 Enforce PK/unique/composite/FK, not-null, accepted-values/CHECK, collation, timezone, DEFAULT/identity/generated values when represented by the protocol.
- [ ] #3 Construct feasible satisfying and deliberately query-rejected rows without breaking enforced constraints or assuming unproven FK targets.
- [ ] #4 Use the same physical schema and constraints for raw SQL, compiled dbt/catalog, and ODCS evidence without adapter-specific solver rules.
- [ ] #5 Add boundary, overflow, NaN, NULL, datetime, constraint conflict and cross-source FK tests including exported Parquet/CSV round-trips.
- [ ] #6 Document supported versus residual variants, add full-result differential tests and update docs/dialect-conformance.md for every changed feature.
<!-- AC:END -->
