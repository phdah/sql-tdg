---
id: TASK-7
title: Port generator with deterministic execution
status: Done
assignee: []
created_date: '2026-10-02'
labels: []
milestone: m-0
dependencies:
  - TASK-3
  - TASK-5
---

## Description

Port generation across solver domains and Arrow-backed table storage.

Preserve semantic behavior, but do not preserve the current shared-RNG concurrency pattern or
panic-only invalid-input paths.

## Acceptance Criteria

- [x] Generated integer, timestamp, and boolean values satisfy their constraints.
- [x] The same query/schema/seed produces identical Rust output across runs.
- [x] Output ordering does not depend on thread scheduling.
- [x] No mutable RNG is shared unsafely between workers.
- [x] Invalid generation requests return errors rather than panicking.
- [x] Row counts do not require accidental divisibility by a fixed worker count.
- [x] Generator tests validate deterministic behavior without depending on Go `math/rand` samples unless compatibility is explicitly required.
