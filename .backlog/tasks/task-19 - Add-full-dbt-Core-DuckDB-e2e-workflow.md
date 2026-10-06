---
id: TASK-19
title: Add full dbt Core DuckDB end-to-end workflow
status: Done
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-2
dependencies:
  - TASK-18
---

## Description

Add a real dbt Core project as the milestone acceptance test.

Use DuckDB as the test warehouse and take feature inspiration from the
`phdah/sql-semantic-protocol/tests/fixtures/dbt_core_project` project, including staging models,
derived columns, joins, aggregates, windows, subqueries, set operations, and independent graph
components.

The test exercises the same generation CLI a user would run. dbt's manifest/catalog artifacts are
translated through SQL Semantic Protocol, sql-tdg generates portable source data, and the test
harness places generated CSV files into the fixture's seed path and runs `dbt seed` before dbt
executes the actual model graph. Parquet export is tested separately as the lossless general-purpose
format; native dbt seeds are CSV.

## Acceptance Criteria

- [x] Add a self-contained dbt Core fixture project configured for DuckDB.
- [x] The fixture includes sources plus multiple staged/intermediate/final models with non-trivial dependencies.
- [x] The fixture covers derived columns, joins, aggregates, windows, subqueries, set operations, and multiple final outcomes where supported.
- [x] The workflow produces/consumes real dbt manifest.json and catalog.json artifacts rather than hand-written substitutes.
- [x] sql-tdg consumes dbt semantics only through SQL Semantic Protocol's dbt adapter/contract.
- [x] The CLI generates source relation files for a selected terminal dbt model without connecting to DuckDB.
- [x] Generated CSV relation files are loaded into the test warehouse using native `dbt seed`.
- [x] The workflow can instead generate data at an explicitly selected intermediate model boundary and test only the downstream graph.
- [x] Generated dbt fixtures include both matching and deliberately rejected rows.
- [x] Final model outputs are compared with approved full-result snapshots inside test support.
- [x] A model predicate change that admits or removes rows causes verification to fail.
- [x] The dbt end-to-end test is implemented as a Rust integration test that invokes the external dbt CLI; no Python test harness is introduced.
- [x] Makefile targets own dbt end-to-end execution, and CI calls the Makefile rather than duplicating dbt commands in workflow YAML.
- [x] The milestone acceptance path is documented in README with a runnable CLI example.
- [x] `make rust-checks` and the dedicated dbt end-to-end target are green.
