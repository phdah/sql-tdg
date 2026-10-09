---
id: m-3
title: Broaden exact SQL test-data generation
---

## Description

Expand sql-tdg after its stable CLI release beyond exact scalar domains and inner equality joins, so it can construct deterministic witness datasets for analytic outputs, set operations, richer joins, subquery predicates, computed conditions and DML state effects. Add optional result-cardinality and scenario goals and a dialect conformance matrix.

## Protocol 3.0 release-candidate strategy (decision 2026-10-09)

The previous protocol TASK-58..65 proofs were useful operator-local foundations but do not prove arbitrary combined transformations or physical-source realizability. Protocol tasks **TASK-66..91** (tracked in [protocol PR #87](https://github.com/phdah/sql-semantic-protocol/pull/87)) now gate the **single v3.0.0 release**. No intermediate protocol releases are required.

The m-3 milestone must exercise all maintainer-approved sql-tdg use cases across raw SQL, dbt, named CTE/producers, repeated sources, joins, sets, aggregation/HAVING, windows/QUALIFY, subqueries, scalar predicates, nested relations, cardinality/distributions, positive/rejected multi-terminal outcomes and CREATE/REPLACE/ALTER/DROP/TRUNCATE/INSERT/UPDATE/DELETE/MERGE/UPSERT scripted states. Additive generator tasks TASK-37..43 cover a global physical-source witness solver, cross-operator goals, stateful DDL/DML fixture production, full type/constraint coverage, advanced table sources, executable dialect and cross-feature matrix, and pinned candidate integration.

**No circular release dependency:** implement and test against a pinned Git SHA for the unpublished protocol v3 candidate; after upstream contract and combined E2E certification, merge protocol Release Please PR #79 and publish v3.0.0 once, then switch Cargo to the published version. The final m-3 gate is sql-tdg TASK-36, which depends on TASK-24..31, TASK-35, and TASK-37..43. Each completed generator task must use canonical protocol obligations only and pass realistic full-output SQL engine comparisons.

An exhaustive guarantee for *arbitrary* SQL/UDFs, stochastic sampling or unbounded recursion is not sound. The finite release-blocking feature-by-dialect matrix in upstream TASK-66 defines coverage. Every supported variant must be exact and execution-certified where an engine is available; every unprovable/unsupported variant must fail closed with a reviewed explicit exclusion, never a silent fallback.
