---
id: TASK-36
title: dbt fixture testing gate for milestone m-3
status: To Do
assignee: []
created_date: '2026-10-09 15:56'
updated_date: '2026-10-09 15:56'
labels:
  - dbt
  - e2e
  - ci
milestone: m-3
dependencies:
  - TASK-24
  - TASK-25
  - TASK-26
  - TASK-27
  - TASK-28
  - TASK-29
  - TASK-30
  - TASK-31
  - TASK-35
references:
  - TASK-21
  - TASK-19
  - tests/fixtures/dbt_core_project/Makefile
  - tests/fixtures/dbt_core_project/
  - tests/dbt_core_e2e.rs
  - Makefile (dbt-e2e target)
  - .github/workflows/rust-checks.yml
priority: high
type: task
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Final testing gate owned by the maintainer before milestone m-3 ("Broaden exact SQL test-data generation") is closed, following the TASK-21 precedent for m-2.

The gate is the demo workflow in `tests/fixtures/dbt_core_project/Makefile`: `make all` bootstraps the dbt project, generates one physical-source dataset for the whole project with sql-tdg defaults (100 matching and 10 rejected rows per relation, no --target), loads the generated CSVs as dbt seeds, runs every dbt model in DuckDB and reports row counts for the sources and the models. The goal is that this single command proves the m-3 features work together on a realistic dbt project, and that CI enforces it.

Current state (2026-10-09):
- `make all` stops at generation: whole-project mode refuses rejected rows (TASK-35) and the fixture models aggregate_summary (HAVING, TASK-28), ranked_orders (QUALIFY, TASK-29), subquery_orders (EXISTS, TASK-26) and unioned_orders (UNION, TASK-24) cannot yet be generated.
- `verify` and `results` only print counts. `make all` exits 0 regardless of whether the counts are correct, so it cannot yet serve as a sign-off.
- The fixture has no models exercising extended joins and self-joins (TASK-25), computed and correlated predicates (TASK-27), output cardinality goals (TASK-30, partially visible through counts only) or DML state transitions (TASK-31).
- The Rust dbt e2e suite (`make dbt-e2e`, tests/dbt_core_e2e.rs) runs on the same fixture and asserts that whole-project mode fails on aggregate_summary with reason=having; that assertion becomes obsolete when TASK-28 lands.
- CI installs dbt-core and dbt-duckdb (tests/requirements-dbt-e2e.txt) but not the DuckDB CLI the Makefile uses.

Open decision (maintainer, before the fixture is extended): how TASK-31 (INSERT/UPDATE/DELETE/MERGE state transitions) is covered: through a dbt pattern in this fixture (for example incremental models or snapshots) or verified outside the dbt gate with the reason recorded here.

Expected outcomes for row-preserving models and for aggregate or grouped models (where output rows differ from source rows) must follow the output-cardinality contract defined by TASK-30 rather than ad hoc numbers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The maintainer's decision on how TASK-31 is covered is recorded in this task
- [ ] #2 The dbt fixture contains models exercising every m-3 feature (set operations, extended joins and self-joins, EXISTS/correlated subqueries, computed and correlated predicates, grouped aggregates with HAVING, window ranks with QUALIFY, output cardinality) or this task records each feature explicitly excluded from the dbt gate with the maintainer's approval
- [ ] #3 `make all` exits non-zero when any source relation, model output or rejected-row expectation does not hold
- [ ] #4 Expected matching and rejected outcomes are derived from the generated metadata.sqltdg and the TASK-30 cardinality contract, not hard-coded counts
- [ ] #5 Rejected rows are shown to be absent from the terminal model outputs they are rejected by
- [ ] #6 Running the gate leaves the fixture unchanged after `make clean`, and the Rust dbt e2e suite still passes afterwards
- [ ] #7 The Rust dbt e2e assertions about unsupported models are updated to match the supported m-3 features
- [ ] #8 CI runs the gate with the pinned dbt versions and the DuckDB CLI, independently of the existing checks
- [ ] #9 README or docs/usage.md describe how to run the dbt demo gate locally
- [ ] #10 The maintainer signs off on the gate before milestone m-3 is closed
- [ ] #11 `make all` with default variables (100 matching, 10 rejected, no target) runs end to end on the committed fixture without disabling models or passing extra arguments
<!-- AC:END -->
