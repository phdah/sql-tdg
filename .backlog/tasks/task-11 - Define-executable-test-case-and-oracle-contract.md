---
id: TASK-11
title: Define executable test-case and oracle contract
status: To Do
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-1
dependencies:
  - TASK-10
---

## Description

Define the durable test-case model used by the CLI, generator, DuckDB execution harness, raw SQL
fixtures, and dbt fixtures.

A generated test case must preserve enough information to reproduce the database exactly and to
verify behavior without regenerating or silently approving a changed query. The approved expected
result is therefore part of the test case and is distinct from the current query text being tested.

The contract must also define how positive and negative generation requests are represented. SQL
Semantic Protocol is the only source of query semantics. If independent predicate/relation semantics
needed for negative generation are not present in the protocol, record the missing protocol
capability and implement it upstream before local generation code depends on it.

## Acceptance Criteria

- [ ] Define one library-owned test-case representation shared by the CLI and integration tests.
- [ ] A test case records workload identity, selected target/layer boundary, dialect, deterministic seed, row counts, and generated relations.
- [ ] A test case distinguishes matching rows, deliberately non-matching rows, and their rejection reason/plan.
- [ ] Expected query results are stored separately from the query under test so rerunning a changed query can detect changed behavior.
- [ ] The expected-result contract defines deterministic comparison semantics, including row ordering when the SQL result is order-sensitive.
- [ ] Re-approval/regeneration of expected results is an explicit action rather than an implicit side effect of verification.
- [ ] The contract supports both raw SQL workloads and dbt project workloads without separate semantic models.
- [ ] The contract supports testing from physical source relations or from selected intermediate relation boundaries.
- [ ] Negative-generation inputs come only from normalized SQL Semantic Protocol semantics; sql-tdg does not inspect SQL syntax.
- [ ] Missing protocol semantics required for rejection plans are treated as an upstream protocol blocker, not locally re-derived.
- [ ] Unit tests cover serialization/reproducibility of the test-case metadata.
- [ ] `make rust-checks` remains green.
