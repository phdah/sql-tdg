---
id: TASK-2
title: Port shared types and invariants
status: Done
assignee: []
created_date: '2026-10-02'
labels: []
milestone: m-0
dependencies:
  - TASK-1
---

## Description

Port the shared domain model needed by parser, solver, and table code. Prefer Rust enums and
concrete types over a mechanical translation of Go interfaces and any-style values.

## Acceptance Criteria

- [x] Column types are represented by a closed Rust enum.
- [x] Column, interval, and constraint representations preserve current supported semantics.
- [x] Constructors enforce interval and other relevant invariants.
- [x] Invalid states are made unrepresentable where practical.
- [x] Shared types do not depend on parser, interop, table, or generator modules.
- [x] Rust tests cover the same observable invariants as the current Go types/utilities.
