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

Add a thin production CLI on top of the backend-neutral generation library.

The CLI generates deterministic test data from raw SQL or protocol-backed dbt artifacts and writes
portable outputs. It does not connect to databases, execute workloads, seed warehouses, approve
query results, or verify backend state. Users own loading the generated files into their target
system.

## Acceptance Criteria

- [ ] The package exposes an installable `sql-tdg` binary while retaining the library API.
- [ ] CLI inputs support raw SQL strings/files and protocol-backed dbt artifact/project workflows.
- [ ] CLI options expose dialect, target outcome, source/intermediate boundary, seed, matching row count, rejected row count, output directory, and output format.
- [ ] A generate command produces deterministic relation files plus reproducibility metadata.
- [ ] CSV and Parquet are supported output formats, with Parquet preferred where Arrow types cannot be represented losslessly in CSV.
- [ ] The CLI never opens, creates, seeds, mutates, or verifies a user database or warehouse.
- [ ] CLI output clearly reports generated relations, positive/rejected row counts, selected target, seed, format, and output paths.
- [ ] Invalid or unsupported semantics return non-zero exit codes with actionable errors.
- [ ] Argument parsing and filesystem I/O remain thin wrappers around library APIs.
- [ ] CLI integration tests run the compiled binary rather than only invoking internal functions.
- [ ] README documents copy-pastable raw SQL and dbt generation/export workflows.
- [ ] `make rust-checks` remains the source of truth for normal Rust CI.
