---
id: TASK-21.5
title: Generate from CTE and derived-table semantics once the protocol carries them
status: To Do
assignee: []
created_date: '2026-10-06 12:39'
updated_date: '2026-10-06 13:26'
labels: []
milestone: m-2
dependencies:
  - TASK-21.4
references:
  - >-
    sql-semantic-protocol TASK-31 (Carry CTE and derived-table semantics through
    query analysis, milestone 1.1.0)
parent_task_id: TASK-21
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Standard dbt models are written as CTE chains. Once sql-semantic-protocol represents joins, predicates, and lineage inside CTEs and derived tables (protocol feature, not yet planned there as of 2026-10-06), sql-tdg must bump its protocol dependency, replace the refusal from the CTE guard subtask with real support, and coordinate join keys and domains through local relations. Blocked on the protocol release.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 sql-tdg depends on a protocol release that carries CTE and derived-table joins, predicates, and lineage
- [ ] #2 A filter inside a CTE or derived table constrains generated source rows
- [ ] #3 An inner equality join inside a CTE coordinates generated join keys
- [ ] #4 The maintainer's daily_revenue dbt model generates joined rows without uncoordinated keys
- [ ] #5 Unsupported local-relation shapes still fail explicitly
<!-- AC:END -->
