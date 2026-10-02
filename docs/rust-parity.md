# Rust parity matrix

The Go implementation remains the reference until TASK-8. Rust tests are ported with the migration
slice that owns the behavior so both suites remain green throughout the rewrite.

## Parity contract

**Exact behavioral parity** applies to parser IR, solver domain narrowing, constraint errors,
timestamp conversions, table behavior, and deterministic full-query behavior that does not depend
on random sampling.

**Seeded-generator parity** preserves semantics rather than Go's exact `math/rand` sequence. For a
fixed Rust seed, repeated Rust runs must produce the same ordered output, and every generated value
must satisfy its constraints. Rust is not required to reproduce the exact random samples currently
asserted by Go generator tests.

## Existing Go test catalog

| Go test | Planned Rust counterpart | Owner task | Contract |
| --- | --- | --- | --- |
| `internals/interop/interop_test.go::TestInterop_FullQueryGeneratorInts` | `tests/full_query.rs::full_query_generator_ints` | TASK-8 | exact semantics |
| `internals/interop/interop_test.go::TestInterop_FullQueryGeneratorBool` | `tests/full_query.rs::full_query_generator_bool` | TASK-8 | exact semantics |
| `internals/interop/interop_test.go::TestInterop_FullQueryGeneratorTimestamp` | `tests/full_query.rs::full_query_generator_timestamp` | TASK-8 | exact semantics |
| `internals/parser/parser_test.go::TestParse_QuaryParsing` | `src/parser/tests.rs::query_parsing` | TASK-4 | exact |
| `internals/solver/bools_test.go::TestBool_Single_Apply` | `src/solver/bools.rs::tests::single_apply` | TASK-3 | exact |
| `internals/solver/bools_test.go::TestBool_Multi_Apply` | `src/solver/bools.rs::tests::multi_apply` | TASK-3 | exact |
| `internals/solver/generator_test.go::TestIntGenerator_Generate` | `src/generator.rs::tests::generate_ints` | TASK-7 | seeded-generator |
| `internals/solver/generator_test.go::TestTimestampGenerator_Generate` | `src/generator.rs::tests::generate_timestamps` | TASK-7 | seeded-generator |
| `internals/solver/generator_test.go::TestBoolGenerator_Generate` | `src/generator.rs::tests::generate_bools` | TASK-7 | seeded-generator |
| `internals/solver/integers_test.go::TestInt_Single_Apply` | `src/solver/integers.rs::tests::single_apply` | TASK-3 | exact |
| `internals/solver/integers_test.go::TestInt_Multi_Apply` | `src/solver/integers.rs::tests::multi_apply` | TASK-3 | exact |
| `internals/solver/timestamp_test.go::TestTimestamp_Single_Apply` | `src/solver/timestamp.rs::tests::single_apply` | TASK-3 | exact |
| `internals/solver/timestamp_test.go::TestTimestamp_Multi_Apply` | `src/solver/timestamp.rs::tests::multi_apply` | TASK-3 | exact |
| `internals/solver/timestamp_test.go::TestToDate` | `src/solver/timestamp.rs::tests::to_date` | TASK-3 | exact |
| `internals/solver/timestamp_test.go::TestToTimestamp` | `src/solver/timestamp.rs::tests::to_timestamp` | TASK-3 | exact |
| `internals/solver/timestamp_test.go::TestFromInt` | `src/solver/timestamp.rs::tests::from_int` | TASK-3 | exact |
| `internals/table/table_test.go::TestTable_Append` | `src/table/tests.rs::append` | TASK-5 | exact |
| `internals/table/table_test.go::TestTable_AppendRejectsWrongType` | `src/table/tests.rs::append_rejects_wrong_type` | TASK-5 | exact |
| `internals/table/table_test.go::TestTable_Wipe` | `src/table/tests.rs::wipe` | TASK-5 | exact |
| `internals/table/table_test.go::TestTable_SortInts` | `src/table/tests.rs::sort_ints` | TASK-5 | exact |
| `internals/table/table_test.go::TestTable_AllColumnTypesUseArrowStorage` | `src/table/tests.rs::all_column_types_use_arrow_storage` | TASK-5 | exact |


The constraint-mapping portion exercised by the three interop full-query tests is covered in
TASK-6 by `src/interop/tests.rs`. Their generator-dependent end-to-end behavior remains assigned
to TASK-8.

The bootstrap smoke test in `tests/bootstrap.rs` is infrastructure-only and is not a parity case.
