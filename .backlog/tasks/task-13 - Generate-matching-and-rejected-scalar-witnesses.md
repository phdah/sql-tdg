---
id: TASK-13
title: Generate matching and rejected scalar witnesses
status: Done
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-2
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

## Progress

Implemented classified scalar generation directly from SQL Semantic Protocol value domains.
Matching rows sample only allowed values. Rejected rows deterministically select one safely
complementable constrained column and sample outside its protocol domain while every other column
continues to sample normally. Complement generation covers ranges, exclusions, finite sets,
booleans, strings, and nullable domains without inspecting SQL predicates.

## Acceptance Criteria

- [x] Callers can request deterministic counts or ratios of matching and rejected rows.
- [x] Matching values are sampled only from the allowed protocol domain.
- [x] Each rejected row deterministically selects one constrained generated column and samples outside that column's allowed protocol domain.
- [x] Other generated columns continue to sample from their normal protocol-provided domains.
- [x] Complement sampling supports bounded and disjoint ranges, inclusive/exclusive bounds, excluded values, finite sets, NULL semantics, and supported string/boolean domains as represented by the protocol.
- [x] sql-tdg never inspects SQL predicates or reconstructs AND/OR expression semantics.
- [x] Unknown, unbounded, empty, or otherwise insufficient domains fail explicitly when they cannot safely satisfy the requested classification.
- [x] Selection of the rejected column/value is seeded and reproducible.
- [x] Tests prove matching rows fall inside the selected protocol domains and rejected rows fall outside at least one selected protocol domain.
- [x] `make rust-checks` remains green.
