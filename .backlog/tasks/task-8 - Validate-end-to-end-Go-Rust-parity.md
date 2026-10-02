---
id: TASK-8
title: Validate end-to-end Go/Rust parity
status: To Do
assignee: []
created_date: '2026-10-02'
labels: []
milestone: m-0
dependencies:
  - TASK-6
  - TASK-7
---

## Description

Validate the completed Rust implementation against the existing Go reference before any Go code
is removed.

The same representative inputs must be runnable through both implementations so their observable
behavior can be compared directly. This task is the acceptance gate for the migration and must
include an interactive/manual comparison by the user in addition to automated parity coverage.

The Python proof of concept remains frozen.

## Acceptance Criteria

- [ ] Full-query Rust tests cover the currently supported integer, boolean, and timestamp flows.
- [ ] A repeatable parity harness can run the same representative query/schema/input cases through both Go and Rust.
- [ ] Deterministic parser, solver, interop, and table behavior is compared directly between Go and Rust.
- [ ] Generator parity verifies equivalent semantics and constraint satisfaction without requiring Rust to reproduce Go `math/rand` samples.
- [ ] Seeded Rust generation is deterministic and all generated rows satisfy supported constraints.
- [ ] Unsupported behavior remains explicit rather than being silently accepted.
- [ ] The user interactively runs representative inputs through both implementations one after another and accepts the observed parity.
- [ ] Go remains present and runnable after this task is complete.
- [ ] TASK-9 does not start until this parity task has been accepted.
