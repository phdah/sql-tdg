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
dependencies:
  - TASK-37
  - TASK-38
  - TASK-43

references:
  - 'sql-semantic-protocol TASK-86'
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

**Approved maintainer definition (2026-10-09):** rejection is **per terminal outcome**. A physical row may be qualifying for terminal A while provably rejected by terminal B, with full membership classification stored in generated metadata. For every rejected row, choose **randomly across all provably constructive rejecting predicate/column alternatives**, using the seeded generator RNG. Multiple seeds must demonstrate every feasible alternative can be selected; a fixed seed must be reproducible. Falsifying only one arm of an OR is not rejection if another arm still passes. The protocol must supply complete typed proof alternatives, not sql-tdg-local SQL analysis. See [approved scope](https://github.com/phdah/sql-semantic-protocol/blob/feat/task-66-dialect-feature-inventory/docs/coverage-signoff.md).

Semantics, lineage and predicate analysis belong to SQL Semantic Protocol. If the chosen semantics need information the protocol does not provide, implement and release the protocol prerequisite first; do not add interpretation fallbacks in sql-tdg. Keep exactness errors: when rejected rows cannot be guaranteed for some outcome, fail explicitly naming that outcome instead of silently producing fewer or mislabeled rows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Maintainer-approved definition of a rejected row across multiple terminal outcomes is recorded in this task before implementation starts
- [ ] #2 Every generated rejected row satisfies the chosen definition and every matching row still satisfies all dependent terminal models
- [ ] #3 Rejected rows still satisfy honored keys, foreign keys and dbt data-test constraints on generated relations
- [ ] #4 Metadata row classifications identify rejected rows and the terminal outcome(s) they are rejected by
- [ ] #5 When rejected rows cannot be guaranteed for an outcome, generation fails with an explicit error naming the outcome
- [ ] #6 Output is deterministic for the same inputs, row counts and seed
- [ ] #7 --scenarios behavior is either extended consistently or keeps an explicit, documented restriction
- [ ] #8 Library, CLI and dbt DuckDB e2e tests cover shared sources, disjoint sources, conflicting outcomes and the default --rejected 10 invocation
- [ ] #9 README, docs/usage.md and CLI help describe rejected-row semantics in whole-project mode
- [ ] #10 Whole-project generation succeeds with the default --rejected 10 (no --rejected flag passed) for projects whose terminal models are all supported
- [ ] #11 After maintainer approval of the rejection definition, require protocol-issued per-terminal classification vectors and independently provable absence; do not infer outcome membership from source predicates.
- [ ] #12 Choose each rejected row's failing predicate/column from every **eligible and provably rejecting** alternative via the injected seeded RNG. Repeatability for a fixed seed, actual cross-seed diversity for at least two different columns/predicates, and eventual coverage of *all* eligible alternatives must have automated tests.
- [ ] #13 Evaluate composite AND/OR/NOT/NULL predicates and multi-layer lineage before selecting the alternative; violating one predicate only counts when the entire terminal remains provably absent. Include negative tests where OR's other arm, a set branch, a join match or DML state makes a nominally 'rejected' row pass.
- [ ] #14 Exact matching/rejection counts, FK/unique/dbt constraints and all terminal classifications hold simultaneously; no alternative is silently discarded, and infeasible or unproven choices fail with typed diagnostics.
<!-- AC:END -->

## Protocol v3 release gate (2026-10-09)

**Release dependency:** This task must be validated against the upstream single 3.0.0 release candidate pinned by sql-tdg TASK-43; the protocol release remains blocked on upstream TASK-91. Original criteria remain binding. Do not mark Done using only an operator-local proof where the physical-source DAG cannot realize it. Fully execute SQL to verify terminal rows and deliberately rejected cases, and document supported/residual variants in the feature-by-dialect matrix. Companion planning PR: https://github.com/phdah/sql-semantic-protocol/pull/87.

## 2026-10-09 maintainer decision

**Decision complete, implementation pending.** Rejected rows are per-terminal, and failure predicate/column is selected randomly from protocol-certified constructive alternatives using the supplied seed. Do not confuse stochastic selection with uncertain correctness: every generated negative row still requires an exact absence proof and actual E2E execution check. Owner: protocol TASK-86; generator task TASK-35; final acceptance TASK-36. This preserves the default `--rejected 10` in whole-project mode.
