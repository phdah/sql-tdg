---
id: TASK-41
title: Generate advanced table sources and finite recursive relation witnesses
status: To Do
assignee: []
created_date: '2026-10-09'
updated_date: '2026-10-09'
labels: []
milestone: m-3
dependencies: 
  - TASK-37
  - TASK-40
  - TASK-43
references:
  - 'sql-semantic-protocol TASK-77'
  - 'sql-semantic-protocol TASK-78'
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generator-side implementation for the single SQL Semantic Protocol 3.0.0 contract. This task must not reinterpret SQL, guess missing semantics or claim partial/local operator witnesses imply complete physical-source/output correctness. Validate with the pinned unpublished protocol release candidate before publication, then with the published crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 Consume protocol output/source obligations for VALUES, UNNEST/EXPLODE, PIVOT/UNPIVOT, JSON/array/table-valued relations, positional ordinality and SELECT star variants where the canonical contract proves exactness.
- [ ] #2 Generate finite, provable recursive CTE base/step witnesses with termination/cycle limits and valid row multiplicities; reject nontermination and unbounded or opaque function cases.
- [ ] #3 Preserve intermediate and physical boundary separation through table-expanding, pivot and recursively built producers.
- [ ] #4 Execute representative portable SQL fixtures with full output row identity, cardinality, data values and explicit unsupported paths.
- [ ] #5 Update dialect conformance matrix for each exact/fail-closed capability.
- [ ] #6 Document supported versus residual variants, add full-result differential tests and update docs/dialect-conformance.md for every changed feature.
<!-- AC:END -->
