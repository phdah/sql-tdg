---
id: TASK-21.6
title: Generate realistic varied values for unconstrained columns
status: Done
assignee:
  - '@opencode'
created_date: '2026-10-06 12:39'
updated_date: '2026-10-06 12:45'
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
- [x] #1 Unconstrained numeric, temporal, string, and boolean columns are sampled from varied moderate values rather than type extremes
- [x] #2 Generated unconstrained columns contain many distinct values when many rows are requested
- [x] #3 Typical arithmetic aggregates over unconstrained numeric columns do not overflow for default row counts
- [x] #4 Constrained columns still satisfy their protocol domains exactly
- [x] #5 Output remains deterministic for the same inputs and seed
- [x] #6 Tests cover each supported type family and determinism
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Keep `default_values` (also used for boundary probing, set exclusion, relationship candidates). Add `GenerationDomain::Unconstrained { data_type }` sampled by `protocol_value::sample_unconstrained_value`. `generic_generation_domain` maps `None`/`Unbounded` to it. Intermediate boundaries use it when the producer domain is unbounded and the downstream domain is absent or unbounded. Whole-project mode uses it when all outcome domains are unbounded. Moderate ranges: integers 1..=min(1000, type max), decimals 0..=1000 within precision, floats 0..=1000 in cents, dates/timestamps 2020-01-01..2025-12-31 at second granularity, time any second of day, varying strings `value-N` truncated to max length, NULL one in ten for nullable; other types sample their defaults.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Evidence: tests/unconstrained_values.rs (bigint/tinyint/unsigned, decimal/double, varchar/varchar(3)/boolean, date/timestamp, constrained column keeps domain, determinism; 200 rows with >=50 distinct values per varied column). All existing suites pass unchanged. The dbt e2e golden snapshot was updated (amount 40/57 -> 55/56) because unconstrained columns consume the seeded random stream differently; rows and categories are unchanged and values remain inside the model domains.

Not changed (open question for maintainer): relationship join keys still take one constant value per component from `default_values` (e.g. int64 max), and half-open protocol ranges such as `x >= 10` are still sampled across the full representable interval as documented, which can produce very large values.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Columns without a protocol domain now get varied, moderate values instead of three type extremes. Added `GenerationDomain::Unconstrained` and `sample_unconstrained_value`, used for single-outcome, intermediate-boundary, and whole-project generation. Constrained domains, rejected-row logic, and relationship candidates are unchanged. README documents the ranges. Tests: tests/unconstrained_values.rs; dbt e2e snapshot updated for the shifted random stream; `make rust-checks` and `make dbt-e2e` pass.
<!-- SECTION:FINAL_SUMMARY:END -->
