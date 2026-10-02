---
id: TASK-9
title: Cut over to Rust and remove Go
status: To Do
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

- [ ] TASK-8 is complete and its interactive Go/Rust parity comparison has been accepted.
- [ ] Rust becomes the implementation described by README.md and AGENTS.md.
- [ ] AGENTS.md is simplified from dual-language migration guidance to Rust-only guidance.
- [ ] Rust CI created in TASK-1 remains mandatory and delegates verification to the Makefile.
- [ ] Go-only CI is removed.
- [ ] Makefile targets and the pre-push hook use the Rust toolchain as the active implementation.
- [ ] The default integer domain is aligned with the timestamp domain: `0..=i32::MAX`.
- [ ] Active Go implementation files, go.mod, and go.sum are removed.
- [ ] The repository is green after the Go removal.
