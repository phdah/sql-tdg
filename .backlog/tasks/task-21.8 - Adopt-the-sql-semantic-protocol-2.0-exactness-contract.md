---
id: TASK-21.8
title: Adopt the sql-semantic-protocol 2.0 exactness contract
status: To Do
assignee: []
created_date: '2026-10-08 11:36'
updated_date: '2026-10-08 11:36'
labels: []
milestone: m-2
dependencies: []
references:
  - sql-semantic-protocol TASK-43 (exactness contract)
  - sql-semantic-protocol TASK-45 (join equalities)
  - sql-semantic-protocol TASK-53 (comparison-semantics assumptions)
  - sql-semantic-protocol TASK-54 (residual reasons)
  - src/protocol.rs
  - src/main.rs
parent_task_id: TASK-21
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
sql-semantic-protocol 2.0.0 (unreleased at the time of writing; verified at protocol commit 3a4d3c6) makes the generation guarantee explicit. sql-tdg does not consume it yet, so today it still generates rows that violate supported-looking queries: cross-column OR, LIKE, CAST or function predicates, HAVING, QUALIFY, LIMIT/OFFSET/FETCH, and TABLESAMPLE all produced violating or unconstrained rows in manual probes. The protocol now flags every one of these. sql-tdg only checks per-column Unknown domains and composed diagnostics, so it never sees the flag.

Protocol 2.0 surfaces consumed by this task:
- `ResolvedComposedSemantics::condition_exactness()` (and the same on each query scope), with status `exact`, `conditional`, or `residual`:
  - `exact`: the column domains plus join equalities exactly decide which source-row combinations qualify.
  - `conditional`: exact only under the listed comparison-semantics assumptions (`binary_collation`, `no_char_padding`, `no_nan`, `signed_zero_equivalent`, `session_time_zone`).
  - `residual`: lists conditions that are not represented, each with a stable reason and clause.
- `ResolvedComposedSemantics::join_equalities()`: every inner equality relationship on physical columns with relation instance, join kind, and originating layer, including joins inside CTEs, derived tables, implicit WHERE joins, and upstream layers.
- Caller-declared comparison assumptions (`AnalysisBundle::declare_comparison_assumptions`, CLI and analysis-manifest equivalents in the protocol), recorded in the emitted bundle.

Decision (proposed; confirm with the maintainer when taking the task): following "never over-claim", `conditional` semantics generate only when the caller declares the required assumptions through sql-tdg (CLI option and library parameter passed through to the protocol). Otherwise generation fails naming the open assumptions and the conditions that depend on them.

Removing the CTE guard and CTE end-to-end coverage belong to TASK-21.5; this task adopts the contract everywhere else.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 sql-tdg depends on the published sql-semantic-protocol 2.x release instead of a path dependency, and make rust-checks and make dbt-e2e pass
- [ ] #2 Generation for a selected outcome, for every terminal outcome in whole-project mode, and at intermediate boundaries fails with an explicit error naming each residual reason, clause, and originating layer whenever the relevant exactness status is residual
- [ ] #3 Conditional semantics generate only when every listed assumption is declared by the caller; otherwise generation fails naming the open assumptions and their dependent conditions
- [ ] #4 The CLI and library accept comparison-semantics assumptions, pass them to the protocol, and record them in metadata.sqltdg
- [ ] #5 Relational generation coordinates keys from join_equalities rather than walking per-layer join trees, and still refuses outer joins and repeated relation instances explicitly
- [ ] #6 Rejected-row generation is allowed only for exact semantics (including declared assumptions)
- [ ] #7 Tests assert explicit errors for cross-column OR, LIKE, CAST, HAVING, QUALIFY, LIMIT, OFFSET, FETCH, TABLESAMPLE, unknown schema columns, and lossy literal coercion
- [ ] #8 Tests cover conditional string, float, and timestamp filters with and without declared assumptions, and the dbt end-to-end fixture runs with its required assumptions declared
- [ ] #9 README documents exactness handling, assumption declaration, and the resulting errors
- [ ] #10 Existing fixtures that rely on semantics the protocol now marks residual (for example tests/fixtures window_ranked.sql with QUALIFY and LIMIT) are rewritten or converted to explicit-error tests, never kept passing by ignoring exactness
<!-- AC:END -->
