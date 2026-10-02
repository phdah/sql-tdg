---
id: TASK-4
title: Port SQL parser and IR
status: Done
assignee: []
created_date: '2026-10-02'
labels: []
milestone: m-0
dependencies:
  - TASK-1
---

## Description

Port SQL parsing and lowering to the project's condition and join IR types.

A general-purpose Rust SQL parser may accept more syntax than the generator supports. Accepted
syntax must still fail explicitly during lowering when the project cannot safely represent it.

## Acceptance Criteria

- [x] Current SELECT, FROM, JOIN, WHERE, QUALIFY, AND, OR, comparison, identifier, literal, and function parsing has Rust coverage.
- [x] Condition IR matches current supported lowering behavior.
- [x] Join kind and join condition IR match current supported behavior.
- [x] Unsupported constructs return explicit errors instead of being silently discarded.
- [x] Parser-specific AST types do not leak past the parser boundary.
- [x] Rust parser tests cover every current Go parser expectation before the task is complete.
