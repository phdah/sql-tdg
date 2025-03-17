from typing import Any, Callable, Dict, List, Union

from sqlglot.expressions import Identifier
from sql_tdg.tdg import z3

# A z3 condition
_Cond = Any


class Condition:
    """Represents a condition applied to a column in Z3 solver constraints.

    Attributes:
        colName (str): The name of the column.
        op (str): The operator used in the condition.
        condition (Union[str, int]): The value to compare against.
        opFunc (Callable): The function that applies the operation.
    """

    def __init__(
        self,
        columnName: str,
        conditionOp: str,
        conditionValue: Union[str, int, Identifier],
    ) -> None:
        """Initializes a Condition object.

        Args:
            columnName (str): The name of the column.
            conditionOp (str): The operator (e.g., 'eq', 'lt', 'gt').
            conditionValue (Union[str, int]): The value to compare against.

        Raises:
            NotImplementedError: If the specified operation is not supported.
        """
        self.colName = columnName
        self.op = conditionOp
        self.condition = conditionValue
        opFunc = Condition.conditionMapping.get(conditionOp, None)
        if not opFunc:
            raise NotImplementedError(
                f"Operation doesn't exist, passed was {conditionOp}, available are {Condition.conditionMapping}"
            )
        self.opFunc: Callable = opFunc

    @staticmethod
    def distinct(data: z3.colType) -> z3.BoolRef:
        """Ensures that all values in the given data list are distinct.

        Args:
            data (z3.colType): A list of Z3 expressions.

        Returns:
            z3.BoolRef: A Z3 boolean constraint ensuring distinct values.
        """
        return z3.Distinct(data)

    @staticmethod
    def orBool(data: z3.colType) -> Union[z3.Probe, z3.BoolRef]:
        """Applies a logical OR operation across all elements in data.

        Args:
            data (z3.colType): A list of Z3 boolean expressions.

        Returns:
            Union[z3.Probe, z3.BoolRef]: A Z3 logical OR expression.
        """
        return z3.Or(data)

    @staticmethod
    def eq(
        data: z3.colType, const: z3.valTypeOrConst, negate: bool = False
    ) -> List[_Cond]:
        """Applies equality constraints to each element in data, optionally negating them.

        Args:
            data (z3.colType): A list of Z3 expressions.
            const (z3.valTypeOrConst): A constant value to compare against.
            negate (bool): If True, negates the condition. Defaults to False.

        Returns:
            List[_Cond]: A list of Z3 boolean expressions representing equality.
        """
        conditions = [var == const for var in data]
        return [z3.Not(cond) if negate else cond for cond in conditions]

    @staticmethod
    def neq(
        data: z3.colType, const: z3.valTypeOrConst, negate: bool = False
    ) -> List[_Cond]:
        """Applies inequality constraints to each element in data, optionally negating them.

        Args:
            data (z3.colType): A list of Z3 expressions.
            const (z3.valTypeOrConst): A constant value to compare against.
            negate (bool): If True, negates the condition. Defaults to False.

        Returns:
            List[_Cond]: A list of Z3 boolean expressions representing inequality.
        """
        conditions = [var != const for var in data]
        return [z3.Not(cond) if negate else cond for cond in conditions]

    @staticmethod
    def lt(
        data: List[z3.ArithRef], const: z3.valTypeOrConst, negate: bool = False
    ) -> List[_Cond]:
        """Applies less-than constraints to each element in data, optionally negating them.

        Args:
            data (List[z3.ArithRef]): A list of Z3 arithmetic expressions.
            const (z3.valTypeOrConst): A constant value to compare against.
            negate (bool): If True, negates the condition. Defaults to False.

        Returns:
            List[_Cond]: A list of Z3 boolean expressions representing `<` comparisons.
        """
        conditions = [var < const for var in data]
        return [z3.Not(cond) if negate else cond for cond in conditions]

    @staticmethod
    def gt(
        data: List[z3.ArithRef], const: z3.valTypeOrConst, negate: bool = False
    ) -> List[_Cond]:
        """Applies greater-than constraints to each element in data, optionally negating them.

        Args:
            data (List[z3.ArithRef]): A list of Z3 arithmetic expressions.
            const (z3.valTypeOrConst): A constant value to compare against.
            negate (bool): If True, negates the condition. Defaults to False.

        Returns:
            List[_Cond]: A list of Z3 boolean expressions representing `>` comparisons.
        """
        conditions = [var > const for var in data]
        return [z3.Not(cond) if negate else cond for cond in conditions]

    @staticmethod
    def lte(
        data: List[z3.ArithRef], const: z3.valTypeOrConst, negate: bool = False
    ) -> List[_Cond]:
        """Applies less-than-or-equal-to constraints to each element in data, optionally negating them.

        Args:
            data (List[z3.ArithRef]): A list of Z3 arithmetic expressions.
            const (z3.valTypeOrConst): A constant value to compare against.
            negate (bool): If True, negates the condition. Defaults to False.

        Returns:
            List[_Cond]: A list of Z3 boolean expressions representing `<=` comparisons.
        """
        conditions = [var <= const for var in data]
        return [z3.Not(cond) if negate else cond for cond in conditions]

    @staticmethod
    def gte(
        data: List[z3.ArithRef], const: z3.valTypeOrConst, negate: bool = False
    ) -> List[_Cond]:
        """Applies greater-than-or-equal-to constraints to each element in data, optionally negating them.

        Args:
            data (List[z3.ArithRef]): A list of Z3 arithmetic expressions.
            const (z3.valTypeOrConst): A constant value to compare against.
            negate (bool): If True, negates the condition. Defaults to False.

        Returns:
            List[_Cond]: A list of Z3 boolean expressions representing `>=` comparisons.
        """
        conditions = [var >= const for var in data]
        return [z3.Not(cond) if negate else cond for cond in conditions]

    conditionMapping: Dict[str, Callable] = {
        "eq": eq,
        "neq": neq,
        "gt": gt,
        "gte": gte,
        "lt": lt,
        "lte": lte,
    }


class Conditions:
    """Manages a collection of conditions grouped by column name.

    Attributes:
        cols (set): A set of column names that have conditions.
        conds (Dict[str, List[Condition]]): A dictionary mapping column names to lists of conditions.
    """

    def __init__(self) -> None:
        """Initializes the Conditions object."""
        self.cols = set()
        self.conds: Dict[str, List[Condition]] = {}

    def add(self, cond: Condition) -> None:
        """Adds a condition to the collection.

        Args:
            cond (Condition): The condition to add.
        """
        self.cols.add(cond.colName)
        self.conds.setdefault(cond.colName, []).append(cond)
