---
id: TASK-21.3
title: Generate one consistent source dataset for all dbt models without --target
status: To Do
assignee: []
created_date: '2026-10-06 09:18'
labels: []
milestone: m-2
dependencies: []
parent_task_id: TASK-21
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Running `sql-tdg generate` with dbt input (or multiple raw SQL inputs) and no `--target`/`--target-layer` currently fails when the project has more than one terminal outcome (`AmbiguousTerminalOutcome`). The maintainer needs one run that produces a complete set of physical source tables for an entire dbt project.

Decided semantics (maintainer, 2026-10-06): every physical source used by any terminal model is generated once. When several terminal models depend on the same source, all of their predicates apply to that source's columns: each generated row must satisfy every dependent model's supported conditions at the same time (intersection). If the combined conditions are contradictory or cannot be guaranteed jointly (e.g. relational witnesses that cannot be combined), generation fails with an explicit error naming the conflicting models; it never silently drops a model or a condition. Sources used by only one model follow that model's semantics. Selecting `--target`/`--target-layer` keeps existing single-outcome behavior.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Without a target selector, generation covers every terminal outcome and emits each physical source relation exactly once
- [ ] #2 Every generated row of a shared source satisfies the supported conditions of every terminal model that reads it
- [ ] #3 Contradictory or not jointly supported conditions across models fail with an explicit error naming the involved outcomes
- [ ] #4 Explicit --target/--target-layer behavior is unchanged
- [ ] #5 Metadata records that the run targets all terminal outcomes
- [ ] #6 Output is deterministic for the same inputs and seed
- [ ] #7 Library and CLI tests cover disjoint sources, shared sources, and conflicting models
- [ ] #8 README/CLI help document whole-project generation
<!-- AC:END -->
