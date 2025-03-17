from datetime import datetime

import duckdb
from sql_tdg.tdg.data import TDG
from sql_tdg.tdp.parser import Parser
from sql_tdg.tdg.types import (
    ColType,
    Schema,
)


table_name = "tdg_table"
query = f"""
    select
        a, b, c, d
    from {table_name}
         where a > 100
            and a <= 900
            and (a >= 110 or a < 1000) -- Try Paren type
            and b = 'hello' -- Ensures not distinct
            and b != 'no hello' -- Try not equal to
"""


schema = Schema(
    cols=[
        ColType("a", int),
        ColType("b", str),
        ColType("c", datetime),
        ColType("d", bool),
    ],
)

if __name__ == "__main__":
    # Parse query
    p = Parser(query)
    p.parseQuery()

    # Construct Test Data Generator object (TDG)
    data = TDG(schema=schema, conditions=p.conditions)

    # Setup a duckdb connection
    conn = duckdb.connect()
    # Create data, and insert into duckdb table
    data.getData().to_duckdb(conn, "tdg_table")

    # Insert "bad" data
    conn.sql("insert into tdg_table values (1, 'mark', '1970-01-01 01:00:19', False)")
    conn.sql(
        "insert into tdg_table values (120, 'no hello', '1970-01-01 01:00:20', False)"
    )

    # Show all data, and filtered data
    conn.table(table_name).show()
    conn.sql(query).show()
