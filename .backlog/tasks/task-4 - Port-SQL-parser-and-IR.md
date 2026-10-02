---
id: TASK-4
title: Port SQL parser and IR
status: To Do
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

- [ ] Current SELECT, FROM, JOIN, WHERE, QUALIFY, AND, OR, comparison, identifier, literal, and function parsing has Rust coverage.
- [ ] Condition IR matches current supported lowering behavior.
- [ ] Join kind and join condition IR match current supported behavior.
- [ ] Unsupported constructs return explicit errors instead of being silently discarded.
- [ ] Parser-specific AST types do not leak past the parser boundary.
- [ ] Rust parser tests cover every current Go parser expectation before the task is complete.
