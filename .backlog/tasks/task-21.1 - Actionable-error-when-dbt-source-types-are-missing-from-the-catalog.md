---
id: TASK-21.1
title: Actionable error when dbt source types are missing from the catalog
status: Done
assignee:
  - '@opencode'
created_date: '2026-10-06 09:18'
updated_date: '2026-10-06 09:20'
labels: []
milestone: m-2
dependencies: []
parent_task_id: TASK-21
priority: high
type: enhancement
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
When a dbt physical dependency has no warehouse schema in catalog.json (typically because the source tables do not exist in the warehouse when `dbt docs generate` runs, producing an empty catalog), the CLI currently surfaces only the protocol message `dbt catalog has no warehouse schema for physical dependency '<relation>'`. The CLI must keep refusing (types are unknown, so generating would over-claim) but tell the user how to fix it: create the source tables in the warehouse (or declare column `data_type` once the protocol supports manifest-declared types) and rerun `dbt docs generate`. Decision (maintainer, 2026-10-06): do both this hint and the manifest data_type fallback.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Missing catalog schema for a dbt physical dependency fails with an error naming the relation
- [x] #2 The error explains how to provide the missing column types and rerun dbt docs generate
- [x] #3 A CLI test covers the empty-catalog case
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. In `src/main.rs` `analyze_dbt`, match `DbtArtifactsError::MissingCatalogSchema` and append a hint: the source tables must exist in the warehouse when `dbt docs generate` runs (catalog.json is built from the warehouse); create them and rerun `dbt docs generate`. Other errors keep the existing message.
2. Add a CLI test in `tests/cli.rs` using a manifest with a source and an empty catalog asserting the relation name and hint appear and exit code is non-zero.
3. Run `make rust-checks`.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Verified on the maintainer fixture: error names '"target"."main"."order_items"' and prints the hint, exit 2. After creating the two source tables in a copy of target.db and rerunning `dbt docs generate`, `sql-tdg generate --dbt-project . ` (no --target, single model) succeeds.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
CLI now appends an actionable hint to `DbtArtifactsError::MissingCatalogSchema`: catalog.json is built from the warehouse, so source tables must exist when `dbt docs generate` runs; create them (seed/DDL), rerun docs generate, retry. Generation still refuses because the types are unknown. Added `compiled_cli_explains_dbt_sources_missing_from_catalog` in tests/cli.rs with an inline manifest and empty catalog. `make rust-checks` passes.
<!-- SECTION:FINAL_SUMMARY:END -->
