---
id: TASK-6
title: Port parser-solver interop
status: Done
assignee: []
created_date: '2026-10-02'
labels: []
milestone: m-0
dependencies:
  - TASK-3
  - TASK-4
  - TASK-5
---

## Description

Port the boundary that maps parser IR and table schema information into typed solver constraints.

This remains the only parser-to-solver conversion boundary.

## Acceptance Criteria

- [x] Integer condition IR maps to the equivalent typed constraints.
- [x] Boolean condition IR maps to the equivalent typed constraints.
- [x] Timestamp condition IR parses and maps to the equivalent typed constraints.
- [x] Unknown columns return explicit errors.
- [x] Unsupported column types and operators return explicit errors.
- [x] Rust interop tests cover every current Go interop expectation before the task is complete.
