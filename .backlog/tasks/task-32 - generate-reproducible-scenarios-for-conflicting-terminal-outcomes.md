---
id: TASK-32
title: Generate reproducible scenarios for conflicting terminal outcomes
status: Done
assignee: []
created_date: '2026-10-08'
updated_date: '2026-10-08'
labels: []
milestone: m-3
dependencies: []
references:
  - 'TASK-21.3'
  - 'TASK-19'
  - 'docs/usage.md'
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Whole-project generation intersects all terminal constraints, and mutually contradictory model requirements yield ConflictingOutcomes. Add an opt-in scenario mode that partitions compatible outcomes rather than silently weakening any of them.

This is post-1.0 feature work, not a claim about existing CLI behavior. Canonical semantics, dialect resolution, lineage and SQL analysis belong solely to SQL Semantic Protocol. If upstream semantics are missing, implement and release the protocol prerequisite first; do not add interpretation fallbacks in sql-tdg. Preserve deterministic outputs and exactness errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Keep existing all-outcomes single-dataset mode unchanged and still fail when no single shared dataset can meet the selected outcomes.
- [x] #2 Provide a deterministic scenario partitioning mode with explicit outcome membership, distinct files/directories and metadata describing which terminal models each scenario satisfies.
- [x] #3 Select scenarios from canonical protocol outcome and dependency graph semantics; do not reparse SQL or invent weakened constraints.
- [x] #4 Test conflicting and compatible model groups, shared sources, seed reproducibility and independently executable dbt fixtures.
- [x] #5 Expose a clear CLI/API option and document why scenarios are not one universal dataset.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented opt-in `--scenarios` and `generate_scenarios_from_bundle` using a deterministic
first-fit partition of canonical protocol terminal outcomes. Candidate groups are accepted
only when the existing exact shared-outcomes generator succeeds. Unsupported outcomes
still fail closed, and default whole-project generation is unchanged. Each scenario has
an isolated numbered directory and a `scenario-outcomes` metadata target that records its
members. Scenario mode requires matching-only physical sources with no explicit target.
Added API regression tests, compiled dbt fixture/CLI integration tests, independent DuckDB
execution of the scenario sources, determinism and error-path checks, and documentation.

Rust formatting, lint, rustdoc and Rust test jobs passed on PR #43 revision
`082d8a2`; the first dbt Core E2E job also passed. Final CI will validate the
completed task record and final CLI diagnostic-output change.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added isolated reproducible scenario datasets for incompatible terminal outcomes
without weakening the protocol's exactness guarantees. Compatible models share a
scenario, incompatible models use separate source datasets, and every scenario
records explicit outcome membership for independent verification. Implemented
in PR #43.
<!-- SECTION:FINAL_SUMMARY:END -->
