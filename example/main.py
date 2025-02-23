from datetime import datetime
from sql_tdg.tdg.data import Data
from sql_tdg.tdg.types import (
    Col,
    Schema,
)


# Define a small dataset size (e.g., 10 rows)
schema = Schema(
    cols={Col("a", int), Col("b", str), Col("c", datetime), Col("d", bool)},
)

d = Data(schema)
d.generate()
print(d.getData())
