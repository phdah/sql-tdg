# sql-tdg

> SQL Query Test Data Generator

sql-tdg is a Rust library that generates Arrow-backed test data satisfying the supported
conditions of a SQL query.

Given this query:

```sql
select
    a,
    b
from table
where a > 10
```

and a schema containing an integer column `a`, sql-tdg narrows the column domain so generated
values satisfy `a > 10`.

## Quick start

The project currently exposes a library API and has no CLI entry point.

```rust
use sql_tdg::{Column, ColumnType, Generator, Table, apply_conditions, parse_query};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = parse_query("SELECT a FROM table WHERE a > 10")?;
    let mut table = Table::new(vec![Column::new("a", ColumnType::Int)], 10)?;

    apply_conditions(&query, &mut table)?;
    Generator::new().generate(&mut table, 42)?;
    table.build_ints();

    let values = table.get_ints("a")?.expect("integer column should be built");
    assert!(values.iter().all(|value| *value > 10));

    Ok(())
}
```

Supported generation domains currently include integers, booleans, and timestamps. Unsupported
SQL constructs, operators, or generation types return explicit errors rather than silently
producing data that may violate the query.

The original Python proof of concept remains under `python_poc/` as frozen reference material and
is not part of the active implementation.

## Verification

Run the complete Rust verification suite with:

```console
make rust-checks
```
