---
id: TASK-22
title: Honor dbt data tests in generated data
status: To Do
assignee: []
created_date: '2026-10-06 12:39'
updated_date: '2026-10-08 14:51'
labels: []
dependencies:
  - TASK-21.8
  - TASK-21.9
references:
  - sql-semantic-protocol TASK-30
  - sql-semantic-protocol TASK-33
  - sql-semantic-protocol TASK-40
  - sql-semantic-protocol TASK-41
  - sql-semantic-protocol TASK-42
  - sql-semantic-protocol TASK-48
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
dbt generic data tests declared in YAML (unique, not_null, accepted_values, relationships) are constraints the generated data must satisfy for `dbt build` to pass. On the maintainer fixture, generated sources fail 4 source tests (duplicate ids, null product_id, product_id outside accepted values). sql-tdg must consume these constraints through sql-semantic-protocol, never by reading dbt YAML or SQL itself. Every generated row must satisfy every supported declared test; unsupported tests fail explicitly or are reported, never silently ignored.

Protocol status (verified at protocol commit 3a4d3c6, release 2.0.0):
- `AnalysisBundle::relation_constraints()` returns per-relation `RelationConstraintSet`s with PrimaryKey, UniqueKey (including composite), ForeignKey, NotNull, and AcceptedValues, extracted from dbt tests and dbt constraints (protocol TASK-30, TASK-33).
- Foreign-key targets are canonical relation names; unresolvable targets fail analysis (protocol TASK-40).
- Unsupported metadata produces diagnostics, never silence:
  - codes: `unsupported_dbt_test`, `unsupported_dbt_singular_test`, `unsupported_dbt_test_config` (where, severity, thresholds), `unsupported_check_constraint`, `unsupported_dbt_constraint`, `unsupported_dbt_accepted_value`, `unsatisfiable_accepted_values`, `conflicting_accepted_values_quoting`;
  - location: on `RelationConstraintSet::diagnostics()`, or bundle-level `AnalysisBundle::constraint_diagnostics()` when no relation applies (protocol TASK-41).
- NULL semantics are explicit through `RelationConstraint::admits_null()`. Accepted values are typed, including `quote: false` (protocol TASK-42).
- Constraint columns are validated against schema evidence (protocol TASK-48).
- Constraints are not propagated through transformations, so a test on a derived model can only be honored when that model is a generation boundary.
- `RelationConstraint` and `ConstraintValue` are non-exhaustive, so sql-tdg needs explicit unsupported arms.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 sql-tdg depends on a protocol 2.x release exposing canonical unique, not-null, accepted-values, and foreign-key constraints from dbt artifacts
- [ ] #2 Generated data satisfies unique and not_null constraints
- [ ] #3 Generated data satisfies accepted_values constraints
- [ ] #4 Generated data satisfies relationships (foreign key) constraints across generated relations
- [ ] #5 Constraints conflicting with query domains fail with an explicit error
- [ ] #6 A dbt end-to-end test runs `dbt build` (including data tests) successfully on generated sources
- [ ] #7 Any constraint diagnostic on a generated relation, and any bundle-level constraint diagnostic, fails generation with an explicit error naming its code
- [ ] #8 NULL handling follows RelationConstraint::admits_null, and accepted values are checked against the column datatype before sampling
- [ ] #9 Composite keys and self-referencing foreign keys are generated correctly or fail explicitly
- [ ] #10 Constraints on relations that are neither generated sources nor selected boundaries are reported as not honored rather than silently ignored
- [ ] #11 Unknown future RelationConstraint or ConstraintValue variants fail with an explicit unsupported error
- [ ] #12 The `unattributed_dbt_test` diagnostic from protocol 2.0.2 fails generation like every other constraint diagnostic, and a test asserts this
- [ ] #13 The maintainer's paper_trail fixture generates sources that pass every dbt source test it declares (unique, not_null, accepted_values, and the order_items to orders relationship)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-08: Protocol 2.0.2 (PR #72) adds the constraint diagnostic code `unattributed_dbt_test` (bundle-level `constraint_diagnostics`) for built-in dbt tests whose tested resource cannot be identified. Source-level tests, where dbt writes attached_node: null, are now attributed from the test's `kwargs.model` argument, including relationships between sources and self-referencing ones. Verified on the maintainer's paper_trail fixture: all 19 generic tests attach to the right relation; the two singular tests are reported as `unsupported_dbt_singular_test` on daily_revenue. Constraints on daily_revenue (a model, not a generated source) must be reported as not honored under AC #10.
<!-- SECTION:NOTES:END -->
