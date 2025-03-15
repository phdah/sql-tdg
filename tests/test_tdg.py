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


class TestTDG(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        queryString = """
            select
                a, b, c, d
            from table
        """
        queryStringWhere = (
            queryString
            + """
                 where a > 100
                    and a <= 900
                    and (a >= 110 or a < 1000) -- Try Paren type
                    and b = "hello" -- Ensures not distinct
                    and b != "no hello" -- Try not equal to
        """
        )
        p = Parser(queryString)
        p.parseQuery()
        pWhere = Parser(queryStringWhere)
        pWhere.parseQuery()

        # Define a small dataset size (e.g., 10 rows)
        cls.schema = Schema(
            cols={
                ColType("a", int),
                ColType("b", str),
                ColType("c", datetime),
                ColType("d", bool),
            },
        )

        cls.d = TDG(schema=cls.schema, conditions=p.conditions)
        cls.dWhere = TDG(schema=cls.schema, conditions=pWhere.conditions)

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


if __name__ == "__main__":
    unittest.main()
