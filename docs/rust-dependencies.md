# Rust dependency decisions

TASK-1 deliberately introduces no third-party Rust dependency. The bootstrap crate only needs the
standard library, and adding unused crates before their migration slice would increase compile
time and supply-chain surface without exercising them.

The following crates are the selected starting points for later migration tasks. They should only
be added to `Cargo.toml` when the task that needs them is implemented, and the version/features
should be chosen narrowly at that point.

| Need | Selected crate | First task | Rationale |
| --- | --- | --- | --- |
| SQL parsing | `sqlparser` | TASK-4 | General-purpose SQL parser with an AST that can stay behind the parser boundary and be lowered into project-owned IR. |
| Arrow storage | `arrow-array`, `arrow-schema`, `arrow-ord` | TASK-5 | Official modular Arrow crates cover arrays/schema/sorting without pulling the full umbrella crate by default. |
| Deterministic RNG | `rand_chacha`, `rand_core` | TASK-7 | A seedable, explicit algorithm makes Rust output reproducible without copying Go `math/rand` samples. |
| Date/time parsing | `chrono` | TASK-3 | Covers the date and RFC3339 timestamp behavior already exposed by the Go solver. |

No error-handling helper crate is selected. Domain errors should initially use project-owned enums
and standard-library `Error` implementations; a helper crate is only justified if the concrete
error model later demonstrates a need for one.
