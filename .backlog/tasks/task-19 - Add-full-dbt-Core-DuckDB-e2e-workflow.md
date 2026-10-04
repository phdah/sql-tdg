---
id: TASK-19
title: Add full dbt Core DuckDB end-to-end workflow
status: To Do
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-2
dependencies:
  - TASK-18
---

## Description

Add a real dbt Core project as the milestone acceptance test.

Use DuckDB as the warehouse and take feature inspiration from the
`phdah/sql-semantic-protocol/tests/fixtures/dbt_core_project` project, including staging models,
derived columns, joins, aggregates, windows, subqueries, set operations, and independent graph
components.

The test must exercise the same public CLI a user would run. dbt's manifest/catalog artifacts are
translated through SQL Semantic Protocol, generated relations are loaded into DuckDB, dbt executes
the actual models, and terminal results are compared with approved expectations.

## Acceptance Criteria

- [ ] Add a self-contained dbt Core fixture project configured for DuckDB.
- [ ] The fixture includes sources plus multiple staged/intermediate/final models with non-trivial dependencies.
- [ ] The fixture covers the protocol project's representative features such as derived columns, joins, aggregates, windows, subqueries, set operations, and multiple final outcomes where supported.
- [ ] The workflow produces/consumes real dbt manifest.json and catalog.json artifacts rather than hand-written substitutes.
- [ ] sql-tdg consumes dbt semantics only through SQL Semantic Protocol's dbt adapter/contract.
- [ ] The CLI can generate DuckDB source data for a selected terminal dbt model and run the model graph.
- [ ] The CLI can instead generate data at an explicitly selected intermediate model boundary and test only the downstream graph.
- [ ] Generated dbt fixtures include both matching and deliberately rejected rows.
- [ ] Final model outputs are compared with approved full-result snapshots.
- [ ] A model predicate change that admits or removes rows causes verification to fail.
- [ ] The dbt end-to-end test is implemented as a Rust integration test that invokes the external dbt CLI; no Python test harness is introduced.
- [ ] Makefile targets own dbt end-to-end execution, and CI calls the Makefile rather than duplicating dbt commands in workflow YAML.
- [ ] The milestone acceptance path is documented in README with a runnable CLI example.
- [ ] `make rust-checks` and the dedicated dbt end-to-end target are green.
