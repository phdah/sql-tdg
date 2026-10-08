---
id: TASK-21.10
title: Spread generated join keys across parent rows with moderate values
status: Done
assignee:
  - '@opencode'
created_date: '2026-10-08 17:22'
updated_date: '2026-10-08 18:03'
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
- [x] #3 Unique and primary key constraints, foreign keys, query domains, and NULL rules are still satisfied, and rejected-row generation still breaks exactly one relationship
- [x] #4 Output stays deterministic for the same inputs and seed
- [x] #5 Tests assert key distribution and moderate values for a top-level join, a join inside a CTE, and a dbt source relationships constraint
- [x] #6 On the maintainer's paper_trail fixture, generated sources still pass all 12 dbt source tests and daily_revenue contains more than one row
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Fix for AC #3 (rejected parent rows still joining matching rows; maintainer chose to fix within this task, 2026-10-08).

Root cause: each rejected row index breaks one relationship column with a witness, but the other relations' rejected rows keep the component baseline key, and matching rows also use that key. A rejected parent row therefore joins a matching child and qualifies.

Fix, in both `generate_relational_data` and `generate_prepared_relational_data`, via one shared helper replacing `distribute_matching_relationship_values`:
- When rejected rows are requested, each relationship component reserves key values that no matching row uses (taken from the end of the moderate-first candidate list).
- Matching rows cycle over the remaining values.
- Rejected row r gets reserved[r] on every column of the component, so no rejected row in any relation can join a matching row.
- The existing witness then breaks one relationship. It picks a value absent from the neighbor column, which now includes the reserved keys, so rejected rows cannot join each other either.
- Reserved values are distinct per rejected row. Reuse is allowed only when the bundle has a single relationship, because in a chain shared keys would let rejected rows from different indices combine into a qualifying row.
- With no spare value, or too few for a chain, generation fails with NoBreakableRelationship.

Tests:
- A regression test runs the query against generated data in DuckDB and asserts no rejected row of any relation produces output, for a two-relation join and a three-relation chain.
- Existing relational tests stay green.
<!-- SECTION:PLAN:END -->

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

2026-10-08 AC #3 fix implemented and verified.

Code:
- `assign_relationship_values` replaces `distribute_matching_relationship_values` in single-outcome relational generation, intermediate-boundary relational generation, and whole-project generation (rejected rows are not allowed there, so it passes rejected = 0).
- When rejected rows are requested, each equality-relationship component reserves keys from the end of the moderate-first candidate order (`split_reserved_relationship_values`). Matching rows cycle over the rest; every rejected row gets its own reserved key on all component columns. The existing witness then breaks one relationship, picking a value absent from the neighbor column.
- Reuse of reserved keys is allowed only for a single relationship. A chain without enough spare keys fails with NoBreakableRelationship.
- Boundary candidate maps also contain non-key columns. Those single-column components keep matching distribution but are excluded from rejected key isolation; without that exclusion the advanced_boundary fixture failed.

Tests, in tests/full_query.rs:
- `rejected_rows_of_every_joined_relation_produce_no_query_output`: two-relation join; evaluates the join over all rows.
- `rejected_rows_of_a_join_chain_produce_no_query_output`: three-relation chain across 32 seeds.
- `join_chain_without_spare_keys_for_rejected_rows_is_an_explicit_error`.
- Both property tests fail without the fix, with `orders[30] joins items[1] on 60` and `seed 0: orders[12], customers[6], regions[6]`.

Verification:
- `make rust-checks` and `make dbt-e2e` (4 tests) pass.
- The original probe (matching 30, rejected 10) now yields 30 query rows and 0 involving a rejected row from either relation. Rejected order ids are 55..64; matching keys are 30 distinct values in 1..54.
- paper_trail rerun: 12/12 source tests pass and daily_revenue has 50 rows; only the two seed-specific singular tests fail.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Spreads generated join keys across parent rows with moderate values, and makes rejected relational rows non-qualifying on every side of a join.

Key distribution (PR #33):
- Typed moderate key candidates (1..64 per type).
- Deterministic cycling of matching join keys across distinct values.
- Foreign keys cycle across valid parent tuples.
- Tests cover a top-level join, a CTE join, and a dbt source relationship.

Rejected-row fix (this task, AC #3): rejected rows of the non-broken side previously kept a baseline key also used by matching rows, so a rejected parent row could join a matching child and qualify. This bug predates this task (present on main at 7c4bc98).
- When rejected rows are requested, each equality-relationship component now reserves keys used by no matching row, and gives each rejected row its own reserved key on every side.
- The existing witness then breaks one relationship.
- Reuse of reserved keys is limited to single-relationship joins; chains without enough spare keys fail with NoBreakableRelationship.
- Non-key boundary columns are excluded from this isolation.

Tests:
- New property tests evaluate the join over all generated rows for a two-relation join and a three-relation chain (32 seeds). Both fail without the fix.
- A test asserts the explicit error for insufficient keys.
- `make rust-checks` and `make dbt-e2e` pass.

Validation on the maintainer's paper_trail fixture:
- 50 distinct keys; 12/12 dbt source tests pass; daily_revenue has 50 rows instead of 1.
- Only the two singular tests with seed-specific hard-coded totals fail, as before.
<!-- SECTION:FINAL_SUMMARY:END -->
