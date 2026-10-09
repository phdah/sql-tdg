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
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generator-side implementation for the single SQL Semantic Protocol 3.0.0 contract. This task must not reinterpret SQL, guess missing semantics or claim partial/local operator witnesses imply complete physical-source/output correctness. Validate with the pinned unpublished protocol release candidate before publication, then with the published crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 Create a documented pre-release integration branch/CI path consuming the protocol candidate through a pinned Git commit SHA and explicit Cargo lock resolution (never floating main).
- [ ] #2 Adapt versioned API/JSON emission and strict typed capability checks; preserve existing sql-tdg 1.0.0 behavior until the new witness solver is enabled.
- [ ] #3 Run Rust formatting, clippy, unit tests, docs and dbt E2E against the candidate, with deterministic fixtures and compatibility diagnostics.
- [ ] #4 Prevent a dependency cycle: execute protocol+generator acceptance before protocol 3.0.0 exists, then switch to exactly published protocol 3.0.0 and verify the same fixture signatures.
- [ ] #5 Document compatibility evidence and the precise pinned commit used for protocol TASK-91 sign-off.
- [ ] #6 Document supported versus residual variants, add full-result differential tests and update docs/dialect-conformance.md for every changed feature.
<!-- AC:END -->
