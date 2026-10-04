---
id: TASK-9
title: Cut over to Rust and remove Go
status: Done
assignee: []
created_date: '2026-10-02'
labels: []
milestone: m-0
dependencies:
  - TASK-8
---

## Description

After end-to-end Go/Rust parity has been explicitly accepted in TASK-8, make Rust the sole active
implementation and remove the Go reference implementation.

Do not begin this task before TASK-8 is complete and accepted.

The Python proof of concept remains frozen.

## Acceptance Criteria

- [x] TASK-8 is complete and its interactive Go/Rust parity comparison has been accepted.
- [x] Rust becomes the implementation described by README.md and AGENTS.md.
- [x] AGENTS.md is simplified from dual-language migration guidance to Rust-only guidance.
- [x] Rust CI created in TASK-1 remains mandatory and delegates verification to the Makefile.
- [x] Go-only CI is removed.
- [x] Makefile targets and the pre-push hook use the Rust toolchain as the active implementation.
- [x] The default integer domain uses the full signed range: `i32::MIN..=i32::MAX`, while timestamps remain `0..=i32::MAX`.
- [x] Active Go implementation files, go.mod, and go.sum are removed.
- [x] The repository is green after the Go removal.
