---
id: TASK-21
title: Manual dbt project testing gate before 1.0.0 release
status: In Progress
assignee:
  - '@opencode'
created_date: '2026-10-06 08:23'
updated_date: '2026-10-06 09:18'
labels: []
milestone: m-2
dependencies: []
references:
  - TASK-20
priority: high
type: task
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Final manual testing gate owned by the maintainer before the Release Please `1.0.0` PR is merged and `v1.0.0` is released (TASK-20). The maintainer exercises the installed `sql-tdg` CLI against real dbt projects (for example the DuckDB fixture at `~/repos/work/dp/data-core/antstack/tests/fixtures/dbt_project`: two sources `target.orders` and `target.order_items`, one model `daily_revenue`) and every defect or usability gap found is fixed before release.

Findings so far:
1. `sql-tdg generate --dbt-manifest target/manifest.json --dbt-catalog target/catalog.json --target target.main.daily_revenue` fails with `dbt catalog has no warehouse schema for physical dependency '"target"."main"."order_items"'` when the source tables do not exist in the warehouse (so `dbt docs generate` writes an empty catalog). The user-facing outcome must be either successful generation from legitimate type metadata or an actionable error explaining how to provide source types.
2. dbt generation requires selecting a single target. The maintainer needs to run without `--target` and get one complete, consistent set of generated tables covering every model in the dbt project.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The maintainer has manually run the installed CLI against at least one real dbt project and recorded the commands and results in the task notes
- [ ] #2 A dbt project whose source tables are absent from the warehouse either generates data from legitimate type metadata or fails with an error that names the relation and explains how to supply its column types
- [ ] #3 `sql-tdg generate` with dbt input and no `--target` generates data for every model in the project in one run without over-claiming any model's semantics
- [ ] #4 Every defect found during the gate is fixed with tests or explicitly deferred with the maintainer's approval
- [ ] #5 `make rust-checks` passes on the branch merged for this gate
- [ ] #6 The maintainer signs off on the gate before the 1.0.0 release PR is merged
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Branch: feat/dbt-whole-project-generation.
Subtasks (sequenced):
1. TASK-21.1 actionable missing-catalog-schema error (sql-tdg CLI).
2. TASK-21.3 whole-project generation without --target using intersection semantics across terminal outcomes (sql-tdg library + CLI).
3. TASK-21.2 manifest data_type fallback: requires a change and release in sql-semantic-protocol first, then a dependency bump here.
4. Maintainer runs manual tests against real dbt projects and signs off; then TASK-20 can proceed.
<!-- SECTION:PLAN:END -->
