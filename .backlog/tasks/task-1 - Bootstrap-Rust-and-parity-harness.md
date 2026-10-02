---
id: TASK-1
title: Bootstrap Rust and parity harness
status: Done
assignee: []
created_date: '2026-10-02'
labels: []
milestone: m-0
dependencies: []
---

## Description

Create the Rust crate alongside the current Go module and establish a continuously green parity
workflow. The Go implementation remains the reference until final cutover.

Rust CI is mandatory in this first task. As soon as there is Rust code to test, pull requests
must verify formatting, Clippy, and tests so no later migration task can introduce unchecked
Rust code.

The GitHub workflow delegates Rust verification to the root Makefile so local and CI execution
use the same commands.

Do not port every Rust test up front as a permanently failing suite. Port tests together with each
migration slice so failures stay localized.

## Acceptance Criteria

- [x] A root Rust crate is initialized without removing the Go module.
- [x] Repository tooling can run both Go and Rust tests.
- [x] A Rust pull-request workflow is added in this task, not deferred to later migration work.
- [x] Rust CI calls `make rust-checks` rather than duplicating Cargo commands in workflow YAML.
- [x] `make rust-checks` runs `cargo fmt --all -- --check`.
- [x] `make rust-checks` runs `cargo clippy --all-targets --all-features -- -D warnings`.
- [x] `make rust-checks` runs `cargo test --all-targets --all-features`.
- [x] Existing Go CI remains enabled so pull requests verify both implementations during migration.
- [x] Existing Go tests are catalogued in a parity matrix with a planned Rust counterpart.
- [x] The parity contract distinguishes exact behavioral parity from seeded-generator determinism.
- [x] Required Rust dependencies are selected deliberately and documented.
- [x] The initial Rust suite is green.
