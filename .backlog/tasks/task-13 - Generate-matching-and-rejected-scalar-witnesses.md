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

Generate both matching and deliberately non-matching rows directly from the resolved value domains
provided by SQL Semantic Protocol.

sql-tdg must not reason about WHERE clauses, boolean expression structure, or which SQL condition
produced a domain. Matching values are sampled from the provided allowed domain. For a rejected row,
the seeded generator selects one constrained generated column and samples a value outside its
provided allowed domain while generating the remaining columns normally.

If the protocol does not provide a sufficiently precise domain to construct a guaranteed matching
or rejected value, generation fails explicitly rather than reconstructing SQL semantics locally.

## Acceptance Criteria

- [ ] Callers can request deterministic counts or ratios of matching and rejected rows.
- [ ] Matching values are sampled only from the allowed protocol domain.
- [ ] Each rejected row deterministically selects one constrained generated column and samples outside that column's allowed protocol domain.
- [ ] Other generated columns continue to sample from their normal protocol-provided domains.
- [ ] Complement sampling supports bounded and disjoint ranges, inclusive/exclusive bounds, excluded values, finite sets, NULL semantics, and supported string/boolean domains as represented by the protocol.
- [ ] sql-tdg never inspects SQL predicates or reconstructs AND/OR expression semantics.
- [ ] Unknown, unbounded, empty, or otherwise insufficient domains fail explicitly when they cannot safely satisfy the requested classification.
- [ ] Selection of the rejected column/value is seeded and reproducible.
- [ ] Tests prove matching rows fall inside the selected protocol domains and rejected rows fall outside at least one selected protocol domain.
- [ ] `make rust-checks` remains green.
