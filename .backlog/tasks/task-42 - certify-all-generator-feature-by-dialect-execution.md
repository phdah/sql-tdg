---
id: TASK-42
title: Certify complete generator feature-by-dialect and cross-feature execution matrix
status: To Do
assignee: []
created_date: '2026-10-09'
updated_date: '2026-10-09'
labels: []
milestone: m-3
dependencies: 
  - TASK-24
  - TASK-25
  - TASK-26
  - TASK-27
  - TASK-28
  - TASK-29
  - TASK-30
  - TASK-31
  - TASK-35
  - TASK-37
  - TASK-38
  - TASK-39
  - TASK-40
  - TASK-41
  - TASK-43
references:
  - 'sql-semantic-protocol TASK-66'
  - 'sql-semantic-protocol TASK-88'
  - 'sql-semantic-protocol TASK-89'
  - 'TASK-33'
  - 'TASK-36'
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generator-side implementation for the single SQL Semantic Protocol 3.0.0 contract. This task must not reinterpret SQL, guess missing semantics or claim partial/local operator witnesses imply complete physical-source/output correctness. Validate with the pinned unpublished protocol release candidate before publication, then with the published crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 Consume the protocol's release-scope machine-readable feature matrix; mirror every release-blocking feature+variant and its supported/fail-closed proof status in sql-tdg tests.
- [ ] #2 Track parsing, protocol exactness, generator constructive success, target-engine execution and documented exclusions independently across the 13 dialect families.
- [ ] #3 Run portable DuckDB raw SQL and dbt E2E positive/rejected cases spanning nested sets, windows, joins, CTEs, grouping, subqueries and mixed DML/DDL scripts.
- [ ] #4 Use actual per-dialect execution engines where provisioned; state explicitly when only parsing or contract tests exist and never certify unexecuted dialect behavior.
- [ ] #5 Fail CI on wrong full result snapshots, falsely exact unsupported cases, missing rows/rejections or unreviewed coverage-manifest entries; provide reproducible failure artifacts.
- [ ] #6 Document supported versus residual variants, add full-result differential tests and update docs/dialect-conformance.md for every changed feature.
<!-- AC:END -->
