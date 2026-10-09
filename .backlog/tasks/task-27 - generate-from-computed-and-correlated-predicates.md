---
id: TASK-27
title: Generate from computed and correlated predicates
status: To Do
assignee: []
created_date: '2026-10-08'
labels: []
milestone: m-3
dependencies:
  - TASK-37
  - TASK-40
  - TASK-43

references:
  - 'sql-semantic-protocol TASK-70'
  - 'sql-semantic-protocol TASK-79'
  - 'sql-semantic-protocol TASK-63'
  - 'TASK-13'
  - 'TASK-21.8'
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The generator cannot currently satisfy source-row membership for computed comparisons, LIKE and cross-column boolean expressions. Extend the solver to consume safe canonical constraints from upstream TASK-63.

This is post-1.0 feature work, not a claim about existing CLI behavior. Canonical semantics, dialect resolution, lineage and SQL analysis belong solely to SQL Semantic Protocol. If upstream semantics are missing, implement and release the protocol prerequisite first; do not add interpretation fallbacks in sql-tdg. Preserve deterministic outputs and exactness errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Consume typed protocol constraints for scoped computed comparisons, safe pattern cases, and cross-column OR/AND correlations rather than sampling column intervals independently.
- [ ] #2 Preserve datatypes, collation, NULL, overflow and comparison assumptions and deterministic seed behavior.
- [ ] #3 Guarantee matching and rejected witnesses with appropriate coupling across columns, or fail explicitly for unsatisfiable/noninvertible conditions.
- [ ] #4 Run SQL-level DuckDB comparisons for a matrix of expressions, casts, prefix patterns and cross-column disjunctions.
- [ ] #5 Update the feature matrix and provide actionable errors where protocol exactness is absent.
- [ ] #6 Cover full reviewed predicate families and typed correlations across filters, joins and projected expressions, including computed source dependencies without independent Cartesian sampling.
<!-- AC:END -->

## Protocol v3 release gate (2026-10-09)

**Release dependency:** This task must be validated against the upstream single 3.0.0 release candidate pinned by sql-tdg TASK-43; the protocol release remains blocked on upstream TASK-91. Original criteria remain binding. Do not mark Done using only an operator-local proof where the physical-source DAG cannot realize it. Fully execute SQL to verify terminal rows and deliberately rejected cases, and document supported/residual variants in the feature-by-dialect matrix. Companion planning PR: https://github.com/phdah/sql-semantic-protocol/pull/87.
