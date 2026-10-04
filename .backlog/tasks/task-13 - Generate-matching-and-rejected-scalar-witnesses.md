---
id: TASK-13
title: Generate matching and rejected scalar witnesses
status: To Do
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-1
dependencies:
  - TASK-11
  - TASK-12
---

## Description

Generate both rows that satisfy scalar query semantics and rows that are deliberately rejected by
them.

Rejected rows should be useful counterexamples, not arbitrary garbage. For conjunctive predicates,
the seeded generator should choose one supported condition to violate while satisfying the remaining
conditions whenever that is possible. For disjunctions, violating only one branch is insufficient;
the rejection plan must falsify all alternatives required for the row to be excluded.

## Acceptance Criteria

- [ ] Callers can request deterministic counts or ratios of matching and rejected rows.
- [ ] Every matching row satisfies all supported scalar constraints represented by the selected protocol semantics.
- [ ] For conjunctions, each rejected row deterministically selects one supported rejection target and violates it while satisfying the other compatible conditions.
- [ ] For disjunctions, rejection plans falsify every branch required to make the complete predicate false rather than naively violating one branch.
- [ ] Rejection plans cover bounded/disjoint ranges, inclusive/exclusive bounds, excluded values, equality/inequality, finite sets, NULL semantics, and supported string/boolean constraints.
- [ ] Generated rows retain metadata identifying whether they are expected to match and, for rejected rows, which rejection plan was chosen.
- [ ] Unsatisfiable requests fail explicitly instead of emitting rows whose classification is uncertain.
- [ ] Selection of rejection plans is seeded and reproducible.
- [ ] Tests prove that matching rows satisfy the protocol domains and rejected rows fall outside the intended acceptance semantics.
- [ ] `make rust-checks` remains green.
