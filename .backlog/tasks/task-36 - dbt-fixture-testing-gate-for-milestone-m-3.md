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
  - TASK-37
  - TASK-38
  - TASK-39
  - TASK-40
  - TASK-41
  - TASK-42
  - TASK-43
  - TASK-44
references:
  - 'sql-semantic-protocol TASK-66'
  - 'sql-semantic-protocol TASK-89'
  - 'sql-semantic-protocol TASK-91'
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

**Maintainer-approved decision (2026-10-09):** use **both** dbt-native workflows (including incremental models and snapshots where relevant) and a **required companion scripted DuckDB DML/DDL transition harness**, invoked by the **same** committed fixture `make all`. This does not permit skipping DDL/MERGE/INSERT cases because a dbt model DAG alone cannot express them. The single CI acceptance gate fails if either portion fails.

Expected outcomes for row-preserving models and for aggregate or grouped models (where output rows differ from source rows) must follow the output-cardinality contract defined by TASK-30 rather than ad hoc numbers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The maintainer's decision on how TASK-31 is covered is recorded in this task
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
- [ ] #12 The final gate must run against the pinned protocol v3 candidate *before* its release, cover the complete approved feature matrix, assert complete model values/counts/negative membership, and sign off protocol TASK-91 before release PR #79 is merged.
- [ ] #13 The committed dbt fixture `make all` performs a mandatory scripted DuckDB DDL/DML E2E with exact pre/post snapshots (CREATE/REPLACE/ALTER/DROP, INSERT/UPDATE/DELETE/MERGE/UPSERT, transactions, conflict branches) plus feasible native dbt incremental and model DAG cases; failure anywhere makes the single gate red.
- [ ] #14 Seeded randomized per-terminal negative predicates/columns cover **every protocol-proven alternative across multiple seeds** and all terminal absence vectors agree with independently executed SQL; fixed-seed output remains reproducible.
- [ ] #15 Every approved supported dialect/feature pair is parsed and semantically compared to its canonical equivalent in CI, with all 13 exposed dialects checked and DuckDB used only for executable equivalent transformations. Evidence is recorded in the upstream and downstream conformance matrices.
<!-- AC:END -->

## Protocol v3 release gate (2026-10-09)

**Release dependency:** This task must be validated against the upstream single 3.0.0 release candidate pinned by sql-tdg TASK-43; the protocol release remains blocked on upstream TASK-91. Original criteria remain binding. Do not mark Done using only an operator-local proof where the physical-source DAG cannot realize it. Fully execute SQL to verify terminal rows and deliberately rejected cases, and document supported/residual variants in the feature-by-dialect matrix. Companion planning PR: https://github.com/phdah/sql-semantic-protocol/pull/87.

## Approved E2E scope (2026-10-09)

The maintainer approved one **unified**, mandatory and repeatable `make all` gate. It covers the dbt DAG plus scripted DuckDB DML/DDL, not a separate optional sign-off. The repo's existing fixture Makefile currently only runs dbt models and prints counts; it **does not yet satisfy** this decision. Extend verification to full values, bag multiplicities, physical-source pre/post state, constraints and per-terminal negatives, with deterministic seeds and varied failing columns. Follow [docs/m3-acceptance-plan.md](../../docs/m3-acceptance-plan.md) and [protocol scope](https://github.com/phdah/sql-semantic-protocol/blob/feat/task-66-dialect-feature-inventory/docs/coverage-signoff.md).
