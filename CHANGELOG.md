# Changelog

## [1.0.0](https://github.com/phdah/sql-tdg/compare/v0.1.0...v1.0.0) (2026-10-08)


### ⚠ BREAKING CHANGES

* **cli:** the CLI now generates 10 rejected rows by default. Generating for all terminal outcomes, dbt sources with relationship constraints, or domains without a safe complement requires --rejected 0.
* **cli:** ProtocolGenerationError gains ConflictingOutcomes and TerminalOutcome variants and TargetKind gains AllTerminalOutcomes.

### Features

* **cli:** add backend-neutral generation workflow ([#23](https://github.com/phdah/sql-tdg/issues/23)) ([70aa682](https://github.com/phdah/sql-tdg/commit/70aa68220f08886acb2299c595bed82c4a1c3807))
* **cli:** dbt whole-project generation and safety fixes ([#27](https://github.com/phdah/sql-tdg/issues/27)) ([d9a0307](https://github.com/phdah/sql-tdg/commit/d9a03075206e914ee4ab3e69840e1a3e8c82d131))
* **cli:** present grouped, terminal-aware help ([#35](https://github.com/phdah/sql-tdg/issues/35)) ([2690fbc](https://github.com/phdah/sql-tdg/commit/2690fbcab42be8556d347d756eb43ba962f01c51))
* **dbt:** generate from declared schemas without catalog ([#29](https://github.com/phdah/sql-tdg/issues/29)) ([4b26c21](https://github.com/phdah/sql-tdg/commit/4b26c21846f6633cefbe361b5451798e48a4abe7))
* **export:** add backend-neutral outputs and test-only DuckDB harness ([#22](https://github.com/phdah/sql-tdg/issues/22)) ([807bd88](https://github.com/phdah/sql-tdg/commit/807bd886f6df638018417025c7b7993feb27e1d7))
* **generator:** expand lossless protocol type coverage ([#18](https://github.com/phdah/sql-tdg/issues/18)) ([63ca3b6](https://github.com/phdah/sql-tdg/commit/63ca3b675539c28d3d47f38500700e5726bf9a93))
* **generator:** port deterministic Rust generator ([#9](https://github.com/phdah/sql-tdg/issues/9)) ([ee1c52e](https://github.com/phdah/sql-tdg/commit/ee1c52ef0bfe5fb2cf2551117152c68039c83243))
* **interop:** port parser-solver interop ([#8](https://github.com/phdah/sql-tdg/issues/8)) ([fec25ca](https://github.com/phdah/sql-tdg/commit/fec25ca0eb4677515f5a8c519286294eb2c69b53))
* **parser:** port SQL parser and IR to Rust ([#6](https://github.com/phdah/sql-tdg/issues/6)) ([6e434af](https://github.com/phdah/sql-tdg/commit/6e434af25a9ac72d34d992e3935c6956da42110f))
* **protocol:** consume SQL Semantic Protocol ([#13](https://github.com/phdah/sql-tdg/issues/13)) ([066cc1e](https://github.com/phdah/sql-tdg/commit/066cc1ea02ac61cde4d7623b08cbb59d7f22cb3f))
* **protocol:** cover reachable CASE branches with source witnesses ([#31](https://github.com/phdah/sql-tdg/issues/31)) ([8969f53](https://github.com/phdah/sql-tdg/commit/8969f530c16230a13579ed0261347606fbb880b6))
* **protocol:** distribute moderate join and foreign keys ([#33](https://github.com/phdah/sql-tdg/issues/33)) ([5afaf74](https://github.com/phdah/sql-tdg/commit/5afaf74fb9aefd8eb0962b4c7952fcdd42df5a08))
* **protocol:** enforce SQL Semantic Protocol 2.0 exactness contract ([#28](https://github.com/phdah/sql-tdg/issues/28)) ([7f50dd8](https://github.com/phdah/sql-tdg/commit/7f50dd8fd51cdacbe2f902c28b8b28761de46036))
* **protocol:** generate at intermediate boundaries ([#21](https://github.com/phdah/sql-tdg/issues/21)) ([12d196e](https://github.com/phdah/sql-tdg/commit/12d196e01ca1a1730a69a058ca7ffc8e87690e9d))
* **protocol:** generate rejected scalar witnesses ([#19](https://github.com/phdah/sql-tdg/issues/19)) ([005188a](https://github.com/phdah/sql-tdg/commit/005188a651e68926327540a33a17071e61c7809c))
* **protocol:** generate relational witnesses ([#20](https://github.com/phdah/sql-tdg/issues/20)) ([74bd147](https://github.com/phdah/sql-tdg/commit/74bd147ebab90b68ccf7413a65087246c603b87b))
* **protocol:** generate through CTEs and derived tables (TASK-21.5) ([#30](https://github.com/phdah/sql-tdg/issues/30)) ([d724120](https://github.com/phdah/sql-tdg/commit/d724120f11b7218239d6eee785c9e5aa803c99f5))
* **protocol:** honor canonical dbt relation constraints ([#32](https://github.com/phdah/sql-tdg/issues/32)) ([7c4bc98](https://github.com/phdah/sql-tdg/commit/7c4bc98d2d905448be10532492d63113a2fb7bec))
* **solver:** port Rust solver domains ([#5](https://github.com/phdah/sql-tdg/issues/5)) ([245dfac](https://github.com/phdah/sql-tdg/commit/245dfac0d089386f3c60a8fab425bc86ebbffc7c))
* **table:** port Arrow table storage to Rust ([#7](https://github.com/phdah/sql-tdg/issues/7)) ([fc6af8c](https://github.com/phdah/sql-tdg/commit/fc6af8c418b8171cb932052f4f33e97616239424))
* **test-case:** define executable test case contract ([#17](https://github.com/phdah/sql-tdg/issues/17)) ([4bd0e8a](https://github.com/phdah/sql-tdg/commit/4bd0e8a27fbdbe01e1591ff993903162526df197))
* **types:** port shared Rust types and invariants ([#4](https://github.com/phdah/sql-tdg/issues/4)) ([f784476](https://github.com/phdah/sql-tdg/commit/f784476ebbdee2ec5aef62f20f29b9c37918bc59))

## Changelog

Release Please maintains this file from Conventional Commits.
