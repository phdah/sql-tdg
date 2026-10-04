---
id: TASK-14
title: Generate relational positive and negative witnesses
status: Done
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-2
dependencies:
  - TASK-13
---

## Description

Extend protocol-driven generation across multiple relations.

SQL Semantic Protocol already owns join, lineage, dependency, and relationship semantics. sql-tdg
only consumes those resolved relationships to coordinate generated values across tables. It must
not inspect JOIN/EXISTS/IN syntax or derive its own relational semantics.

Matching data satisfies the protocol-provided relationships. Deliberately non-matching data is
created by selecting one protocol-provided relationship that can safely be broken and generating
values outside that relationship while otherwise following the supplied generation constraints.

## Progress

Implemented deterministic relational witnesses for protocol-resolved inner equality relationships.
Join keys are mapped to physical source columns through protocol dependency edges and composed
lineage, then coordinated across connected relations. Rejected rows break one safely isolatable
relationship while preserving scalar domains and all other supplied relationships. Composite
equality joins are supported; unsupported, ambiguous, incompatible, or unbreakable relationship
shapes fail explicitly.

## Acceptance Criteria

- [x] Generate coordinated matching values for supported cross-relation relationships represented by the protocol.
- [x] Generate deterministic non-matching relation rows by breaking one selected protocol-provided relationship where doing so guarantees a non-match.
- [x] Multi-table generation uses protocol relation identities, schemas, domains, lineage, and relationship metadata directly.
- [x] Multi-table generation plans are deterministic and independent of unordered map/set iteration.
- [x] sql-tdg never infers join, subquery, set-membership, or other relational semantics from SQL text.
- [x] Unknown or insufficient protocol relationship semantics return explicit errors rather than triggering local semantic analysis.
- [x] Tests execute representative multi-table queries and prove matching and deliberately non-matching generated data behaves as classified.
- [x] `make rust-checks` remains green.
