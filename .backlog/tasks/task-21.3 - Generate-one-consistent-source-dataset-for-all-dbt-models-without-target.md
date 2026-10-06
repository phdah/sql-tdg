---
id: TASK-21.3
title: Generate one consistent source dataset for all dbt models without --target
status: Done
assignee:
  - '@opencode'
created_date: '2026-10-06 09:18'
updated_date: '2026-10-06 09:30'
labels: []
milestone: m-2
dependencies: []
parent_task_id: TASK-21
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Running `sql-tdg generate` with dbt input (or multiple raw SQL inputs) and no `--target`/`--target-layer` currently fails when the project has more than one terminal outcome (`AmbiguousTerminalOutcome`). The maintainer needs one run that produces a complete set of physical source tables for an entire dbt project.

Decided semantics (maintainer, 2026-10-06): every physical source used by any terminal model is generated once. When several terminal models depend on the same source, all of their predicates apply to that source's columns: each generated row must satisfy every dependent model's supported conditions at the same time (intersection). If the combined conditions are contradictory or cannot be guaranteed jointly (e.g. relational witnesses that cannot be combined), generation fails with an explicit error naming the conflicting models; it never silently drops a model or a condition. Sources used by only one model follow that model's semantics. Selecting `--target`/`--target-layer` keeps existing single-outcome behavior.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Without a target selector, generation covers every terminal outcome and emits each physical source relation exactly once
- [x] #2 Every generated row of a shared source satisfies the supported conditions of every terminal model that reads it
- [x] #3 Contradictory or not jointly supported conditions across models fail with an explicit error naming the involved outcomes
- [x] #4 Explicit --target/--target-layer behavior is unchanged
- [x] #5 Metadata records that the run targets all terminal outcomes
- [x] #6 Output is deterministic for the same inputs and seed
- [x] #7 Library and CLI tests cover disjoint sources, shared sources, and conflicting models
- [x] #8 README/CLI help document whole-project generation
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Library (`src/protocol.rs`):
1. In `generate_classified_from_bundle_at_boundary`, when the selector is `None`, the boundary is physical sources, and the bundle has more than one terminal outcome, route to a new joint path `generate_all_outcomes_data`. A single terminal outcome and explicit selectors keep the existing path unchanged; an intermediate boundary without a selector keeps `AmbiguousTerminalOutcome`.
2. Extract `outcome_layer` (DatasetRef -> layer id + description) from `select_terminal_layer` so both paths resolve outcomes identically.
3. Joint path: for every outcome, require resolved semantics without diagnostics, `validate_composed_domains`, and `collect_equality_relationships`. Union the dependencies (BTreeSet) and relationships.
4. Per source column, collect the domains from every outcome that constrains it. If 0 or 1 outcomes constrain it, use the existing `map_domain` (keeps range sampling). If 2 or more do, candidates = union of each domain's candidates filtered to values satisfying all domains -> `GenerationDomain::Values`. An empty result returns new `ProtocolGenerationError::ConflictingOutcomes { outcomes, message }`.
5. Relationships: relationship-column candidates come from the same intersected domains. Extract `relationship_components` from `choose_component_values`; before choosing values, check each component's common values and report `ConflictingOutcomes` naming the outcomes that constrain or relate any column in the component.
6. Rejected rows > 0 in joint mode -> `InvalidRowConfiguration` (no defined meaning across outcomes).
Metadata (`src/test_case/model.rs`): add `TargetKind::AllTerminalOutcomes` ("all-terminal-outcomes") and `TestTarget::all_terminal_outcomes(descriptions)`.
CLI (`src/main.rs`): no selector with several outcomes -> all-outcomes target in metadata and stdout `target=all:...`; update help text.
Tests: replace `multiple_terminal_outcomes_require_selection` (old behavior) with joint tests for disjoint sources, a shared source with intersected domains, conflicting outcomes, shared relationships, rejected-row error, and determinism; add a CLI test; keep the intermediate-boundary ambiguity test. Update README.
Trade-off: columns constrained by 2+ outcomes sample from the discrete candidate set (range bounds) rather than full ranges; correctness is preserved.

Adjustments during implementation: (a) relationship components pool candidates across all component columns and keep values satisfying every column's outcome domains (`shared_component_values`), because per-column range candidates alone missed overlaps (customer_id >= 5 vs id <= 7). (b) Failures while preparing one outcome in joint mode are wrapped in `ProtocolGenerationError::TerminalOutcome { outcome, source }` so the failing model is named. (c) The CLI sorts outcome descriptions in the all-outcomes target.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Evidence: tests/all_outcomes_generation.rs (9 tests: disjoint sources, shared source intersection, shared relationship keys, conflicting domains, conflicting relationships, unsupported outcome named, rejected rows refused, determinism, explicit selector unchanged); tests/cli.rs shared dataset + metadata AllTerminalOutcomes round-trip + conflict error; tests/dbt_core_e2e.rs new ignored test names subquery_orders (EXISTS) in whole-project mode. `make rust-checks` and `make dbt-e2e` (with data-core venv dbt-duckdb on PATH) pass.

Manual: e2e dbt fixture minus subquery_orders generated 4 sources for 7 terminal models; after dbt seed+run every terminal model returned rows. Removed `multiple_terminal_outcomes_require_selection` from tests/full_query.rs because it asserted the replaced behavior; intermediate-boundary ambiguity test kept.

Breaking public API additions: new variants `ProtocolGenerationError::{ConflictingOutcomes, TerminalOutcome}` and `TargetKind::AllTerminalOutcomes` (enums are exhaustive). Ships before 1.0.0.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Whole-project generation: without `--target`/`--target-layer` and with several terminal outcomes, physical-source generation now produces one shared dataset instead of failing with `AmbiguousTerminalOutcome`.

- `generate_classified_from_bundle_at_boundary` routes selector-less, physical-source, multi-outcome bundles to `generate_all_outcomes_data`. Single outcomes, explicit selectors, and intermediate boundaries are unchanged.
- Each source is generated once. A column constrained by one outcome keeps its exact domain (range sampling). A column constrained by several outcomes samples from candidates that satisfy every domain. Relationship keys get a value allowed by every domain on every connected column.
- Explicit errors: `ConflictingOutcomes { outcomes, message }` for contradictory domains or relationships; `TerminalOutcome { outcome, source }` when one model has unsupported semantics; `InvalidRowConfiguration` for rejected rows in joint mode.
- Metadata: `TargetKind::AllTerminalOutcomes` (`all-terminal-outcomes`), `TestTarget::all_terminal_outcomes`; CLI prints `target=all:<sorted outcomes>`.
- README and CLI help document the mode.
Trade-off: columns shared by several outcomes sample discrete boundary candidates rather than full ranges.
Tests: new tests/all_outcomes_generation.rs, CLI tests, dbt e2e test; `make rust-checks` and `make dbt-e2e` pass.
<!-- SECTION:FINAL_SUMMARY:END -->
