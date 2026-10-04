---
id: TASK-14
title: Generate relational positive and negative witnesses
status: To Do
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

## Acceptance Criteria

- [ ] Generate coordinated matching values for supported cross-relation relationships represented by the protocol.
- [ ] Generate deterministic non-matching relation rows by breaking one selected protocol-provided relationship where doing so guarantees a non-match.
- [ ] Multi-table generation uses protocol relation identities, schemas, domains, lineage, and relationship metadata directly.
- [ ] Multi-table generation plans are deterministic and independent of unordered map/set iteration.
- [ ] sql-tdg never infers join, subquery, set-membership, or other relational semantics from SQL text.
- [ ] Unknown or insufficient protocol relationship semantics return explicit errors rather than triggering local semantic analysis.
- [ ] Tests execute representative multi-table queries and prove matching and deliberately non-matching generated data behaves as classified.
- [ ] `make rust-checks` remains green.
