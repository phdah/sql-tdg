# sql-tdg Agent Instructions

See [README.md](README.md) for the project overview: given a SQL query and a table schema,
generate test data that satisfies the query's conditions (e.g. `where a > 10` yields rows
where `a > 10`). The Go module at the repo root is the active implementation;
`python_poc/` is the original proof of concept and is frozen unless the user asks
otherwise. This file holds principles only.

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

The CLI entry point lives in `cmd/sql-tdg/` and sits on top of the pipeline; nothing
under `internals/` imports it.

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

**Minimize dependency footprint** The standard library, participle, and testify should
cover most needs. Confirm with the user before adding a module. Keep `go.mod` tidy with
`go mod tidy`; direct dependencies must not be marked `// indirect`.

**Tooling** Use the `go` toolchain for everything. Read `go.mod` before writing code; it is
the authoritative source for the Go version and dependencies, so only use language and
standard library features available in that version. Code is formatted with `gofmt` and
must pass `go vet` with no findings.

Don't add `//nolint` directives or other lint suppressions without confirmation from the
user. Fix the underlying issue, or consult the user for explicit guidance.

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

Run checks proportional to the change. Before considering a change done:

```console
gofmt -l .        # must print nothing
go vet ./...
go test -race ./...
go mod tidy       # must leave go.mod and go.sum unchanged
```

`make tests` runs the unit tests verbosely and is what the pre-push hook
(`.gitHooks/pre-push`) uses. Use `go test -run TestName ./internals/<pkg>` to iterate on a
single test.

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
