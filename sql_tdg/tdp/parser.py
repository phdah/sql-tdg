from typing import Union
import sqlglot
from sqlglot.expressions import (
    EQ,
    GT,
    GTE,
    LT,
    LTE,
    NEQ,
    And,
    Expression,
    Identifier,
    Or,
    Paren,
)

from sql_tdg.tdg.conditions import Condition, Conditions


class Parser:
    """Parses an SQL query and extracts conditions from the WHERE clause.

    Attributes:
        queryString (str): The SQL query string to parse.
        conditions (Conditions): A collection of parsed conditions.
    """

    def __init__(self, queryString: str) -> None:
        """Initializes the Parser with a SQL query string.

        Args:
            queryString (str): The SQL query string to be parsed.
        """
        self.queryString = queryString
        self.conditions = Conditions()

    def parseQuery(self) -> None:
        """Parses the SQL query and extracts conditions.

        Raises:
            ValueError: If no query is passed.
            NotImplementedError: If the query contains multiple statements.
        """
        p = sqlglot.parse(self.queryString)
        if not p[0]:
            raise ValueError("No query passed")
        if len(p) != 1:
            raise NotImplementedError("Only support single queries")

        if p[0].args.get("where"):
            where = p[0].args["where"].this
            self._getWhere(where)

    def _getWhere(self, obj: sqlglot.Expression) -> None:
        """Processes the WHERE clause of the query.

        Args:
            obj (sqlglot.Expression): The root expression of the WHERE clause.
        """
        self._recurseFindCondition(obj)

    def _recurseFindCondition(self, obj: Expression) -> None:
        """Recursively traverses the WHERE clause to extract conditions.

        Args:
            obj (sqlglot.Expression): The current expression node.

        Raises:
            AttributeError: If the parsed SQL structure is unexpected.
        """
        if isinstance(obj, (And, Or)):  # Handle logical operators
            self._recurseFindCondition(obj.left)
            self._recurseFindCondition(obj.right)
            return

        if isinstance(obj, Paren):  # Handle parentheses
            self._recurseFindCondition(obj.this)
            return

        if isinstance(obj, (EQ, NEQ, GT, GTE, LT, LTE)):
            columnName: str = obj.left.this.this
            conditionOp: str = obj.key
            conditionValue: Union[str, Identifier] = obj.right.this

            if isinstance(
                conditionValue, Identifier
            ):  # Handle column-to-column comparisons
                conditionValue = conditionValue.this

            cond = Condition(columnName, conditionOp, conditionValue)
            self.conditions.add(cond)
        else:
            raise NotImplementedError(f"Doesn't support expression: {type(obj)}")
