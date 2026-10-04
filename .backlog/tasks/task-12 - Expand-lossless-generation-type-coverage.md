---
id: TASK-12
title: Expand lossless generation type coverage
status: Done
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-2
dependencies:
  - TASK-10
---

## Description

Expand the Arrow-backed generator so realistic SQL and dbt workloads are not limited to the current
32-bit integer, boolean, timestamp, and string subset.

Use the canonical datatype model supplied by SQL Semantic Protocol. Support every canonical type
that can be represented losslessly by the Arrow storage boundary. Types that still cannot be
represented exactly must continue to fail explicitly. DuckDB round-trip validation is owned by
TASK-16 when the execution backend and dependency are introduced.

## Progress

Implemented lossless Arrow-backed generation for the supported canonical SQL Semantic Protocol
datatype surface, including wider numeric types, temporal precision variants, binary/nullable
values, nested collections, structured values, JSON-like storage, UUIDs, and finite-domain types.
Unsupported custom/vendor datatypes remain explicit errors. DuckDB round-trip validation is
intentionally deferred to TASK-16, where the DuckDB execution dependency is introduced.

## Acceptance Criteria

- [x] Add exact generation for the signed integer widths required by the protocol and DuckDB fixtures, including 64-bit integers.
- [x] Add exact generation for supported unsigned integer, floating-point, decimal/numeric, date, time, and timestamp variants.
- [x] Add binary values and nullable values with deterministic null generation where the domain permits NULL.
- [x] Add lossless support for protocol nested/semi-structured types that DuckDB and Arrow can round-trip, including lists/arrays, structs, maps, and JSON-like data where supported.
- [x] Enum and other finite-domain types generate only values allowed by their protocol representation.
- [x] Recursive types reuse the protocol datatype model rather than introducing a parallel local SQL datatype taxonomy.
- [x] Boundary and representative values are covered for every newly supported type.
- [x] Arrow storage tests prove generated values preserve the canonical protocol type and value semantics.
- [x] Unsupported vendor/custom types continue to return explicit errors naming the unsupported type.
- [x] Determinism holds for every newly supported type for a fixed seed.
- [x] `make rust-checks` remains green.
