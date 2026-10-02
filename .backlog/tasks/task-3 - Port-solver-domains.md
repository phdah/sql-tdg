---
id: TASK-3
title: Port solver domains
status: Done
assignee: []
created_date: '2026-10-02'
labels: []
milestone: m-0
dependencies:
  - TASK-2
---

## Description

Port integer, boolean, and timestamp domain solving to Rust using the existing Go tests as the
behavioral reference.

## Acceptance Criteria

- [x] Integer equality, inequality, ordered bounds, and interval splitting match current semantics.
- [x] Boolean constraints detect contradictory requirements.
- [x] Timestamp parsing and domain narrowing match current supported date/timestamp behavior.
- [x] Unsatisfiable domains return explicit errors.
- [x] Randomness is injected rather than read from global state.
- [x] Rust solver tests cover every current Go solver test case before the task is complete.
