from typing import Dict, List
import unittest
from datetime import datetime
from sql_tdg.tdg.data import TDG
from sql_tdg.tdg.z3 import Solver
from sql_tdg.tdg.types import (
    ColType,
    Schema,
)
from sql_tdg.tdp.parser import Parser
import duckdb
import pandas as pd
import pandas.testing as pdt


class TestTDG(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        queryString = """
            select
                a, b, c, d
            from tdg_table
        """
        cls.queryStringWhere = (
            queryString
            + """
                 where a > 100
                    and a <= 900
                    and (a >= 110 or a < 1000) -- Try Paren type
                    and b = 'hello' -- Ensures not distinct
                    and b != 'no hello' -- Try not equal to
        """
        )
        p = Parser(queryString)
        p.parseQuery()
        pWhere = Parser(cls.queryStringWhere)
        pWhere.parseQuery()

        # Define a small dataset size (e.g., 10 rows)
        cls.schema = Schema(
            cols=[
                ColType("a", int),
                ColType("b", str),
                ColType("c", datetime),
                ColType("d", bool),
            ],
        )

        cls.d = TDG(schema=cls.schema, conditions=p.conditions)
        cls.dWhere = TDG(schema=cls.schema, conditions=pWhere.conditions)

        data = {
            "a": [119, 114, 110, 117, 111, 113, 116, 118, 112, 115],
            "b": ["hello"] * 10,
            "c": pd.to_datetime(
                [
                    "1970-01-01 01:00:00",
                    "1970-01-01 01:00:10",
                    "1970-01-01 01:00:11",
                    "1970-01-01 01:00:12",
                    "1970-01-01 01:00:13",
                    "1970-01-01 01:00:14",
                    "1970-01-01 01:00:15",
                    "1970-01-01 01:00:16",
                    "1970-01-01 01:00:17",
                    "1970-01-01 01:00:18",
                ]
            ),
            "d": [True, False, False, False, False, False, False, False, False, False],
        }
        df = pd.DataFrame(data, columns=cls.schema.getColumnNames()) # pyright: ignore
        # Ensure correct dtypes
        df["c"] = pd.to_datetime(df["c"])  # Ensure datetime
        df["a"] = df["a"].astype("int64")  # Ensure integer
        df["d"] = df["d"].astype("bool")  # Ensure boolean
        df["b"] = df["b"].astype("object")  # Ensure object (string)
        cls.dfExpected = df

    def testDataDim(self):
        dim = self.dWhere.dim
        rows = dim.rows
        columns = dim.columns
        self.assertIs(rows, 10)
        self.assertIs(columns, len(self.schema))

    def testGenerator(self):
        self.dWhere.generate()
        self.assertIsInstance(self.dWhere.s, Solver)

    def testGetData(self):
        data = self.dWhere.getData()
        self.assertIsInstance(data.table, Dict)
        self.assertIsInstance(data.table["a"], List)

    def testDataOutputTypes(self):
        data = self.dWhere.getData()
        table = data.table
        self.assertIsInstance(table["a"][0], int)
        self.assertIsInstance(table["b"][0], str)
        self.assertIsInstance(table["c"][0], datetime)
        self.assertIsInstance(table["d"][0], bool)

    def testDataOutputValuesWithoutConditions(self):
        data = self.d.getData()
        table = data.table
        self.assertEqual(
            len(table["a"]), len(set(table["a"])), "Expected to be all distinct"
        )
        self.assertEqual(
            len(table["b"]), len(set(table["b"])), "Expected to be all distinct"
        )
        self.assertEqual(
            len(table["c"]), len(set(table["c"])), "Expected to be all distinct"
        )
        self.assertIn(True, table["d"], "Expected at least one True")
        self.assertIn(False, table["d"], "Expected at least one False")

    def testDataOutputValues(self):
        data = self.dWhere.getData()
        table = data.table
        self.assertEqual(
            len(table["a"]), len(set(table["a"])), "Expected to be all distinct"
        )
        self.assertEqual(len(set(table["b"])), 1, "Expected to be all equal")
        self.assertEqual(
            len(table["c"]), len(set(table["c"])), "Expected to be all distinct"
        )
        self.assertIn(True, table["d"], "Expected at least one True")
        self.assertIn(False, table["d"], "Expected at least one False")

    def testPandasDF(self):
        data = self.dWhere.getData()
        dfActual = data.to_pandas()

        pdt.assert_frame_equal(dfActual, self.dfExpected)

    def testDuckDB(self):
        # Convert expected DataFrame to DuckDB table
        conn = duckdb.connect()
        conn.register("expected_table", self.dfExpected)
        duckdbExpected = conn.table("expected_table")

        data = self.dWhere.getData()
        data.to_duckdb(conn, "tdg_table")
        dfActual = conn.table("tdg_table")

        self.assertEqual(dfActual.fetchall(), duckdbExpected.fetchall())

    def testQueryTestingData(self):
        # Convert expected DataFrame to DuckDB table
        conn = duckdb.connect()
        conn.register("expected_table", self.dfExpected)
        duckdbExpected = conn.table("expected_table")

        data = self.dWhere.getData()
        data.to_duckdb(conn, "tdg_table")
        # TODO: Add "bad" data manually, this should be done in the generator
        conn.sql(
            "insert into tdg_table values (1, 'mark', '1970-01-01 01:00:00', False)"
        )

        df = conn.sql(self.queryStringWhere)

        self.assertEqual(df.fetchall(), duckdbExpected.fetchall())


if __name__ == "__main__":
    unittest.main()
