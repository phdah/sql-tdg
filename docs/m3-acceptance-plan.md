# m-3 unified generator acceptance plan

**Status: maintainer scope approved 2026-10-09; implementation and final acceptance pending.** This plan defines what the existing [dbt fixture](../tests/fixtures/dbt_core_project/Makefile) and CI **must become**, not what they already do. [Protocol decision record](https://github.com/phdah/sql-semantic-protocol/blob/feat/task-66-dialect-feature-inventory/docs/coverage-signoff.md).

## Single entry point

The committed fixture **`make all`** must be the one CI and maintainer release gate (TASK-36). It must execute the following ordered stages, failing immediately on any incorrect state:

| Stage | Required actions | Independently verified evidence |
| --- | --- | --- |
| 1. Source/schema preparation | Build/provision temporary DuckDB warehouse, bootstrap dbt sources, `dbt compile`, `dbt docs generate`, inspect manifest+catalog | Physical source and target relation types, declared keys/FKs/uniques/tests, deterministic fixtures |
| 2. Protocol-driven generation | Generate one shared compatible dataset for all terminal dbt models, **100 matching + 10 per-terminal rejected by default**, with pinned v3 Git SHA and seed | Complete source data, typed per-row/per-terminal membership vector, failure-alternative ID and seed |
| 3. dbt DAG execution | Seed all generated sources; run all model/view/table/incremental/snapshot materializations and dbt data tests (or explicit supported equivalent) | Full sorted/multiset contents for *every* terminal and relevant intermediate, not merely row counts |
| 4. Required DDL/DML scripted harness | Invoke committed DuckDB mutation scripts through the **same `make all`**; load generated pre-state then run ordered `CREATE`, `REPLACE`, `ALTER`, `DROP`, `CTAS`, `INSERT`, `UPDATE`, `DELETE`, `MERGE`, conflict/UPSERT, transaction scenarios | Exact initial/final states and effects, touched/untouched rows, keys, NULL semantics, idempotence where claimed, all negative witnesses |
| 5. Cross-feature oracle | Query every supported approved combination and compare full SQL output values, including bag multiplicities, order when semantically required, aggregate/window results, generated positive and rejected identifiers | Machine-readable expected/observed snapshots and reproducible failure counterexample |
| 6. Dialect conformance | Run protocol parser/analysis for every supported variant and all 13 named dialects; compare identical **canonical** protocol outcome for semantically equivalent syntax and explicit residual when semantics differ | Dialect provenance excluded only from equivalence comparison; per-variant parser, canonical, generator and DuckDB-engine proof recorded separately |
| 7. Cleanup/repeatability | Re-run identical seed, test different seeds, `make clean` and check tracked fixture unchanged | Stable byte-for-byte generation; all eligible rejected alternatives covered **across seeds**; no leftover warehouse/artifacts |

The Makefile `all` target currently only runs `bootstrap generate load verify dbt-run results` and prints counts. Tasks 31, 35, 36 and 44 must implement the stages above **before** this plan can be marked complete. All new test targets should be explicitly dependencies of the *existing* `all` target (not optional steps or a manually assembled second command). Rust `make dbt-e2e` and CI must invoke the same gate or a faithfully equivalent checked artifact.

## Random rejection contract: testable examples

Every generated physical row has **stable source-row identity**, chosen failing alternative and per-terminal `qualifies/rejected/impossible/residual` classification. A negative for terminal T is a provably absent contributor from **T's final SQL result**, including correlated paths and mutation effects. It may still contribute to another terminal.

| SQL shape | Feasible randomized alternatives | Verification trap |
| --- | --- | --- |
| `WHERE a >= 10 AND b IS NOT NULL AND c = 2` | Violate a, b or c, if types/constraints permit, selected via seeded RNG | Vary seeds until each feasible column gets selected, while all output rows for terminal T exclude that source |
| `WHERE a = 1 OR b = 1` | Make **both** arms non-TRUE; protocol supplies a valid whole-condition negative proof | Failing only a is **not** a rejected case when b=1 |
| `NOT (a > 0)`, `IS NULL`, `IN` with NULL | Use SQL three-valued truth, not two-valued Boolean negation | UNKNOWN may not pass WHERE but must be labeled according to protocol's exact truth contract |
| `INNER JOIN`, `EXISTS` and set branches | Disrupt a relevant join/EXISTS/set branch only when final contribution is impossible across *all* paths | A matching alternative join branch must not turn a supposed rejected row positive |
| `GROUP BY/HAVING`, `QUALIFY` and DML | Select only coordinated physical-source plans that produce provable absence after materialization | A one-column change does not generally prove the group/window/post-DML membership is absent |

Selection **must** use the generator's injected RNG, respect deterministic fixed seeds and sample from every *eligible* protocol-certified alternative (never choose unsupported candidates and call them covered). Independent DuckDB execution validates membership; test enough seeds to attain all eligible alternatives, and separately enforce exact feasibility/infeasibility classification. A single random run need not cover all choices but must always produce valid negatives.

## Required coverage families and source of truth

The authoritative inventory is protocol [docs/coverage-manifest.json](https://github.com/phdah/sql-semantic-protocol/blob/feat/task-66-dialect-feature-inventory/docs/coverage-manifest.json). Extend the dbt models and scripted harness until every release-approved **supported** variant is exercised. In particular cover extended and self-joins, set ALL/DISTINCT and null multiplicities, CTE/scope, EXISTS/IN/correlated subqueries, scalar and Boolean predicates, casts/CASE/types, groups/HAVING, windows/QUALIFY/rank ties, limit/sort/cardinality, nested relations, constraints/metadata parity, all approved DML/DDL branches and transaction effects, and composite pipelines mixing several families.

For variants whose correctness depends on vendor-specific laws, pass declared canonical assumptions and compare meaning-preserving dialect renderings to executable DuckDB SQL. **DuckDB execution is not native vendor-engine certification.** The 13-dialect parser+semantic tests must run without external databases, and the canonical protocol must be equal for equivalent meaning, including bounds, lineage, cardinality, positive/negative witness obligations and write effects. Do not use `sqlparser` or any SQL reinterpretation in sql-tdg.

## Future features and residuals

Opaque/UDF functions, nonterminating recursion, unseeded stochastic functions and unavailable vendor runtime laws are **deferred pending verifiable typed evidence**, not forbidden forever. Future versions may read structured declarations, database catalog queries, dbt-defined function/macro metadata or ODCS-like contracts, with adapter provenance and canonical semantics owned by the protocol. Until modeled exactly, supported-claim gates must fail closed and should carry a typed unsupported/residual reason. A safely modeled subset is still subject to full m-3 acceptance if advertised.

## Release sequencing

1. Implement and merge protocol TASK-66..90 while keeping Release Please PR #79 unmerged.
2. Pin sql-tdg TASK-43 to an **unpublished v3 protocol Git commit SHA**, resolve Cargo lock and implement TASK-24..31, 35, 36 and 44.
3. Run the same unified committed `make all` and all Rust/dialect/cross-feature checks in CI, persist counterexample snapshots on failure, verify no missed approved variant.
4. Maintainer signs off protocol TASK-91 only after all release-blocking evidence is green; then merge Release Please PR #79 and publish protocol 3.0.0 once.
5. Switch sql-tdg to the published crate and verify identical semantics and fixtures.

Approving the 2026-10-09 design decisions **does not waive any of these checks**.
