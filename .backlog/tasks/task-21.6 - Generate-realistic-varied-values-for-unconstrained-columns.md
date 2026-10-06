---
id: TASK-21.6
title: Generate realistic varied values for unconstrained columns
status: To Do
assignee: []
created_date: '2026-10-06 12:39'
labels: []
milestone: m-2
dependencies: []
parent_task_id: TASK-21
priority: high
type: enhancement
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Columns without a protocol domain are generated from a tiny default candidate set at type extremes (int64 min/0/max, dates 1969-12-31..1970-01-02, doubles -1/0/1, strings ''/'value'). Real models then overflow or produce nonsense aggregates (the maintainer's daily_revenue reached 4.6e20) and get only ~3 distinct values per column. Unconstrained columns should be sampled deterministically from varied, moderate, type-valid values. Constrained columns keep their exact protocol domains, and correctness guarantees must not weaken. Decision (maintainer, 2026-10-06): fix as part of the 1.0.0 gate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Unconstrained numeric, temporal, string, and boolean columns are sampled from varied moderate values rather than type extremes
- [ ] #2 Generated unconstrained columns contain many distinct values when many rows are requested
- [ ] #3 Typical arithmetic aggregates over unconstrained numeric columns do not overflow for default row counts
- [ ] #4 Constrained columns still satisfy their protocol domains exactly
- [ ] #5 Output remains deterministic for the same inputs and seed
- [ ] #6 Tests cover each supported type family and determinism
<!-- AC:END -->
