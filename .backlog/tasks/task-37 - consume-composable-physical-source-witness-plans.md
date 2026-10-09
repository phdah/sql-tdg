---
id: TASK-37
title: Consume composable protocol witnesses with a global source solver
status: To Do
assignee: []
created_date: '2026-10-09'
updated_date: '2026-10-09'
labels: []
milestone: m-3
dependencies: 
  - TASK-43
references:
  - 'sql-semantic-protocol TASK-67'
  - 'sql-semantic-protocol TASK-68'
  - 'sql-semantic-protocol TASK-69'
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generator-side implementation for the single SQL Semantic Protocol 3.0.0 contract. This task must not reinterpret SQL, guess missing semantics or claim partial/local operator witnesses imply complete physical-source/output correctness. Validate with the pinned unpublished protocol release candidate before publication, then with the published crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 Consume typed canonical physical-source plans, constraints, row identities, duplicate counts, closed-world zero-count cases, and intermediate-boundary realization from protocol v3; no SQL parsing, AST inspection or operator inference.
- [ ] #2 Build a deterministic shared-source solver for positive and rejected witnesses with distinct relation instances, row/link/group membership, partial state effects and explicit typed feasibility/unsatisfiable/residual handling.
- [ ] #3 Generate complete physical relations, not independent per-operator samples; honor existing source schemas, keys, FK relationships, NOT NULL/accepted values, all relevant datatype and comparison restrictions.
- [ ] #4 Verify end-to-end combined WHERE+JOIN+GROUP BY+HAVING+QUALIFY+UNION with full DuckDB output snapshots and explicit refusal for unproven composition.
- [ ] #5 Keep solver, generator, table/export and protocol adapter responsibilities separate; update public API and CLI error reporting without fallback interpretation.
- [ ] #6 Document supported versus residual variants, add full-result differential tests and update docs/dialect-conformance.md for every changed feature.
<!-- AC:END -->
