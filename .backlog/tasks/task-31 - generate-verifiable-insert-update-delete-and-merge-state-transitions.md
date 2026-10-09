---
id: TASK-31
title: Generate verifiable INSERT UPDATE DELETE and MERGE state transitions
status: To Do
assignee: []
created_date: '2026-10-08'
labels: []
milestone: m-3
dependencies:
  - TASK-37
  - TASK-39
  - TASK-40
  - TASK-43

references:
  - 'sql-semantic-protocol TASK-79'
  - 'sql-semantic-protocol TASK-80'
  - 'sql-semantic-protocol TASK-81'
  - 'sql-semantic-protocol TASK-82'
  - 'sql-semantic-protocol TASK-84'
  - 'sql-semantic-protocol TASK-65'
  - 'TASK-18'
  - 'TASK-19'
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Current tests cover INSERT and protocol models partial MERGE semantics, but sql-tdg cannot synthesize guaranteed pre-state and post-state transitions for richer incremental SQL. Depend on upstream TASK-65.

This is post-1.0 feature work, not a claim about existing CLI behavior. Canonical semantics, dialect resolution, lineage and SQL analysis belong solely to SQL Semantic Protocol. If upstream semantics are missing, implement and release the protocol prerequisite first; do not add interpretation fallbacks in sql-tdg. Preserve deterministic outputs and exactness errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Consume protocol-declared initial-state requirements, mutation branches, effect domains and post-state constraints; no local SQL DML semantic parser.
- [ ] #2 Generate source and initial target tables for a safe subset of INSERT, UPDATE, DELETE and MERGE with deterministic touched/untouched rows.
- [ ] #3 Honor key/foreign-key constraints, matched/unmatched branches, source filters and exact after-state obligations; fail closed for unsupported effect semantics.
- [ ] #4 Execute real DuckDB mutations and assert before/after snapshots, idempotence where specified and intentionally nonmatching cases.
- [ ] #5 Document incremental-model capabilities, limitations and how exported files are loaded by callers.
- [ ] #6 Provide verifiable prestate, target effects, constraints, afterstate and idempotence for the reviewed INSERT/UPDATE/DELETE/MERGE/UPSERT and dialect-specific branches, including joined updates and conflicts.
- [ ] #7 Include reviewed DDL producers and schema evolution (CREATE, CTAS, CREATE VIEW, CREATE OR REPLACE, ALTER, DROP, and dependent rewrites) and ordered transaction/effect semantics alongside DML, with fail-closed errors for any unsupported case.
- [ ] #8 The canonical dbt project workflow `make all` executes **both** native dbt DAG/incremental workflows and a mandatory DuckDB scripted mutation/DDL harness; check full prestate/poststate contents, multiplicities, impacted and untouched rows, positive and randomized negative classification and relevant constraints.
- [ ] #9 For vendor-only syntax use the native protocol parser and compare its canonical meaning to equivalent DuckDB-executable syntax when sound, without claiming execution against unavailable vendor databases.
<!-- AC:END -->

## Protocol v3 release gate (2026-10-09)

**Release dependency:** This task must be validated against the upstream single 3.0.0 release candidate pinned by sql-tdg TASK-43; the protocol release remains blocked on upstream TASK-91. Original criteria remain binding. Do not mark Done using only an operator-local proof where the physical-source DAG cannot realize it. Fully execute SQL to verify terminal rows and deliberately rejected cases, and document supported/residual variants in the feature-by-dialect matrix. Companion planning PR: https://github.com/phdah/sql-semantic-protocol/pull/87.

## Approved m-3 gate (2026-10-09)

The maintainer approved **rebuilding and extending the dbt E2E**, rather than separating DML/DDL acceptance from the demo. A scripted DuckDB DML/DDL harness is allowed and required for statements dbt cannot model directly, but it **must run under the same top-level dbt fixture `make all` and CI job**, with failure propagation. No unsupported SQL should be silently counted as covered. See docs/m3-acceptance-plan.md and protocol TASK-80..84, TASK-89.
