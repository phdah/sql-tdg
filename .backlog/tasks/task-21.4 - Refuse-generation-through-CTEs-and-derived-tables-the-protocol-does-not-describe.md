---
id: TASK-21.4
title: >-
  Refuse generation through CTEs and derived tables the protocol does not
  describe
status: Done
assignee:
  - '@opencode'
created_date: '2026-10-06 12:39'
updated_date: '2026-10-06 12:41'
labels: []
milestone: m-2
dependencies: []
parent_task_id: TASK-21
priority: high
type: bug
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
sql-semantic-protocol 1.0.1 drops the joins, WHERE/HAVING predicates, and column lineage inside CTE bodies and derived tables (it only collects their physical dependencies) while still reporting resolved composed semantics with no diagnostics. sql-tdg therefore silently generates rows that violate the query: `with x as (select a from t where a > 1000) select a from x` yields a = -2147483648, 0, 2147483647, and the maintainer's dbt model `daily_revenue` (join inside a CTE) gets uncoordinated join keys. This violates the never-over-claim principle. Until the protocol carries these semantics, sql-tdg must detect query sources that are local relations (not a physical dependency of the statement and not a relation produced by another layer) in every layer it generates from, and fail with an explicit error naming the layer/outcome and the local relation. sql-tdg must not inspect SQL text to do this.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Generation from a query whose source is a CTE fails with an explicit error naming the local relation
- [x] #2 Generation from a query whose source is a derived table fails with an explicit error
- [x] #3 The check applies to the target layer and every ancestor layer used for physical-source, intermediate-boundary, and all-outcomes generation
- [x] #4 Queries reading only physical or in-bundle produced relations are unaffected
- [x] #5 Tests cover CTE, derived table, ancestor-layer, and whole-project cases
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. In `src/protocol.rs` add `reject_local_relation_sources(bundle, layer_id)`: for the layer and all `ancestor_layer_ids`, for each Query statement, every `query.sources()` name must be in `query.dependencies()` or be the relation of a graph edge consumed by that layer. Otherwise return `UnsupportedSemantics { layer_id, code: "local_relation_semantics", message }` naming the source.
2. Call it in `generate_classified_from_bundle_at_boundary` for the selected layer (physical and intermediate boundaries) and in `prepare_outcome` for all-outcomes mode (so the error is wrapped with the outcome name).
3. Tests in tests/full_query.rs or a new focused test file: CTE, derived table, ancestor view reading a CTE, whole-project naming the outcome; existing suites must pass unchanged. Run `make rust-checks` and `make dbt-e2e`.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Evidence: tests/local_relations.rs (CTE, derived table, ancestor CTE for physical + intermediate boundary, whole-project TerminalOutcome naming `relation:nested`, physical/produced sources unaffected). `make rust-checks` and `make dbt-e2e` pass. Maintainer daily_revenue model now fails with `local_relation_semantics: source "revenue_by_day" is a CTE or derived table` instead of generating uncoordinated join keys.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added `reject_local_relation_sources` in src/protocol.rs. For the selected layer and all ancestor layers, every query source must be a physical dependency or a relation produced by another layer. Otherwise generation returns `UnsupportedSemantics` with code `local_relation_semantics`, naming the CTE or derived table. It is called for single-outcome generation (physical and intermediate boundaries) and per outcome in whole-project mode, where the error is wrapped with the outcome name. This closes a silent over-claim: protocol 1.0.1 drops joins, predicates, and lineage inside local relations while still reporting resolved semantics. Real support is tracked in TASK-21.5 and needs a protocol feature. Tests: tests/local_relations.rs; rust-checks and dbt-e2e pass.
<!-- SECTION:FINAL_SUMMARY:END -->
