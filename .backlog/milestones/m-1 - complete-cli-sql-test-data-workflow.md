---
id: m-1
title: "Complete CLI SQL test-data workflow"
---

## Description

Turn sql-tdg into a complete command-line workflow for generating deterministic, feature-rich SQL
test data and executing real SQL workloads against it.

The milestone covers both raw SQL workloads and full dbt Core projects using DuckDB as the initial
execution engine. Generated datasets must include rows that satisfy the selected semantics and rows
that are deliberately rejected by them, so a test workload exercises both sides of its filters and
relationships rather than containing only happy-path data.

Generation must work at physical source boundaries and at selected intermediate relation boundaries.
This allows callers to test an entire transformation graph from raw sources or isolate a downstream
query/model by materializing generated data for its immediate upstream relations.

SQL Semantic Protocol remains the sole semantic boundary. If a required matching, rejection, schema,
relationship, or layer semantic is not represented by the protocol, extend the protocol upstream
rather than reparsing SQL or deriving a second SQL semantic model inside sql-tdg.

The milestone is complete when the CLI can create a deterministic DuckDB-backed test case, execute
advanced raw SQL and a representative dbt project, compare results against an approved expected
output, and fail when a query change alters observable behavior.
