from datetime import datetime
from typing import Callable, List, Set, Type, Union
import sql_tdg.tdg.z3 as z3
from sql_tdg.tdg.types import (
    Col,
    Schema,
)


class Dim:
    def __init__(self, rows: int, columns: int) -> None:
        self.rows = rows
        self.columns = columns


class Data:
    def __init__(self, schema: Schema, outputSize: int = 10) -> None:
        self.schema = schema
        self.dim = Dim(outputSize, len(self.schema))

    def getColumnNames(self) -> Set[str]:
        return self.schema.getColumnNames()

    def generateColumn(self, col: Col):
        typeFunction = self._getTypeFunction(col.type)
        return self._generateColumn(col.name, typeFunction)

    def _generateColumn(self, name: str, func: Callable):
        return func(" ".join(f"{name}{i}" for i in range(self.dim.rows)))

    def _getTypeFunction(
        self, type: Type[int | str | datetime | bool]
    ) -> Callable[[str], Union[List[z3.ArithRef], List[z3.SeqRef]]]:
        if type is int:
            return z3.Ints
        if type is str:
            return z3.Strings
        if type is datetime:
            return z3.Timestamps
        else:
            raise ValueError(f"No matching function for type {type}")

    def generate(self):
        a0, a1, a2 = z3.Ints("a0 a1 a2")
        b0, b1, b2 = z3.Strings("b0 b1 b2")
        c0, c1, c2 = z3.Timestamps("c0 c1 c2")
        d0, d1, d2 = z3.Bools("d0 d1 d2")
        self.s = z3.Solver()

        # Constraint: At least one value occurs twice
        self.s.add(z3.Or(a0 == a1, a0 == a2, a1 == a2), a0 > 5, a0 + a1 + a2 == 200)
        self.s.add(z3.Length(b0) > 0, z3.Length(b1) > 4, z3.Length(b2) > 0)
        self.s.add(b0 != b2, b1 == b0)
        self.s.add(
            z3.to_timestamp("2023-01-01") < c0,
            c0 < z3.to_timestamp("2024-01-01"),
            c0 > c1,
            c0 < c2,
        )
        self.s.add(z3.And(d0, d2), z3.Not(d1))

    def getData(self):
        if self.s.check() == z3.sat:
            model = self.s.model()
            df = [col for col in model]
            return df
        else:
            print("No solution")
