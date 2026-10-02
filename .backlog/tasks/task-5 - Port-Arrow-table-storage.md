---
id: TASK-5
title: Port Arrow table storage
status: To Do
assignee: []
created_date: '2026-10-02'
labels: []
milestone: m-0
dependencies:
  - TASK-2
---

## Description

Port the Arrow-backed table implementation to Rust after the shared type model is stable.

The Rust representation may use more idiomatic Arrow structures than the current four-map Go
implementation as long as observable table behavior remains equivalent.

## Acceptance Criteria

- [ ] Integer, timestamp, boolean, and string columns use Arrow-backed storage.
- [ ] Append validates column existence and value type.
- [ ] Finalization/build behavior is covered for every supported column type.
- [ ] Typed getters preserve schema column identity and values.
- [ ] Wipe resets built arrays and builders safely for reuse.
- [ ] Integer and timestamp sorting preserve current observable behavior.
- [ ] Rust table tests cover every current Go table expectation before the task is complete.
