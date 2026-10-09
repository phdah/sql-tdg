---
id: TASK-44
title: Certify equivalent canonical outcomes for every supported dialect
status: To Do
assignee: []
created_date: '2026-10-09'
updated_date: '2026-10-09'
labels:
  - dialects
  - conformance
  - e2e
milestone: m-3
dependencies:
  - TASK-43
references:
  - 'sql-semantic-protocol TASK-66'
  - 'sql-semantic-protocol TASK-88'
  - 'sql-semantic-protocol TASK-89'
  - 'TASK-33'
  - 'TASK-36'
priority: high
type: task
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-33 established a **representative** dialect matrix. The maintainer-approved v3 release gate is stronger: every advertised dialect/feature/variant must be tested for parser acceptance and identical **parser-independent canonical protocol outcomes** where the SQL has the same meaning, and end-to-end generation must be checked using equivalent executable DuckDB SQL. Parsing success alone does not count as evidence for protocol or generator exactness. This task does not reopen completed TASK-33; it establishes the additional mandatory m-3 certification scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Consume upstream TASK-66's approved manifest as a single authoritative feature/variant/dialect inventory and fail CI when supported rows lack test evidence; do not hard-code a separate production SQL dialect whitelist.
- [ ] #2 For shared semantic SQL and every advertised supported variant, analyze all thirteen exposed dialect families and compare canonical protocol facts (physical lineage, domains and bound inclusivity, NULL/three-valued logic, exact positive/rejected witnesses, cardinality, ordered DML/DDL state effects, and residual classification); ignore only explicitly nonsemantic dialect provenance.
- [ ] #3 For genuinely dialect-specific syntax (QUALIFY, TOP, FETCH, MINUS, BY NAME, UPSERT, INSERT OVERWRITE, MERGE, ASOF/APPLY, CREATE OR REPLACE etc.) provide semantically equivalent alternative syntax, specific canonical equality expectations, or typed differential/residual diagnostics. Record engine/session assumptions where meaning differs; never force false equality.
- [ ] #4 Evaluate generated physical sources under DuckDB's equivalent executable SQL, including joined group/window/set/CTE combinations and ordered DML/DDL scripts. Test complete result values, multiplicities and requested per-terminal membership rather than only row counts.
- [ ] #5 Explicitly distinguish parser/canonical/generator proof from native engine execution: DuckDB E2E is mandatory; unprovisioned BigQuery, Snowflake, MSSQL, PostgreSQL, MySQL, etc. are not labeled execution-certified.
- [ ] #6 Update docs/dialect-conformance.md, upstream coverage manifest, runnable fixture tests and the pinned candidate SHA documentation; block final TASK-36/91 sign-off for any uncovered advertised support.
<!-- AC:END -->

## Delivery guidance

Required for the single protocol 3.0.0 and sql-tdg m-3 acceptance before Release Please PR #79. Depends on the upstream TASK-88 dialect equivalence semantics and pinned protocol candidate in TASK-43. Parser tests require no external vendor database; use DuckDB only for independently executable equivalent shapes. The scope approval itself is documented in protocol PR #88.
