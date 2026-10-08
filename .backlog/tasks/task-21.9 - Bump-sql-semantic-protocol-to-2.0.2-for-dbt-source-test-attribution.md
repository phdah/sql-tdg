---
id: TASK-21.9
title: Bump sql-semantic-protocol to 2.0.2 for dbt source-test attribution
status: Done
assignee:
  - '@opencode'
created_date: '2026-10-08 14:51'
updated_date: '2026-10-08 15:01'
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
- [x] #1 Cargo.toml requires sql-semantic-protocol 2.0.2 or later and Cargo.lock resolves the published 2.0.2 (or newer) crate
- [x] #2 The protocol dependency's default features are an explicit maintainer decision: either the default odcs feature is kept or default-features = false is set, recorded in the task notes
- [x] #3 A regression test generates source data for a dbt manifest whose sources carry a relationships test with attached_node: null (the paper_trail shape), and the manifest no longer fails analysis
- [x] #4 make rust-checks and make dbt-e2e pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Cargo.toml: `sql-semantic-protocol = { version = "2.0.2", default-features = false }`. Maintainer decision 2026-10-08: disable the default `odcs` feature (drops saphyr), because sql-tdg does not use the ODCS adapter.
2. Maintainer runs `cargo update -p sql-semantic-protocol` to lock the published 2.0.2 crate.
3. Add fixture `tests/fixtures/dbt_manifest_source_relationships.json` in the paper_trail shape:
   - two sources with declared types;
   - a source-level `relationships` test with `attached_node: null` and two dependencies;
   - one model reading a source.
4. Add CLI regression test in `tests/cli.rs`: generation for that manifest succeeds without a catalog. Under 2.0.0 this aborted analysis.
5. Verify `make rust-checks` and `make dbt-e2e`, then finalize.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-08: Maintainer decision: disable the protocol's default `odcs` feature (`default-features = false`), because sql-tdg does not use the ODCS adapter; this drops saphyr. Cargo.toml bumped to `{ version = "2.0.2", default-features = false }`. Added fixture tests/fixtures/dbt_manifest_source_relationships.json and CLI regression test `compiled_cli_generates_dbt_sources_with_unattached_source_relationships_test` in tests/cli.rs. Waiting for the maintainer to run `cargo update -p sql-semantic-protocol`; tests not yet run against 2.0.2.

2026-10-08 verification:
- Maintainer ran `cargo update -p sql-semantic-protocol`; the lock now resolves 2.0.2 from crates.io and removes saphyr, saphyr-parser, arraydeque, foldhash, hashlink 0.12, ordered-float, and thiserror.
- `make rust-checks` passes, including the new test.
- `make dbt-e2e` passes (3 tests).
- The new regression test fails against `=2.0.0` in a scratch copy with the maintainer's paper_trail error (`built-in relationships test must identify its attached resource`) and passes against 2.0.2.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Bumps `sql-semantic-protocol` from 2.0.0 to 2.0.2 and disables its default features.

Why:
- 2.0.0 aborted analysis of any dbt project whose sources carry a `relationships` test (dbt writes `attached_node: null` for source tests), so the paper_trail gate project could not be analyzed.
- 2.0.2 attributes such tests correctly, and reports any it still cannot attribute as `unattributed_dbt_test`.

Changes:
- `Cargo.toml`: `sql-semantic-protocol = { version = "2.0.2", default-features = false }`. Maintainer decision: sql-tdg does not use the ODCS adapter, so its default `odcs` feature is off. That removes saphyr and seven transitive crates from `Cargo.lock`.
- `Cargo.lock`: resolves the published 2.0.2.
- `tests/fixtures/dbt_manifest_source_relationships.json`: a paper_trail-shaped manifest with two typed sources, a source-level relationships test with null `attached_node` and two dependencies, and one filtered model.
- `tests/cli.rs`: `compiled_cli_generates_dbt_sources_with_unattached_source_relationships_test` generates from that manifest without a catalog and checks every row satisfies the model filter.

Tests:
- `make rust-checks` and `make dbt-e2e` pass.
- The new test fails against 2.0.0 with the original paper_trail error, confirming it guards the regression.

Follow-up: consuming the new `unattributed_dbt_test` diagnostic is part of TASK-22.
<!-- SECTION:FINAL_SUMMARY:END -->
