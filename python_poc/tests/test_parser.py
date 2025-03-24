import unittest
from sql_tdg.tdp.parser import Parser


class TestParser(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.emptyQuery = ""
        cls.multipleQuerys = """
            select * from table_1;
            select * from table_2;
        """
        cls.queryString = """
            select
                a, b, c, d
            from table
        """
        cls.queryStringWhere = (
            cls.queryString
            + """
                 where a > 100
                    and a <= 900
                    and (a >= 110 or a < 1000) -- Try Paren type
                    and b = "hello" -- Ensures not distinct
                    and b != "no hello" -- Try not equal to
        """
        )

    def testEmptyQuery(self):
        with self.assertRaises(ValueError):
            p = Parser(self.emptyQuery)
            p.parseQuery()

    def testMultipleQuerys(self):
        with self.assertRaises(NotImplementedError):
            p = Parser(self.multipleQuerys)
            p.parseQuery()

    def testQueryWithoutWhere(self):
        p = Parser(self.queryString)
        p.parseQuery()

        self.assertEqual(p.conditions.cols, set())

    def testQueryWithWhere(self):
        p = Parser(self.queryStringWhere)
        p.parseQuery()

        self.assertEqual(p.conditions.cols, {'a', 'b'})


if __name__ == "__main__":
    unittest.main()
