---
id: TASK-43
title: Adopt a pinned protocol v3 candidate before crate publication
status: To Do
assignee: []
created_date: '2026-10-09'
updated_date: '2026-10-09'
labels: []
milestone: m-3
dependencies: []
references:
  - 'sql-semantic-protocol TASK-90'
  - 'sql-semantic-protocol TASK-91'
  - 'sql-semantic-protocol PR #87'
  - 'sql-semantic-protocol TASK-99 / PR #105'
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generator-side implementation for the single SQL Semantic Protocol 3.0.0 contract. This task must not reinterpret SQL, guess missing semantics or claim partial/local operator witnesses imply complete physical-source/output correctness. Validate with the pinned unpublished protocol release candidate before publication, then with the published crate. The protocol itself owns the canonical SQL/dbt conformance fixtures and expected semantic outcomes. Consume the suite from the same pinned protocol revision as the library dependency; do not make this consumer repository the authority for SQL meaning or maintain a second canonical copy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 Create a documented pre-release integration branch/CI path consuming the protocol candidate through a pinned Git commit SHA and explicit Cargo lock resolution (never floating main).
- [ ] #2 Adapt versioned API/JSON emission and strict typed capability checks; preserve existing sql-tdg 1.0.0 behavior until the new witness solver is enabled.
- [ ] #3 Run Rust formatting, clippy, unit tests, docs and dbt E2E against the candidate, with deterministic fixtures and compatibility diagnostics.
- [ ] #4 Prevent a dependency cycle: execute protocol+generator acceptance before protocol 3.0.0 exists, then switch to exactly published protocol 3.0.0 and verify the same fixture signatures.
- [ ] #5 Document compatibility evidence and the precise pinned commit used for protocol TASK-91 sign-off.
- [ ] #6 Document supported versus residual variants, add full-result differential tests and update docs/dialect-conformance.md for every changed feature.
- [ ] #7 Verify the pinned protocol candidate includes approved multi-terminal randomized rejecting-alternative contracts, documented conditional future support for opaque behavior, and equivalent canonical semantic outputs for every advertised dialect/feature pair.
- [ ] #8 Consume the protocol-owned `tests/fixtures/conformance/v1/suite.json`, SQL cases, dbt models, source seeds and expected outcomes from the **same pinned protocol Git SHA** resolved by Cargo. Assert suite revision/path compatibility in CI; never pin sql-tdg from the protocol or treat sql-tdg copies as canonical.
- [ ] #9 Map the suite's canonical typed feasible, impossible and residual classifications to generator capabilities. Run consumer-specific generation/output checks separately, fail closed on unsupported physical construction, and document any extensions to the suite in the protocol repository rather than duplicating semantic expectations locally.
<!-- AC:END -->

## Approved upstream scope (2026-10-09)

Scope decisions 1–4 are approved in upstream protocol PR #88, but upstream TASK-91 remains closed for release. The candidate must be pinned by a Git SHA and validated with the unified dbt/DDL/DML `make all` plus 13-dialect canonical equivalence tests before publication.

## Protocol-owned conformance handoff (2026-10-12)

Upstream https://github.com/phdah/sql-semantic-protocol/pull/105 establishes `tests/fixtures/conformance/v1/suite.json` as the
versioned protocol-owned conformance inventory. Version `v1` identifies the
fixture manifest format, **not** protocol 3.0.0. The exact protocol Git SHA
chosen under AC #1 must own both the Rust library and the fixture/expected
outcome inputs used in CI. The initial fixtures were historically adopted
from sql-tdg, but the dependency direction is now protocol -> consumer:
sql-tdg depends on the protocol, not vice versa. TASK-36 and TASK-44 consume
this handoff; generator-only randomized or execution tests remain here.
Protocol TASK-91 and this task's release gate remain blocked pending E2E
acceptance. This note records required work; it is not completion evidence.
