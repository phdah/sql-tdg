---
id: TASK-21.2
title: Use manifest-declared source column data_type when catalog lacks the relation
status: Done
assignee: []
created_date: '2026-10-06 09:18'
updated_date: '2026-10-08 16:03'
labels: []
milestone: m-2
dependencies:
  - TASK-21.8
references:
  - sql-semantic-protocol TASK-34
  - sql-semantic-protocol TASK-48
  - sql-semantic-protocol TASK-49
  - src/main.rs
  - README.md
parent_task_id: TASK-21
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
dbt sources and models may declare `data_type` per column in YAML; the value appears in manifest.json. When catalog.json has no schema for a physical dependency (source tables not yet created in the warehouse), these declared types are legitimate type metadata and should be usable for generation. Per AGENTS.md, dbt artifact interpretation belongs in `sql-semantic-protocol`. Decision (maintainer, 2026-10-06).

Protocol status (verified at protocol commit 3a4d3c6, release 2.0.0):
- `analyze_dbt_artifacts` falls back to complete manifest-declared column types and tags schemas `source_kind: dbt_manifest` (protocol TASK-34).
- A query or constraint reference to a column absent from schema evidence yields an `unknown_schema_column` residual (protocol TASK-48).
- dbt analysis works without catalog.json (`analyze_dbt_manifest_with_schemas`, protocol TASK-49).

Remaining sql-tdg work:
- Accept dbt input without catalog.json. The CLI currently requires a readable catalog file.
- Refuse generation when a model references a column missing from the declared YAML. Today sql-tdg silently writes a source table without that column; through TASK-21.8 this becomes the protocol's `unknown_schema_column` residual.
- Update the `MissingCatalogSchema` CLI hint (`src/main.rs` `dbt_analysis_error`), add one for `MissingDeclaredColumnTypes`, and update the README sentence saying every source must exist in the warehouse.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 sql-semantic-protocol accepts manifest-declared column data_type for physical dependencies absent from catalog.json
- [x] #2 Catalog types take precedence over declared types when both exist
- [x] #3 A relation missing types for any referenced column still fails with an explicit error
- [x] #4 sql-tdg depends on the protocol 2.x release and an end-to-end test generates data for a dbt project whose sources exist only in YAML with declared types
- [x] #5 dbt generation works when catalog.json is absent, using the protocol's catalog-less dbt analysis
- [x] #6 A model referencing a column missing from its source's declared YAML columns fails with an explicit error naming the relation and column, never a table without that column
- [x] #7 CLI hints cover missing catalog schemas and missing declared data_type, and the README describes declaring source column types in YAML
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-08: AC #1-#3 are protocol-side and verified against sql-semantic-protocol 3a4d3c6. #1 was checked by generating from a manifest with declared types and an empty catalog. #2 is covered by the protocol test dbt_catalog_schema_takes_precedence_over_manifest_declared_types. #3 was checked: a declared column without data_type fails with MissingDeclaredColumnTypes, and an undeclared referenced column yields an unknown_schema_column residual.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The CLI uses the protocol's typed manifest analysis when the default dbt catalog is absent.
An explicit --dbt-catalog remains mandatory. Missing source schemas and declared datatypes
produce actionable hints, and the protocol rejects compiled references to undeclared source
columns. CLI integration tests cover typed manifest and project input, range-constrained
generated CSV output, schema provenance, missing types, unknown columns, and missing explicit
catalog paths. README usage documents dbt YAML data_type declarations and catalog precedence.
<!-- SECTION:FINAL_SUMMARY:END -->
