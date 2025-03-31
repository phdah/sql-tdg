# sql-tdg

> SQL Query Test Data Generator

Go from a SQL query, to test data that fulfills the conditions of the query. Take this
query:

```sql
select
    a,
    b
from table
    where a > 10
```

For this query to return any data, the column `a`, need to have values `> 10`. The `TDG`
class can help you do this.

By providing only a `query` and the `schema`, data matching the queries condition will be
produced.

## Quick start

Follow the guide found in the
[example](https://github.com/phdah/sql-tdg/tree/main/python_poc/examples) directory.

# Readmap

[] [gRPC](https://grpc.io/) for interop with other languages.
