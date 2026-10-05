---
id: TASK-17
title: Add complete CLI workflow
status: Done
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

## Progress

Added an installable, dependency-free `sql-tdg generate` binary that delegates semantic analysis
and generation to the existing library and SQL Semantic Protocol. Raw SQL accepts inline/file inputs
plus explicit typed source schemas. dbt workflows consume real manifest/catalog artifacts through
the protocol's dbt adapter, either from explicit artifact paths or a project's `target/` directory.

The CLI exposes terminal target selection, physical/intermediate generation boundaries, deterministic
seed and classified row counts, and CSV/Parquet export. Each run writes backend-neutral relation
files plus stable `metadata.sqltdg` reproducibility metadata. Production CLI code never connects to
or mutates a database. Compiled-binary integration tests cover Parquet, CSV, and actionable failure
behavior, and README documents raw SQL and dbt workflows.

## Acceptance Criteria

- [x] The package exposes an installable `sql-tdg` binary while retaining the library API.
- [x] CLI inputs support raw SQL strings/files and protocol-backed dbt artifact/project workflows.
- [x] CLI options expose dialect, target outcome, source/intermediate boundary, seed, matching row count, rejected row count, output directory, and output format.
- [x] A generate command produces deterministic relation files plus reproducibility metadata.
- [x] CSV and Parquet are supported output formats, with Parquet preferred where Arrow types cannot be represented losslessly in CSV.
- [x] The CLI never opens, creates, seeds, mutates, or verifies a user database or warehouse.
- [x] CLI output clearly reports generated relations, positive/rejected row counts, selected target, seed, format, and output paths.
- [x] Invalid or unsupported semantics return non-zero exit codes with actionable errors.
- [x] Argument parsing and filesystem I/O remain thin wrappers around library APIs.
- [x] CLI integration tests run the compiled binary rather than only invoking internal functions.
- [x] README documents copy-pastable raw SQL and dbt generation/export workflows.
- [x] `make rust-checks` remains the source of truth for normal Rust CI.
