---
id: TASK-21.10
title: Spread generated join keys across parent rows with moderate values
status: To Do
assignee: []
created_date: '2026-10-08 17:22'
labels: []
milestone: m-2
dependencies: []
references:
  - TASK-22
  - TASK-21.6
  - src/protocol.rs
parent_task_id: TASK-21
priority: medium
type: enhancement
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found during the TASK-22 paper_trail validation (2026-10-08, branch feat/task-22-protocol-relation-constraints, HEAD 2cff994, protocol 2.0.3, `sql-tdg generate --dbt-project ... --matching 50 --seed 42`).

Every generated `order_items` row received the same `order_id`, `-9223372036854775808` (i64::MIN). The data was valid: all 13 dbt source tests passed, including the order_items to orders relationship. But it is poor test data:
- downstream joins and aggregations exercise a single key, so the daily_revenue model produced exactly one row;
- the extreme boundary value looks unrealistic.

Earlier probes showed the same single-key behavior for top-level and CTE joins. Unconstrained columns already use moderate values (TASK-21.6); join keys and constrained keys do not.

Outcome: matching rows of related relations spread across several distinct key values, each child key referencing an existing parent key. Key values are moderate, not type extremes, unless the protocol domain requires them. Every existing guarantee still holds: query domains, join equalities, unique/primary keys, foreign keys, NULL rules, rejected-row classification, and determinism.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 For a supported inner equality join, matching child rows reference several distinct parent key values when the domains and row counts allow it, instead of one shared value
- [ ] #2 Generated key values for join and key columns prefer moderate values consistent with TASK-21.6, and use type extremes only when the protocol domain requires them
- [ ] #3 Unique and primary key constraints, foreign keys, query domains, and NULL rules are still satisfied, and rejected-row generation still breaks exactly one relationship
- [ ] #4 Output stays deterministic for the same inputs and seed
- [ ] #5 Tests assert key distribution and moderate values for a top-level join, a join inside a CTE, and a dbt source relationships constraint
- [ ] #6 On the maintainer's paper_trail fixture, generated sources still pass all 13 dbt source tests and daily_revenue contains more than one row
<!-- AC:END -->
