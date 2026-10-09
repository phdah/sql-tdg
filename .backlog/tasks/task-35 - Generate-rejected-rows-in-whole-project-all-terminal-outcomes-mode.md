---
id: TASK-35
title: Generate rejected rows in whole-project (all terminal outcomes) mode
status: To Do
assignee: []
created_date: '2026-10-09 15:48'
updated_date: '2026-10-09 15:48'
labels:
  - dbt
  - cli
  - generation
milestone: m-3
dependencies: []
references:
  - TASK-21.3
  - TASK-13
  - TASK-14
  - TASK-30
  - src/protocol.rs (InvalidRowConfiguration for rejected rows in joint mode)
  - src/main.rs (CLI default --rejected 10)
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Whole-project generation (no --target/--target-layer, several terminal outcomes; TASK-21.3) produces one shared physical-source dataset whose rows satisfy every dependent terminal model. It currently refuses any rejected rows with `InvalidRowConfiguration: rejected rows are not defined when generating for all terminal outcomes; select one terminal outcome`, because TASK-21.3 deliberately left the meaning of a rejected row across multiple outcomes undefined.

This conflicts with the product goal: sql-tdg should take every model of a dbt project into account and generate source data that produces both matching and rejected outcomes after all transformations. It also makes the CLI defaults contradictory: `--rejected` defaults to 10 and omitting a target means whole-project mode, so a plain `sql-tdg generate --dbt-project .` fails on any project with more than one terminal model.

Maintainer decision (2026-10-09): the CLI default stays `--rejected 10` in every mode, including whole-project mode; the default invocation must work rather than be changed to 0.

Open decision (must be settled with the maintainer before implementation): what a rejected source row means when one source feeds several terminal models with different predicates (e.g. dbt fixture `orders` feeds stg_orders, final_orders, boundary_final_orders, derived_orders). Candidate semantics:
- Rejected by every terminal model: the row reaches no terminal output. Simple to verify, but never exercises "passes model A, fails model B".
- Per-model rejected rows: for each terminal model, rows that fail that model while still satisfying keys, relationships and data-test constraints; metadata records which outcomes each row is rejected by. Richer, but such rows may legitimately appear in other models' outputs.
- Another definition the maintainer chooses.

Semantics, lineage and predicate analysis belong to SQL Semantic Protocol. If the chosen semantics need information the protocol does not provide, implement and release the protocol prerequisite first; do not add interpretation fallbacks in sql-tdg. Keep exactness errors: when rejected rows cannot be guaranteed for some outcome, fail explicitly naming that outcome instead of silently producing fewer or mislabeled rows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Maintainer-approved definition of a rejected row across multiple terminal outcomes is recorded in this task before implementation starts
- [ ] #2 Every generated rejected row satisfies the chosen definition and every matching row still satisfies all dependent terminal models
- [ ] #3 Rejected rows still satisfy honored keys, foreign keys and dbt data-test constraints on generated relations
- [ ] #4 Metadata row classifications identify rejected rows and the terminal outcome(s) they are rejected by
- [ ] #5 When rejected rows cannot be guaranteed for an outcome, generation fails with an explicit error naming the outcome
- [ ] #6 Output is deterministic for the same inputs, row counts and seed
- [ ] #7 --scenarios behavior is either extended consistently or keeps an explicit, documented restriction
- [ ] #8 Library, CLI and dbt DuckDB e2e tests cover shared sources, disjoint sources, conflicting outcomes and the default --rejected 10 invocation
- [ ] #9 README, docs/usage.md and CLI help describe rejected-row semantics in whole-project mode
- [ ] #10 Whole-project generation succeeds with the default --rejected 10 (no --rejected flag passed) for projects whose terminal models are all supported
<!-- AC:END -->
