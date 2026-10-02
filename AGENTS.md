# sql-tdg Agent Instructions

See [README.md](README.md) for the project overview: given a SQL query and a table schema,
generate test data that satisfies the query's conditions (e.g. `where a > 10` yields rows
where `a > 10`). The Go module at the repo root is the active implementation;
`python_poc/` is the original proof of concept and is frozen unless the user asks
otherwise. A Rust rewrite is planned and tracked in `.backlog/`; until cutover, Go remains
the reference implementation and Rust is developed alongside it. Language-specific rules below
apply to files written in that language. This file holds principles only.

## Architecture

The pipeline flows in one direction, one package per stage under `internals/`:

- `parser`: participle grammar (`codex.go`) turns SQL text into an AST, then lowers it to
  intermediate representations (`ConditionsIR`, `JoinIR`).
- `interop`: the single boundary that maps parser IR onto solver constraints for a given
  column type.
- `solver`: per-type domains (`IntDomain`, `TimestampDomain`, `BoolDomain`) narrowed by
  constraints, plus the `Generator` that samples values from them.
- `table`: typed, columnar storage for the generated rows.
- `types`: shared domain types and interfaces (`Column`, `Domain`, `Constraints`).

There is currently no active CLI entry point on `main`. If a CLI is introduced, it must sit on
top of the library pipeline and contain no parsing, solving, or storage logic.

Dependencies point downward only: `parser` and `solver` never import `interop`, and
`types` imports nothing from the module. Never introduce import cycles or reach across
stages to work around this.

## Guiding Principles

**Never over-claim** Generated data is only useful if every row actually satisfies the
query. When a condition, operator, type, or SQL construct is not supported, return an
error naming it. Never silently drop a condition and never generate data you cannot
guarantee satisfies it. An error is always preferable to rows that do not match the query.

**Separation of concerns** Keep parsing, constraint mapping, solving, and storage apart.
The participle AST stays inside `parser`; the rest of the module sees only the IR types.
Conversion from IR to solver constraints happens only in `interop`, so a grammar change or
a new solver domain touches one boundary.

**Library first, thin CLI** Core logic lives in importable packages with small exported
APIs. The CLI is a thin `main` package under `cmd/sql-tdg/` that reads the query and
schema, calls the library, and writes the generated data. It holds no parsing or solving
logic, so the library stays testable without spawning a process.

**Strict types** Model the domain with types, not strings. Use named types with `const`
blocks for fixed sets of values (column types, comparison operators, join kinds), and
define distinct named types for identifiers that must not be mixed up. Prefer concrete
types over `any`; when `any` crosses an interface (e.g. `Domain.RandomValue`), check the
type assertion with the two-value form and return an error on mismatch. `switch`
statements over domain enums handle every case explicitly, and the `default` branch
returns an error rather than doing nothing.

**Make invalid states unrepresentable** Construct values through `NewX` constructors that
enforce invariants (e.g. an interval's `Min <= Max`), and keep fields unexported when an
invariant must hold. Make the zero value either useful or impossible to misuse.

**Error handling** Library code returns `error` as the last return value and never
panics on bad input; `panic` is reserved for true programmer errors that cannot be
expressed otherwise. Wrap errors with context using `fmt.Errorf("doing x: %w", err)`, keep
messages lowercase without trailing punctuation, and define sentinel errors (`var
ErrUnsupportedOp = errors.New(...)`) or typed errors when callers need to branch on them
with `errors.Is` / `errors.As`. Never discard an error with `_` without a comment
explaining why. Goroutines must report errors back to the caller (e.g. via
`errgroup`-style collection or a channel), not panic.

**Deterministic output** The same query, schema, and seed must produce identical data.
Never let map iteration order leak into results; sort keys explicitly. A `*rand.Rand` is
not safe for concurrent use, so give each goroutine its own source derived from the seed,
and write results to pre-assigned positions rather than appending in completion order.

**Concurrency** Only add goroutines when there is a measured benefit. Every goroutine has a
clear owner and a guaranteed exit. Protect shared state with a mutex or avoid sharing it;
prefer passing data over channels or giving each worker its own slice. All tests must pass
under `-race`.

**Small, pure functions** Parsing, constraint, and domain functions take inputs and return
values, with no I/O, global mutable state, or environment reads. Randomness is injected as
a `*rand.Rand` parameter, never taken from the global source. If something is hard to
test, the design is wrong.

**Interfaces are small and consumer-defined** Define an interface where it is used, keep
it to the methods actually needed, and introduce one only when there are, or will
concretely be, multiple implementations. Accept interfaces, return concrete types. Avoid
generics that exist only for hypothetical flexibility.

**Readability over cleverness** Follow [Effective Go](https://go.dev/doc/effective_go) and
the [Go Code Review Comments](https://go.dev/wiki/CodeReviewComments). Use short, clear
names; no stutter (`table.Table` is fine, `table.TableSchema` is not). Every exported
identifier has a doc comment starting with its name. Each package has a `// Package x ...`
comment describing its responsibility. Use comments to explain *why*, not *what*.

**Current-state comments** Source comments and API documentation describe only the current
design, behavior, invariants, and rationale. Never explain code by referring to a previous
implementation, migration, rewrite, legacy behavior, historical state, or what the code used to
do. History belongs in version control and task records, not in source comments. Write every
comment so it remains correct if all prior implementations and migration context disappear.

**Minimize dependency footprint** The standard library, participle, and testify should
cover most needs. Confirm with the user before adding a module. Keep `go.mod` tidy with
`go mod tidy`; direct dependencies must not be marked `// indirect`.

**Go tooling** For Go code, use the `go` toolchain. Read `go.mod` before writing Go code;
it is the authoritative source for the Go version and dependencies, so only use language and
standard library features available in that version. Go code is formatted with `gofmt` and
must pass `go vet` with no findings.

Don't add `//nolint`, `#[allow(...)]`, or other lint suppressions without confirmation from
the user. Fix the underlying issue, or consult the user for explicit guidance.

## Rust migration rules

These rules apply to Rust code added during the migration. They supplement the existing
language-independent architecture and correctness principles; they do not weaken the Go rules
while Go remains the reference implementation.

**Rust module boundaries** Preserve the same one-way architecture in Rust. Parser-library AST
types are an input format and stay inside the parser boundary. Lower them into project-owned IR
types before interop or solver code sees them. `src/lib.rs` is the module index and public
library surface; keep its module docs and re-exports synchronized with public API changes.

**Rust strict types** Use enums for closed sets such as column types, comparison operators, and
join kinds; use newtypes when identifiers must not be mixed. Prefer structs with named fields
over positional tuples for domain data. Match project-owned enums exhaustively and avoid
catch-all arms that could hide a newly added variant. A catch-all over a large third-party parser
enum is acceptable only when it produces an explicit unsupported error.

**Rust invariants** Make invalid states unrepresentable. Construct values through constructors
that enforce invariants, keep invariant-bearing fields private where useful, and prefer parsing
into a valid domain value over constructing an invalid value and validating it later.

**Rust ownership** Bindings are immutable by default. Borrow `&str`, `&[T]`, and `&T` in
arguments when ownership is unnecessary and return owned values when the caller must retain them.
Clone deliberately, not merely to satisfy the borrow checker; if cloning feels structural,
revisit the design.

**Rust errors** Library code returns `Result` with domain-specific error variants. Use `?` for
propagation. Do not use `unwrap()`, `expect()`, `panic!()`, or panic-prone indexing in
library code for bad input. An `expect("...")` is acceptable only for a true internal invariant
whose reason is stated. Generic boxed errors belong at an application boundary, not in core
library APIs.

**Rust traits and generics** Prefer plain functions and concrete structs. Introduce a trait only
for a real contract with multiple concrete implementations or a clear consumer need. Do not add
generic or trait-object abstraction only for hypothetical flexibility.

**Rust safety** Do not introduce `unsafe`. The project does not require it.

**Rust dependencies** Minimize crate dependencies and enable only features that are used. Confirm
with the user before adding a new crate. Read `Cargo.toml` before writing Rust code; it is the
authoritative source for the Rust edition, dependencies, and profiles.

**Rust tooling** Use Cargo for Rust work. Rust code uses rustfmt defaults and must pass Clippy
with no warnings. Once the Rust crate exists, pull-request CI must run formatting, Clippy, and
tests for every Rust change.

**Rust tests** Put focused unit tests in `#[cfg(test)] mod tests` beside the implementation when
private behavior needs coverage. Put end-to-end and public API parity tests under `tests/`.
State SQL fixtures inline when practical, name tests for behavior, cover unsupported paths, and
share helpers rather than duplicating setup.

**Semantic versioning** Follow semver. While pre-1.0, a breaking change to the exported
API bumps the minor version.

**Conventional commits** Commit messages follow
[Conventional Commits](https://www.conventionalcommits.org/):
`<type>(<optional scope>): <description>`, with the description in imperative mood and
lowercase, e.g. `feat(solver): support string equality`. Use the package name as the
scope. Common types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `build`, `ci`,
`perf`. Mark breaking changes with `!` after the type/scope and explain them in a
`BREAKING CHANGE:` footer.

## Verification

Run checks proportional to the change. While Go remains active, Go changes must pass:

```console
gofmt -l .        # must print nothing
go vet ./...
go test -race ./...
go mod tidy       # must leave go.mod and go.sum unchanged
```

Once `Cargo.toml` exists, Rust changes must also pass:

```console
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Run `cargo doc --no-deps` when Rust public documentation changes.

`make tests` currently runs the Go unit tests verbosely and is what the pre-push hook
(`.gitHooks/pre-push`) uses. TASK-1 of the Rust rewrite must extend repository tooling and CI so
the Rust checks above are mandatory as soon as the Rust crate exists. Use
`go test -run TestName ./internals/<pkg>` to iterate on a single Go test.

## Test conventions

**Black-box by default** Tests live in `x_test.go` next to the code, in the external
`package x_test`, and exercise only the exported API, exactly as a consumer would. Use an
internal `package x` test file only when a private function genuinely needs direct
coverage.

**Table-driven tests** Group cases for the same function in a `[]struct{ name string; ... }`
table and run each with `t.Run(tt.name, ...)`. Name cases for the behavior, e.g.
`"not equal splits interval"`, and name test functions `TestType_Method` or
`TestFunction_Behavior`.

**Use testify consistently** Use `require` for preconditions and anything after which the
test cannot continue, and `assert` only when later checks are still meaningful. Compare
errors with `require.ErrorIs` / `require.ErrorAs` rather than matching message strings.

**SQL as the fixture** Parser and interop tests state their input SQL inline and assert on
the resulting IR or constraints. End-to-end tests go from SQL text plus schema to a
generated table and assert that every generated row satisfies the query's conditions.

**Cover the unsupported path** For every operator, type, or construct that is not handled,
add a test asserting it returns an error rather than being silently ignored.

**Deterministic tests** Seed every random source explicitly. Never depend on wall-clock
time, map order, or goroutine scheduling. Use `t.Helper()` in assertion helpers and
`t.TempDir()` for any filesystem needs.

**Shared helpers over repetition** When several tests build the same schema, query, or
expected domain, extract a helper in the test file instead of copy-pasting setup.

**Benchmarks for hot paths** Changes to the generator or domains that claim a performance
improvement come with a `BenchmarkX` function and before/after `go test -bench` numbers.

## Task tracking

Todos, planned work, and decisions for this project are tracked in a local
[Backlog.md](https://github.com/MrLesk/Backlog.md) board stored in `.backlog/`. Use the
`backlog_*` MCP tools or the `backlog` CLI when available; otherwise edit the Markdown
files under `.backlog/` directly, following the format of existing files. The board is
versioned with the repo, so commit task changes alongside the work they describe.
