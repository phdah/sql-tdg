---
id: TASK-8
title: Validate parity and cut over to Rust
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

Complete end-to-end parity and make Rust the sole active implementation.

The Python proof of concept remains frozen.

## Acceptance Criteria

- [ ] Full-query Rust tests cover the currently supported integer, boolean, and timestamp flows.
- [ ] Go and Rust parity is checked for deterministic parser, solver, interop, and table behavior.
- [ ] Seeded Rust generation is deterministic and all generated rows satisfy supported constraints.
- [ ] Unsupported behavior remains explicit rather than being silently accepted.
- [ ] Rust becomes the implementation described by README.md and AGENTS.md.
- [ ] AGENTS.md is simplified from dual-language migration guidance to Rust-only guidance.
- [ ] Rust CI created in TASK-1 remains mandatory with formatting, Clippy, and tests.
- [ ] Go-only CI is removed after parity is complete.
- [ ] Makefile targets and the pre-push hook use the Rust toolchain.
- [ ] Active Go implementation files, go.mod, and go.sum are removed.
- [ ] The repository is green after the Go removal.
