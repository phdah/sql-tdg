from typing import Dict, List
import unittest
from datetime import datetime
from sql_tdg.tdg.data import TestData
from sql_tdg.tdg.z3 import Solver
from sql_tdg.tdg.types import (
    ColType,
    Schema,
)


class TestTDG(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Define a small dataset size (e.g., 10 rows)
        cls.schema = Schema(
            cols={
                ColType("a", int),
                ColType("b", str),
                ColType("c", datetime),
                ColType("d", bool),
            },
        )

        cls.d = TestData(cls.schema)

    def testDataDim(self):
        dim = self.d.dim
        rows = dim.rows
        columns = dim.columns
        self.assertIs(rows, 10)
        self.assertIs(columns, len(self.schema))

    def testGenerator(self):
        self.d.generate()
        self.assertIsInstance(self.d.s, Solver)

    def testGetData(self):
        data = self.d.getData()
        self.assertIsInstance(data.table, Dict)
        self.assertIsInstance(data.table["a"], List)

    def testDataOutputTypes(self):
        data = self.d.getData()
        table = data.table
        self.assertIsInstance(table["a"][0], int)
        self.assertIsInstance(table["b"][0], str)
        self.assertIsInstance(table["c"][0], datetime)
        self.assertIsInstance(table["d"][0], bool)

    def testDataOutputValues(self):
        data = self.d.getData()
        table = data.table
        self.assertEqual(len(table["a"]), len(set(table["a"])))
        self.assertEqual(len(table["b"]), len(set(table["b"])))
        self.assertEqual(len(table["c"]), len(set(table["c"])))
        self.assertIn(True, table["d"])
        self.assertIn(False, table["d"])


if __name__ == "__main__":
    unittest.main()
