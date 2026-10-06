---
id: TASK-21.7
title: Cover every CASE branch of output columns in generated data
status: To Do
assignee: []
created_date: '2026-10-06 12:39'
updated_date: '2026-10-06 13:26'
labels: []
milestone: m-2
dependencies: []
references:
  - >-
    sql-semantic-protocol TASK-32 (Emit per-branch source-column domains for
    CASE output expressions, milestone 1.1.0)
parent_task_id: TASK-21
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generated data only satisfies query conditions; it does not exercise output expressions. In the maintainer's daily_revenue model every row produced revenue_category 'high' and never 'medium' or 'low'. Generated matching rows should include rows that reach each reachable CASE branch (including ELSE) of output columns. Per AGENTS.md, predicate-to-domain derivation belongs to sql-semantic-protocol, so this requires the protocol to emit per-branch source-column domains (and lineage through CTEs for this model). Unreachable or non-derivable branches must be reported explicitly, never claimed covered. Decision (maintainer, 2026-10-06): part of the 1.0.0 gate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 sql-tdg depends on a protocol release that emits per-branch source-column domains for CASE output expressions
- [ ] #2 Generated matching rows reach every reachable CASE branch including ELSE
- [ ] #3 Branches whose domains cannot be derived or are unreachable produce an explicit error or report, never a silent omission
- [ ] #4 All rows still satisfy the query's conditions
- [ ] #5 Tests cover searched CASE, simple CASE, ELSE, and unreachable branches
<!-- AC:END -->
