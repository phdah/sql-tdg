# Rust rewrite plan

## Decision

Rewrite the active Go implementation to Rust incrementally in the same repository.

A big-bang rewrite is technically possible because the active implementation is small, but
it provides little benefit. Keeping Go and Rust side by side during the migration gives us a
working reference implementation while each Rust component reaches behavioral parity.

The migration should be test-driven per component:

1. port the relevant Go tests to Rust,
2. implement the Rust component until those tests pass,
3. keep both Go and Rust suites green,
4. move to the next dependency layer.

Do not port the entire Rust test suite first and leave it failing while implementation catches
up. That removes the value of a continuously green branch and makes regressions harder to
localize.

## Current scope

The active Go implementation on `main` consists of:

- 11 implementation files, about 1,340 lines,
- 7 test files, about 1,391 lines,
- 21 top-level Go test functions,
- parser, IR, interop, solver, generator, shared types, and Arrow-backed table storage.

The Arrow-backed `table` package is the largest single component at about 462 lines, roughly
one third of the active implementation.

The Python proof of concept remains frozen and is not part of the rewrite.

`AGENTS.md` currently describes a `cmd/sql-tdg/` CLI, but no such entry point exists on
`main`. The rewrite therefore targets the observed library implementation. A CLI should not
be invented as part of parity work unless one is added separately.

## Size assessment

This is a small-to-medium rewrite, not a ground-up redesign.

The amount of business logic is modest. Most migration risk is concentrated in four areas:

- SQL parser behavior and lowering into the existing IR,
- Arrow array and builder semantics,
- deterministic random generation,
- integration boundaries between parser, solver, table, and generator.

A Rust implementation will likely be similar in size or somewhat larger than the Go
implementation because explicit enums and error types replace several uses of interfaces and
`any`. The total code volume is still small enough that a big-bang rewrite could be completed,
but iteration gives materially better verification with almost no architectural cost.

No Go/Rust FFI layer is needed. Both implementations can coexist as independent build targets
until the final cutover.

## Target Rust structure

Keep the current dependency direction:

```text
parser ──> IR
            │
            v
         interop ──> solver
            │
            v
          table <── generator
```

A practical Rust layout is:

```text
Cargo.toml
src/
  lib.rs
  types.rs
  parser/
  solver/
  table/
  interop.rs
  generator.rs
```

The Rust implementation should prefer typed enums and concrete values over a direct translation
of Go interfaces and `any`. Unsupported SQL, operators, and column types must still fail
explicitly rather than being silently ignored.

Likely dependency categories are:

- a Rust SQL parser that can be lowered into the existing project IR,
- Apache Arrow Rust crates,
- a deterministic RNG abstraction,
- date/time parsing support.

Exact crates and versions should be selected in the bootstrap task rather than fixed by this
planning change.

## Parity contract

Parity means preserving externally meaningful behavior, not reproducing every Go implementation
detail.

Exact parity is appropriate for:

- parsed IR for currently supported SQL,
- join kinds and condition lowering,
- domain narrowing,
- constraint mapping,
- table append/get/build/wipe/sort behavior,
- supported and unsupported error cases.

Generated values need a more careful contract. The current Go tests assert exact pseudo-random
samples from `math/rand`, while the generator shares one RNG across goroutines. A normal Rust
RNG will not reproduce those samples, and preserving scheduler-dependent behavior would be a
mistake.

The Rust migration should therefore guarantee:

- the same seed produces the same Rust output across runs,
- every generated value satisfies its constraints,
- output ordering is deterministic,
- no shared mutable RNG is used unsafely.

If byte-for-byte compatibility with Go's random sequence is later required, implement an
explicit Go-compatible RNG as a separate compatibility requirement.

The rewrite should also avoid carrying forward behavior that conflicts with the repository's
own principles. In particular, invalid input should return errors rather than preserving panic
paths or data races merely for parity.

## Migration order

### 1. Bootstrap Rust and the parity harness

Add a Rust crate beside the Go module, make both test suites runnable from the repository
tooling, and create a parity matrix mapping every existing Go test to its Rust counterpart.
Keep the Rust suite green from the first commit.

The first task must also add mandatory pull-request CI for Rust immediately, before substantive
porting begins. The workflow must run:

```console
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Existing Go CI remains active during the migration, so both implementations are continuously
verified until cutover.

### 2. Port shared types and invariants

Port column types, columns, intervals, constraint representations, and small utilities. Use Rust
types to eliminate invalid states rather than mechanically reproducing Go's dynamic interfaces.

### 3. Port solver domains

Port integer, boolean, and timestamp domains and their constraints. Port the solver tests before
each implementation. Preserve domain semantics and explicit contradiction errors.

### 4. Port parser and IR

Port the supported SQL grammar and lower it into equivalent Rust IR types. Parser acceptance must
not be confused with generator support: syntax that the chosen parser understands but the project
cannot safely lower must return an explicit unsupported error.

### 5. Port Arrow table storage

Port the table only after the primitive types are stable. Preserve Arrow-backed storage for int,
timestamp, bool, and string columns, including append validation, finalization, getters, wipe,
and sorting behavior.

The table is deliberately late because it is the largest component, but it is not the final glue.
It does not depend on parser or solver behavior by itself.

### 6. Port interop

Port conversion from parser IR to typed solver constraints using the Rust table schema. This is
the first major cross-component integration slice.

### 7. Port generator

Connect solver domains to Arrow-backed table generation. Make seeded generation deterministic and
race-free. Return errors rather than panicking on invalid row counts or unsupported column types.

### 8. End-to-end parity and cutover

Port the full-query tests, run Go and Rust parity together, then make Rust the only active
implementation. Remove the Go module only after the Rust suite covers the required behavior.

At cutover:

- remove the Go-only CI path while keeping the Rust CI introduced in the bootstrap task,
- update the Makefile and pre-push hook so Rust is the default active implementation,
- simplify `AGENTS.md` from dual-language migration guidance to Rust-only conventions,
- update the README,
- remove `go.mod`, `go.sum`, and active Go sources,
- leave `python_poc/` frozen.

## Why not table last?

The table should be late, but not literally last. Both `interop` and `generator` depend on it,
and those two modules are the actual integration layers tying the system together. Finishing the
table first lets the final migration steps exercise real end-to-end behavior rather than mocks.

## Completion criterion

The rewrite is complete when the Rust implementation is the sole active implementation, all
ported parity tests pass, unsupported paths remain explicit, seeded generation is deterministic,
and the repository tooling/documentation no longer assumes Go.
