# sql-tdg Agent Instructions

See [README.md](README.md) for the project overview: given a SQL query and a table schema,
generate test data that satisfies the query's supported conditions. The Rust crate at the
repository root is the sole active implementation. `python_poc/` is frozen unless the user asks
otherwise. This file holds project principles and Rust-specific conventions.

## Architecture

The library flows in one direction through modules under `src/`:

- `parser`: parses SQL with `sqlparser` and lowers third-party AST values into project-owned IR.
- `interop`: maps parser IR onto typed constraints attached to table columns.
- `solver`: narrows integer, boolean, and timestamp domains and exposes deterministic sampling
  positions.
- `generator`: samples solved domains from an injected deterministic random stream and appends
  values to table storage.
- `table`: owns Arrow-backed typed columnar storage.
- `types`: owns shared domain types, constraints, identifiers, and invariants.

Parser-library AST types stay inside the parser boundary. Downstream modules consume only
project-owned types. Keep dependencies acyclic and do not reach across stages to bypass the
intended boundary.

There is currently no CLI entry point. If a CLI is introduced, it must sit on top of the library
pipeline and contain no parsing, solving, generation, or storage logic.

## Guiding principles

**Never over-claim** Generated data is useful only when every row is guaranteed to satisfy the
supported query semantics. When a condition, operator, type, or SQL construct is unsupported,
return an explicit error naming it. Never silently drop a condition.

**Separation of concerns** Keep parsing, constraint mapping, solving, generation, and storage
separate. A parser change should not leak third-party AST types into solver or table code.

**Library first, thin CLI** Core behavior belongs in importable library modules with small public
APIs. Any application entry point should only translate I/O into library calls.

**Strict types** Use enums for closed sets such as column types, comparison operators, and join
kinds. Use newtypes when identifiers must not be mixed. Prefer structs with named fields over
positional tuples for domain data. Match project-owned enums exhaustively. A catch-all over a
large third-party enum is acceptable only when it returns an explicit unsupported error.

**Make invalid states unrepresentable** Construct values through constructors that enforce
invariants, keep invariant-bearing fields private where useful, and prefer parsing directly into a
valid domain value over constructing an invalid value and validating it later.

**Error handling** Library code returns `Result` with domain-specific error variants and uses `?`
for propagation. Do not use `unwrap()`, `panic!()`, panic-prone indexing, or unchecked
conversions for bad input. `expect("...")` is acceptable only for a true internal invariant whose
reason is stated. Generic boxed errors belong at application boundaries, not in core APIs.

**Deterministic output** The same query, schema, row count, and seed must produce identical data.
Never let unordered iteration affect observable output. Randomness is injected and owned by the
generator, not read from a global source.

**Small, pure functions** Parsing, constraint, and domain functions take inputs and return values
without I/O, global mutable state, or environment reads. If behavior is hard to test, simplify the
design.

**Traits and generics are earned** Prefer plain functions and concrete structs. Introduce a trait
only for a real contract with multiple implementations or a clear consumer need. Do not add
abstraction for hypothetical flexibility.

**Current-state comments** Source comments and API documentation describe only the current design,
behavior, invariants, and rationale. Never explain code by referring to a previous implementation,
migration, legacy behavior, historical state, or what the code used to do. History belongs in
version control and task records.

**Minimize dependency footprint** Add crates only when they provide clear value. Enable only the
features that are used. Confirm with the user before adding a new dependency. `Cargo.toml` is the
authoritative source for the Rust edition, dependencies, and profiles.

**Rust ownership** Bindings are immutable by default. Borrow `&str`, `&[T]`, and `&T` when
ownership is unnecessary and return owned values when the caller must retain them. Clone
deliberately rather than merely to satisfy the borrow checker.

**Rust safety** Do not introduce `unsafe`.

**Readability over cleverness** Prefer direct control flow, descriptive domain names, and small
functions. Comments explain why, not what. Public items have documentation when their contract is
not obvious from the type signature.

Do not add `#[allow(...)]`, lint suppressions, or equivalent escapes without explicit user
confirmation. Fix the underlying issue instead.

## Versioning and commits

Follow semantic versioning. While pre-1.0, a breaking change to the exported API bumps the minor
version.

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org/):
`<type>(<optional scope>): <description>`, with the description in imperative mood and lowercase.
Use the module name as the scope when useful. Mark breaking changes with `!` and a
`BREAKING CHANGE:` footer.

## Verification

Repository verification is owned by the Makefile. Run:

```console
make rust-checks
```

That command must remain the single source of truth for pull-request CI and runs:

```console
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Run `cargo doc --no-deps` when public documentation changes.

## Test conventions

**Black-box integration tests** Put public API and end-to-end behavior under `tests/`.

**Focused unit tests** Put private behavior beside the implementation in
`#[cfg(test)] mod tests`.

**SQL as the fixture** Parser and interop tests state SQL inline and assert on the resulting IR,
constraints, or generated values.

**Cover unsupported paths** Every intentionally unsupported operator, type, or construct should
have a test asserting an explicit error.

**Deterministic tests** Seed random generation explicitly. Never depend on wall-clock time,
unordered iteration, or scheduling.

**Shared helpers over repetition** Extract repeated schema, query, and assertion setup rather than
copying it across tests.

**Benchmarks for hot paths** Performance claims for generation, solving, or table operations should
include a benchmark and before/after measurements.

## Task tracking

Todos, planned work, and decisions are tracked in the local
[Backlog.md](https://github.com/MrLesk/Backlog.md) board under `.backlog/`. Use Backlog.md tooling
when available; otherwise edit the Markdown files directly following the existing format. Commit
task changes alongside the work they describe.
