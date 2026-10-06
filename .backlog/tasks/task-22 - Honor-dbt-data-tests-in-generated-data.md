---
id: TASK-22
title: Honor dbt data tests in generated data
status: To Do
assignee: []
created_date: '2026-10-06 12:39'
updated_date: '2026-10-06 13:26'
labels: []
dependencies: []
references:
  - >-
    sql-semantic-protocol TASK-33 (not-null and accepted-values constraints,
    milestone 1.1.0)
  - >-
    sql-semantic-protocol TASK-29 and TASK-30 (key and foreign-key constraints,
    milestone 1.1.0)
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
dbt generic data tests declared in YAML (unique, not_null, accepted_values, relationships) are constraints the generated data must satisfy for `dbt build` to pass. On the maintainer fixture, generated sources fail 4 source tests (duplicate ids, null product_id, product_id outside accepted values). sql-tdg must consume these constraints through sql-semantic-protocol, never by reading dbt YAML or SQL itself. The protocol 1.1.0 roadmap (its TASK-29/TASK-30) covers primary/unique/foreign keys; not_null and accepted_values are not yet planned there and need a protocol feature. Every generated row must satisfy every supported declared test; unsupported tests fail explicitly or are reported, never silently ignored.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 sql-tdg depends on a protocol release exposing canonical unique, not-null, accepted-values, and foreign-key constraints from dbt artifacts
- [ ] #2 Generated data satisfies unique and not_null constraints
- [ ] #3 Generated data satisfies accepted_values constraints
- [ ] #4 Generated data satisfies relationships (foreign key) constraints across generated relations
- [ ] #5 Constraints conflicting with query domains fail with an explicit error
- [ ] #6 A dbt end-to-end test runs `dbt build` (including data tests) successfully on generated sources
<!-- AC:END -->
