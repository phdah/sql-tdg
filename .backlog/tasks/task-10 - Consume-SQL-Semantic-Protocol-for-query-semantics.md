---
id: TASK-10
title: Consume SQL Semantic Protocol for query semantics
status: In Progress
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
the protocol does not yet represent, extend the protocol contract rather than re-deriving that
information from SQL inside sql-tdg. TASK-10 is therefore also the consumer acceptance gate for
SQL Semantic Protocol 1.0.0.

## Acceptance Criteria

- [x] `sql-semantic-protocol` is the sole SQL semantic analysis dependency used by sql-tdg.
- [x] sql-tdg no longer depends directly on `sqlparser`.
- [x] The current internal parser module and parser-specific IR are removed once all callers consume protocol types or sql-tdg-owned generation types derived from them.
- [x] Predicate-to-domain derivation is removed from sql-tdg; ranges, inclusive/exclusive bounds, sets, empty domains, and unknown domains are interpreted from the protocol contract.
- [x] Resolved protocol `column_domains` are converted into the existing typed solver/generator domains without weakening the protocol semantics.
- [x] SQL input APIs, if retained, invoke SQL Semantic Protocol internally and then use the resulting protocol exactly like a directly supplied protocol document.
- [x] The generator can consume a resolved terminal outcome identified through `graph.components[].final_outcomes` and its layer `composed_semantics`.
- [x] Multiple or ambiguous candidate outcomes require explicit selection rather than an arbitrary default.
- [x] Unresolved composition, unknown domains, unsupported semantics, missing type information, or other insufficient protocol data return explicit errors; sql-tdg never reparses SQL as a fallback.
- [x] Any source-column datatype/schema information required for generation is obtained from the protocol contract. If the active protocol version cannot represent it, the missing capability is implemented upstream in `phdah/sql-semantic-protocol` before this task is considered complete.
- [x] Existing integer, boolean, and timestamp end-to-end generation behavior remains covered using protocol-driven fixtures.
- [x] Tests demonstrate lower and upper bounds, inclusive and exclusive bounds, excluded values, empty/contradictory domains, and terminal composed semantics.
- [x] README.md and AGENTS.md describe SQL Semantic Protocol as the semantic boundary and no longer describe sql-tdg as owning SQL parsing or semantic analysis.
- [x] `make rust-checks` remains green.


## Protocol 1.0 release gate

Consumer integration identified source-schema and datatype normalization as missing pieces in the
pre-release protocol contract. They are implemented in `phdah/sql-semantic-protocol#31`.

The required 1.0 contract now includes:

- typed source relation schemas for unconstrained source columns
- a recursive parser-independent canonical datatype model
- dialect-specific datatype syntax normalized at the protocol boundary
- explicit preservation of vendor/user-defined custom types
- dialect lookup through the protocol crate so consumers do not depend directly on sqlparser

TASK-10 remains In Progress until protocol PR #31 is merged and sql-tdg is switched from the
pre-release branch dependency to the released 1.0.0 crate. Green consumer tests against #31 are
the approval signal that the protocol contract itself is sufficient for 1.0.0.
