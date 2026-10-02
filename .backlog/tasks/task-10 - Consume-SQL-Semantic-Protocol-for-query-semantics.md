---
id: TASK-10
title: Consume SQL Semantic Protocol for query semantics
status: To Do
assignee: []
created_date: '2026-10-02'
labels: []
dependencies:
  - TASK-9
---

## Description

Replace sql-tdg's internal SQL parsing and semantic constraint derivation with
`phdah/sql-semantic-protocol` as the single semantic input layer.

sql-tdg should remain a test-data generator. SQL parsing, dialect handling, lineage composition,
outcome metadata, and derivation of value domains belong to SQL Semantic Protocol. sql-tdg should
consume that protocol and translate its resolved semantics into generation plans for Arrow-backed
test data.

The preferred integration is library-to-library. sql-tdg may continue to expose a convenient API
that accepts SQL, but that path must delegate semantic analysis to `sql-semantic-protocol`; it
must not maintain an independent SQL parser or duplicate interval/domain derivation.

For protocol documents containing multiple transformation layers, generation must use explicitly
selected outcome semantics. The protocol exposes every layer plus
`graph.components[].final_outcomes`; sql-tdg must not guess when more than one valid outcome could
be selected.

The protocol is still under active development. If generation requires semantic information that
the protocol does not yet represent, extend or wait for the protocol contract rather than
re-deriving that information from SQL inside sql-tdg. In particular, the current protocol exposes
typed literals and value domains but does not yet represent a complete declared datatype schema
for every unconstrained source column, so full schema inference may require an upstream protocol
addition.

## Acceptance Criteria

- [ ] `sql-semantic-protocol` is the sole SQL semantic analysis dependency used by sql-tdg.
- [ ] sql-tdg no longer depends directly on `sqlparser`.
- [ ] The current internal parser module and parser-specific IR are removed once all callers consume protocol types or sql-tdg-owned generation types derived from them.
- [ ] Predicate-to-domain derivation is removed from sql-tdg; ranges, inclusive/exclusive bounds, sets, empty domains, and unknown domains are interpreted from the protocol contract.
- [ ] Resolved protocol `column_domains` are converted into the existing typed solver/generator domains without weakening the protocol semantics.
- [ ] SQL input APIs, if retained, invoke SQL Semantic Protocol internally and then use the resulting protocol exactly like a directly supplied protocol document.
- [ ] The generator can consume a resolved terminal outcome identified through `graph.components[].final_outcomes` and its layer `composed_semantics`.
- [ ] Multiple or ambiguous candidate outcomes require explicit selection rather than an arbitrary default.
- [ ] Unresolved composition, unknown domains, unsupported semantics, missing type information, or other insufficient protocol data return explicit errors; sql-tdg never reparses SQL as a fallback.
- [ ] Any source-column datatype/schema information required for generation is obtained from the protocol contract. If the active protocol version cannot represent it, the missing capability is implemented upstream in `phdah/sql-semantic-protocol` before this task is considered complete.
- [ ] Existing integer, boolean, and timestamp end-to-end generation behavior remains covered using protocol-driven fixtures.
- [ ] Tests demonstrate lower and upper bounds, inclusive and exclusive bounds, excluded values, empty/contradictory domains, and terminal composed semantics.
- [ ] README.md and AGENTS.md describe SQL Semantic Protocol as the semantic boundary and no longer describe sql-tdg as owning SQL parsing or semantic analysis.
- [ ] `make rust-checks` remains green.
