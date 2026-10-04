---
id: TASK-15
title: Generate at source and intermediate layer boundaries
status: To Do
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-1
dependencies:
  - TASK-14
---

## Description

Allow callers to choose where in a transformation graph generated data is materialized.

A full-chain test should generate physical leaf/source relations and execute every transformation to
the selected target. An isolated downstream test should instead be able to materialize generated
data for selected intermediate relations and execute only the downstream portion.

Use SQL Semantic Protocol graph, layer, composed semantics, source schemas, and output schemas to
plan these boundaries. Do not flatten every test into terminal source generation.

## Acceptance Criteria

- [ ] The library can generate physical source relations for a selected terminal outcome.
- [ ] The library can generate data for explicitly selected intermediate produced relations using their protocol output schema/domain semantics.
- [ ] A downstream target can be tested from its immediate upstream relation boundary without materializing unrelated ancestors.
- [ ] Multiple independent graph components require explicit target selection when ambiguous.
- [ ] Generated relation names preserve exact qualified protocol identities.
- [ ] Source schemas and intermediate output schemas are validated before generation.
- [ ] Matching/rejected classification remains valid at the selected boundary.
- [ ] Unsupported partial-write semantics such as ambiguous post-MERGE state fail explicitly unless a safe materialization contract exists.
- [ ] Integration tests cover the same downstream query once from raw sources and once from an intermediate boundary.
- [ ] `make rust-checks` remains green.
