---
id: TASK-21.9
title: Bump sql-semantic-protocol to 2.0.2 for dbt source-test attribution
status: To Do
assignee: []
created_date: '2026-10-08 14:51'
labels: []
milestone: m-2
dependencies: []
references:
  - 'https://github.com/phdah/sql-semantic-protocol/pull/72'
  - 'https://github.com/phdah/sql-semantic-protocol/pull/74'
  - Cargo.toml
parent_task_id: TASK-21
priority: high
type: chore
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
sql-tdg pins `sql-semantic-protocol = "2.0.0"`.

Protocol releases since then:
- **2.0.1:** changes only the protocol CLI help; no library impact.
- **2.0.2** (protocol PR #72, release PR #74): fixes dbt tests declared on sources. dbt writes `attached_node: null` for them.
  - In 2.0.0, a `relationships` test between two sources aborted analysis of the whole project, so the maintainer's paper_trail gate project (TASK-21) could not be analyzed at all.
  - 2.0.2 identifies the tested resource from the test's `kwargs.model` argument, which also covers self-referencing relationships.
  - A built-in test it still cannot attribute becomes the new bundle-level constraint diagnostic `unattributed_dbt_test`, instead of aborting analysis.

The protocol's default `odcs` feature still pulls in `saphyr` (YAML) and a few transitive crates that sql-tdg does not use. Whether to keep it is a maintainer decision (AGENTS.md: minimize dependency footprint, enable only used features).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cargo.toml requires sql-semantic-protocol 2.0.2 or later and Cargo.lock resolves the published 2.0.2 (or newer) crate
- [ ] #2 The protocol dependency's default features are an explicit maintainer decision: either the default odcs feature is kept or default-features = false is set, recorded in the task notes
- [ ] #3 A regression test generates source data for a dbt manifest whose sources carry a relationships test with attached_node: null (the paper_trail shape), and the manifest no longer fails analysis
- [ ] #4 make rust-checks and make dbt-e2e pass
<!-- AC:END -->
