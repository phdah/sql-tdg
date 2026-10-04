---
id: TASK-14
title: Generate relational positive and negative witnesses
status: To Do
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-1
dependencies:
  - TASK-13
---

## Description

Extend positive and negative generation across multiple relations so joins, correlated predicates,
subqueries, set membership, and other relationship semantics are tested with coordinated data.

Rows that should join or satisfy an EXISTS/IN-style relationship must have coordinated keys and
values. Negative witnesses must break the relevant relationship deliberately while preserving other
compatible constraints so the workload contains meaningful rows that should be excluded.

## Acceptance Criteria

- [ ] Generate coordinated matching keys and values for supported inner/outer/semi-style join semantics represented by the protocol.
- [ ] Generate deterministic non-matching join witnesses by breaking a selected relationship while keeping unrelated compatible constraints valid.
- [ ] Support positive and negative witnesses for EXISTS, NOT EXISTS, IN, NOT IN, and correlated subquery semantics when represented by the protocol.
- [ ] Support relation-level witnesses needed by INTERSECT, EXCEPT, UNION/UNION ALL, and DISTINCT test cases where meaningful.
- [ ] Multi-table generation plans are deterministic and independent of unordered map/set iteration.
- [ ] The generator never infers relationship semantics from SQL text.
- [ ] Unknown or insufficient relationship semantics return explicit errors.
- [ ] Tests cover multi-table matching and rejected rows and prove the expected relationship behavior by executing representative queries.
- [ ] `make rust-checks` remains green.
