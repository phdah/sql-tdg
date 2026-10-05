---
id: TASK-15
title: Generate at source and intermediate layer boundaries
status: Done
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-2
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

## Progress

Added explicit graph-boundary generation on top of SQL Semantic Protocol. Physical-source generation
remains the default. Intermediate generation materializes an exact immediate-upstream cut for a
selected terminal target, validates the declared intermediate schemas against producer outputs, and
intersects producer output domains with downstream domains. Rejected scalar rows remain valid for
the producer while violating downstream constraints, and supported inner-equality relationships
retain matching/rejected classification at the intermediate boundary. Incomplete cuts, unresolved
dependencies, ambiguous targets, and partial-write producers fail explicitly.

## Acceptance Criteria

- [x] The library can generate physical source relations for a selected terminal outcome.
- [x] The library can generate data for explicitly selected intermediate produced relations using their protocol output schema/domain semantics.
- [x] A downstream target can be tested from its immediate upstream relation boundary without materializing unrelated ancestors.
- [x] Multiple independent graph components require explicit target selection when ambiguous.
- [x] Generated relation names preserve exact qualified protocol identities.
- [x] Source schemas and intermediate output schemas are validated before generation.
- [x] Matching/rejected classification remains valid at the selected boundary.
- [x] Unsupported partial-write semantics such as ambiguous post-MERGE state fail explicitly unless a safe materialization contract exists.
- [x] Integration tests cover the same downstream query once from raw sources and once from an intermediate boundary.
- [x] `make rust-checks` remains green.
