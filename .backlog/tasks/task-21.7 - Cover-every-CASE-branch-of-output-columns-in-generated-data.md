---
id: TASK-21.7
title: Cover every CASE branch of output columns in generated data
status: To Do
assignee: []
created_date: '2026-10-06 12:39'
updated_date: '2026-10-08 11:36'
labels: []
milestone: m-2
dependencies:
  - TASK-21.8
references:
  - sql-semantic-protocol TASK-32
  - sql-semantic-protocol TASK-39
parent_task_id: TASK-21
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generated data only satisfies query conditions; it does not exercise output expressions. In the maintainer's daily_revenue model every row produced revenue_category 'high' and never 'medium' or 'low'. Generated matching rows should include rows that reach each reachable CASE branch (including ELSE) of output columns. Per AGENTS.md, predicate-to-domain derivation belongs to sql-semantic-protocol. Unreachable or non-derivable branches must be reported explicitly, never claimed covered. Decision (maintainer, 2026-10-06): part of the 1.0.0 gate.

Protocol status (verified at protocol commit 3a4d3c6, release 2.0.0):
- `CaseBranch::source_domains()` and `CaseExpression::else_source_domains()` return `Reachable` (an OR of alternatives, each an AND of column domains), `Unreachable`, or `Unknown` with a reason. Branch order and SQL NULL behavior are respected (protocol TASK-32).
- Branch domains are carried through CTEs, derived tables, and composition layers down to physical columns; a computed hop makes the branch Unknown (protocol TASK-39).
- Reachability is local to the CASE and ignores query WHERE, HAVING, and QUALIFY constraints, so sql-tdg must intersect each branch with the composed column domains. Branch domains over string, float, or timestamp columns are subject to the same comparison-semantics assumptions as filters.
- daily_revenue's CASE is over SUM(...), so every branch is Unknown. That model can only be reported as not coverable, not covered.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 sql-tdg depends on the protocol 2.x release that emits per-branch source-column domains for CASE output expressions
- [ ] #2 Generated matching rows reach every reachable CASE branch including ELSE
- [ ] #3 Branches whose domains cannot be derived or are unreachable produce an explicit error or report, never a silent omission
- [ ] #4 All rows still satisfy the query's conditions
- [ ] #5 Tests cover searched CASE, simple CASE, ELSE, and unreachable branches
- [ ] #6 Branch domains are intersected with the query's composed column domains, and branches made empty by query filters are reported as unreachable under the query
- [ ] #7 Branch coverage follows the same exactness and comparison-assumption rules as TASK-21.8
- [ ] #8 CASE expressions inside CTEs and upstream layers are covered, and a CASE over an aggregate (as in daily_revenue) is reported as not coverable
<!-- AC:END -->
