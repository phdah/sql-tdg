---
id: TASK-21.10
title: Spread generated join keys across parent rows with moderate values
status: In Progress
assignee:
  - '@opencode'
created_date: '2026-10-08 17:22'
updated_date: '2026-10-08 17:53'
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

Every generated `order_items` row received the same `order_id`, `-9223372036854775808` (i64::MIN). The data was valid: all 12 dbt source tests passed, including the order_items to orders relationship. But it is poor test data:
- downstream joins and aggregations exercise a single key, so the daily_revenue model produced exactly one row;
- the extreme boundary value looks unrealistic.

Earlier probes showed the same single-key behavior for top-level and CTE joins. Unconstrained columns already use moderate values (TASK-21.6); join keys and constrained keys do not.

Outcome: matching rows of related relations spread across several distinct key values, each child key referencing an existing parent key. Key values are moderate, not type extremes, unless the protocol domain requires them. Every existing guarantee still holds: query domains, join equalities, unique/primary keys, foreign keys, NULL rules, rejected-row classification, and determinism.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 For a supported inner equality join, matching child rows reference several distinct parent key values when the domains and row counts allow it, instead of one shared value
- [x] #2 Generated key values for join and key columns prefer moderate values consistent with TASK-21.6, and use type extremes only when the protocol domain requires them
- [ ] #3 Unique and primary key constraints, foreign keys, query domains, and NULL rules are still satisfied, and rejected-row generation still breaks exactly one relationship
- [x] #4 Output stays deterministic for the same inputs and seed
- [x] #5 Tests assert key distribution and moderate values for a top-level join, a join inside a CTE, and a dbt source relationships constraint
- [x] #6 On the maintainer's paper_trail fixture, generated sources still pass all 12 dbt source tests and daily_revenue contains more than one row
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-08: PR #33 adds typed moderate key candidates, deterministic distribution across joined source rows, and cycling foreign keys across valid parent tuples. Direct SQL, CTE, and dbt source constraint regressions assert key spread and preserved referential validity. The Rust checks and dbt build constraints regression passed on the previous candidate; final CI pending. AC #6 needs validation against the maintainer-only paper_trail fixture, which is not committed in this repository. Do not claim it has passed until that check runs.

2026-10-08 validation, branch feat/task-21-10-distribute-relationship-keys at f80daec.

**paper_trail (AC #6), checked.** Run as for TASK-22: scratch copy, `generate --dbt-project --matching 50 --seed 42 --format csv`, load into DuckDB, `dbt build`.
- orders: 50 distinct order_ids in 9..58.
- order_items: 50 distinct order_ids in 9..58, item_id in 1..802, 0 orphans.
- All 12 dbt source tests pass. Correction: earlier records said 13; a grep had also matched the daily_revenue relationships test, whose name contains `source_target_orders`.
- daily_revenue builds 50 rows (previously 1), and its 6 generic tests pass.
- Only the two singular tests with seed-specific hard-coded totals fail, as before.
- revenue_category is 'high' for every row, which is expected: its CASE is over an aggregate and the branches are unknown (TASK-21.7).

**Independent probes.** Query: `SELECT o.id, i.qty FROM orders o JOIN items i ON o.id = i.order_id WHERE i.qty BETWEEN 1 AND 3 AND o.region IN (1,2)`, with BIGINT/INTEGER schemas and the duckdb dialect.
- Top-level join: 30 matching rows use 30 distinct parent keys in 1..64, and every row satisfies the query.
- Same query through two CTEs: identical distribution.
- Same seed run twice: byte-identical output.

**AC #3, unchecked.** Rejected-row classification is wrong on the parent side. With `--rejected 10`:
- The 10 rejected items rows are correct: order_id 30 matches no order.
- All 10 rejected orders rows have id 60 (equal to the key of matching item row 2) and region in {1,2}. Each joins item row 2 and satisfies every filter, so the query returns them although they are classified as rejected.
- The same happens on main at 7c4bc98 (before TASK-21.10), so it is a pre-existing bug, not a regression. But the criterion 'rejected-row generation still breaks exactly one relationship' does not hold for rejected parent rows.
<!-- SECTION:NOTES:END -->
