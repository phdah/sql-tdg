---
id: TASK-17
title: Add complete CLI workflow
status: To Do
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-2
dependencies:
  - TASK-16
---

## Description

Add a thin production CLI on top of the library workflow.

The CLI should let a user create, inspect, execute, verify, and explicitly approve deterministic SQL
test cases without embedding SQL analysis, solving, generation, or DuckDB semantics in argument
handling code.

## Acceptance Criteria

- [ ] The package exposes an installable `sql-tdg` binary while retaining the library API.
- [ ] CLI inputs support raw SQL strings/files and protocol-backed dbt artifact/project workflows.
- [ ] CLI options expose dialect, target outcome, source/intermediate boundary, seed, matching row count, rejected row count, and output location.
- [ ] A generate command creates the deterministic test database and test-case metadata.
- [ ] An approve command explicitly records expected result snapshots.
- [ ] A verify command executes the current workload against the existing generated case and fails on result differences.
- [ ] CLI output clearly reports generated relations, positive/rejected row counts, selected target, seed, database path, and verification status.
- [ ] Invalid or unsupported semantics return non-zero exit codes with actionable errors.
- [ ] Argument parsing and filesystem I/O remain thin wrappers around library APIs.
- [ ] CLI integration tests run the compiled binary rather than only invoking internal functions.
- [ ] README documents copy-pastable raw SQL and dbt workflows.
- [ ] `make rust-checks` remains the source of truth for normal Rust CI.
