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

SQL Semantic Protocol remains the complete semantic input. sql-tdg consumes the resolved domains,
schemas, relationships, layers, and outcomes and turns them into generated data. It does not inspect
or reinterpret the original SQL conditions.

## Acceptance Criteria

- [ ] Define one library-owned test-case representation shared by the CLI and integration tests.
- [ ] A test case records workload identity, selected target/layer boundary, dialect, deterministic seed, row counts, and generated relations.
- [ ] A test case distinguishes matching and deliberately non-matching generated rows.
- [ ] Expected query results are stored separately from the query under test so rerunning a changed query can detect changed behavior.
- [ ] The expected-result contract defines deterministic comparison semantics, including row ordering when the SQL result is order-sensitive.
- [ ] Re-approval/regeneration of expected results is an explicit action rather than an implicit side effect of verification.
- [ ] The contract supports both raw SQL workloads and dbt project workloads without separate semantic models.
- [ ] The contract supports testing from physical source relations or from selected intermediate relation boundaries.
- [ ] All generation inputs come from normalized SQL Semantic Protocol output; sql-tdg does not inspect SQL syntax or reconstruct predicate logic.
- [ ] Missing protocol information required for safe generation is treated as an upstream protocol blocker, not locally derived.
- [ ] Unit tests cover serialization/reproducibility of the test-case metadata.
- [ ] `make rust-checks` remains green.
