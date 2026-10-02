---
id: TASK-1
title: Bootstrap Rust and parity harness
status: To Do
assignee: []
created_date: '2026-10-02'
labels: []
milestone: m-0
dependencies: []
---

## Description

Create the Rust crate alongside the current Go module and establish a continuously green parity
workflow. The Go implementation remains the reference until final cutover.

Do not port every Rust test up front as a permanently failing suite. Port tests together with each
migration slice so failures stay localized.

## Acceptance Criteria

- [ ] A root Rust crate is initialized without removing the Go module.
- [ ] Repository tooling can run both Go and Rust tests.
- [ ] Pull-request CI runs both suites during the migration.
- [ ] Existing Go tests are catalogued in a parity matrix with a planned Rust counterpart.
- [ ] The parity contract distinguishes exact behavioral parity from seeded-generator determinism.
- [ ] Required Rust dependencies are selected deliberately and documented.
- [ ] The initial Rust suite is green.
