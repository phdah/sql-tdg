---
id: TASK-12
title: Expand lossless generation type coverage
status: To Do
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-1
dependencies:
  - TASK-10
---

## Description

Expand the Arrow-backed generator so realistic SQL and dbt workloads are not limited to the current
32-bit integer, boolean, timestamp, and string subset.

Use the canonical datatype model supplied by SQL Semantic Protocol. Support every canonical type
that can be represented losslessly by the chosen Arrow storage and DuckDB execution path. Types
that still cannot be represented exactly must continue to fail explicitly.

## Acceptance Criteria

- [ ] Add exact generation for the signed integer widths required by the protocol and DuckDB fixtures, including 64-bit integers.
- [ ] Add exact generation for supported unsigned integer, floating-point, decimal/numeric, date, time, and timestamp variants.
- [ ] Add binary values and nullable values with deterministic null generation where the domain permits NULL.
- [ ] Add lossless support for protocol nested/semi-structured types that DuckDB and Arrow can round-trip, including lists/arrays, structs, maps, and JSON-like data where supported.
- [ ] Enum and other finite-domain types generate only values allowed by their protocol representation.
- [ ] Recursive types reuse the protocol datatype model rather than introducing a parallel local SQL datatype taxonomy.
- [ ] Boundary and representative values are covered for every newly supported type.
- [ ] Arrow-to-DuckDB round-trip tests prove generated values preserve type and value semantics.
- [ ] Unsupported vendor/custom types continue to return explicit errors naming the unsupported type.
- [ ] Determinism holds for every newly supported type for a fixed seed.
- [ ] `make rust-checks` remains green.
