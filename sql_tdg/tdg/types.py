from typing import Dict, List, Type, Any
import sql_tdg.tdg.z3 as z3
from datetime import datetime
import pandas as pd
from duckdb import DuckDBPyConnection


# Schema
class ColType:
    """
    Represents a column in a dataset.

    Attributes:
        name (str): The name of the column.
        type (Type[int | str | datetime | bool]): The data type of the column. Can be int, str, datetime, or bool.
            Note that this is a Type[H], meaning it's a type hint for a union type
            (i.e., one of several possible types).
    """

    def __init__(self, name: str, type: Type[int | str | datetime | bool]) -> None:
        self.name = name
        self.type = type

    def __repr__(self) -> str:
        return f"Col(name='{self.name}', type={self.type.__name__})"

    def __str__(self) -> str:
        return f"{self.name} ({self.type.__name__})"

    def __hash__(self) -> int:
        return hash((self.name, self.type))

    def __eq__(self, other: object) -> bool:
        return isinstance(other, ColType) and (
            self.name == other.name and self.type == other.type
        )


class Schema:
    """
    Represents a schema for a dataset.

    A schema consists of one or more columns, each with its own name and data type.
    This class serves as a container for these columns.

    Attributes:
        cols (List[Col]): The set of columns that comprise this schema.
    """

    def __init__(self, cols: List[ColType]) -> None:
        self.cols = cols

    def getColumnNames(self) -> List[str]:
        """
        Returns the set of column names in this schema.

        Returns:
            List[str]: The list of column names.
        """
        return [c.name for c in self.cols]

    def getColumnTypes(self) -> List[Type[int | str | datetime | bool]]:
        """
        Returns the set of data types for each column in this schema.

        Returns:
            List[Type[int | str | datetime | bool]]: The list of column types.
        """
        return [c.type for c in self.cols]

    def getCol(self, name: str) -> ColType:
        for col in self.cols:
            if col.name == name:
                return col
        raise ValueError(f"No column with name: {name}")

    def __iter__(self):
        yield from self.cols

    def __repr__(self) -> str:
        pair_reprs = [repr(pair) for pair in self.cols]
        return f"Schema(cols={{{', '.join(pair_reprs)}}})"

    def __str__(self) -> str:
        pair_strs = [str(pair) for pair in self.cols]
        return f"Schema with cols: {', '.join(pair_strs)}"

    def __len__(self):
        return len(self.cols)


class Dim:
    """
    Represents the dimensions (rows and columns) of a matrix or table.

    Attributes:
        rows (int): The number of rows in the dimension.
        columns (int): The number of columns in the dimension.
    """

    def __init__(self, rows: int, columns: int) -> None:
        self.rows = rows
        self.columns = columns


class Col:
    """
    Represents a single column in a dataset.

    A column consists of a set of data points, where each data point represents an individual value

    Attributes:
        dataPoints (Union[List[z3.ArithRef], List[z3.SeqRef], List[z3.BoolRef]]): The set of data points that comprise this row.
    """

    def __init__(self, dataPoints: z3.colType) -> None:
        self.dataPoints = dataPoints

    def getDataPoints(self) -> z3.colType:
        return self.dataPoints


class Table:
    """
    Represents a complete table with multiple columns.

    A table consists of a set of columns, where each is composed of data points
    that represent individual values.

    Attributes:
        schema (Schema): The schema of the table.
        dim (Dim): The dimensions of the table.
        table (Dict[str, List[Any]]): The set of columns that comprise this table.
    """

    def __init__(self, schema: Schema, dim: Dim) -> None:
        self.schema = schema
        self.dim = dim
        self.table: Dict[str, List[Any]] = dict()

    def addValue(self, colName: str, index: int, value: Any):
        """
        Add a column to the table

        Args:
            colName (str): The name of the column
            index (int): The row number of the value
            value (Any): The column to be added
        """
        if colName not in self.table:
            self.table[colName] = [None] * self.dim.rows
        self.table[colName][index] = value

    def to_pandas(self) -> pd.DataFrame:
        """
        Converts the table data into a Pandas dataframe.

        Returns:
            pd.DataFrame: A Pandas dataframe containing the table data.

        Raises:
            ValueError: If the table has no data.
        """
        if not self.table:
            raise ValueError("Table is empty. No data to convert to DuckDB.")
        return pd.DataFrame(self.table, columns=self.schema.getColumnNames())  # pyright: ignore

    def to_duckdb(self, conn: DuckDBPyConnection, table_name: str) -> None:
        """
        Converts the table data into a DuckDB table.

        Args:
            conn (DuckDBPyConnection): The DuckDB connection to use:
            table_name (str, optional): The name of the table in DuckDB. Defaults to "tdg_table".

        Raises:
            ValueError: If the table has no data.
        """
        if not self.table:
            raise ValueError("Table is empty. No data to convert to DuckDB.")

        # Convert the table dictionary to a Pandas DataFrame
        df = self.to_pandas()  # noqa: F841

        # Connect to an in-memory DuckDB instance and create the table
        conn.execute(f"CREATE TABLE {table_name} AS SELECT * FROM df")


__all__ = [
    "ColType",
    "Schema",
    "Dim",
    "Col",
    "Table",
]
