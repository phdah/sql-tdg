---
id: TASK-3
title: Port solver domains
status: To Do
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

- [ ] Integer equality, inequality, ordered bounds, and interval splitting match current semantics.
- [ ] Boolean constraints detect contradictory requirements.
- [ ] Timestamp parsing and domain narrowing match current supported date/timestamp behavior.
- [ ] Unsatisfiable domains return explicit errors.
- [ ] Randomness is injected rather than read from global state.
- [ ] Rust solver tests cover every current Go solver test case before the task is complete.
