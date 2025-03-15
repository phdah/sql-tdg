from datetime import datetime
from typing import Callable, Set, Tuple, Type, Union
import sql_tdg.tdg.z3 as z3
from sql_tdg.tdg.types import (
    Col,
    ColType,
    Schema,
    Table,
    Dim,
)


class Condition:
    @staticmethod
    def distinct(data: z3.colType) -> z3.BoolRef:
        return z3.Distinct(data)

    @staticmethod
    def orBool(data: z3.colType) -> Union[z3.Probe, z3.BoolRef]:
        return z3.Or(data)

    @staticmethod
    def eq(var: z3.valTypeBool, const: z3.valTypeOrConst) -> z3.conditionBool:
        return var == const

    @staticmethod
    def neq(var: z3.valTypeBool, const: z3.valTypeOrConst) -> z3.conditionBool:
        return var != const

    @staticmethod
    def les(var: z3.valTypeNum, const: z3.valTypeOrConst):
        return var < const

    @staticmethod
    def lar(var: z3.valTypeNum, const: z3.valTypeOrConst):
        return var > const

    @staticmethod
    def leseq(var: z3.valTypeNum, const: z3.valTypeOrConst) -> z3.conditionNum:
        return var <= const

    @staticmethod
    def lageq(var: z3.valTypeNum, const: z3.valTypeOrConst) -> z3.conditionNum:
        return var >= const


class TestData:
    def __init__(self, schema: Schema, outputSize: int = 10) -> None:
        self.schema = schema
        self.dim = Dim(outputSize, len(self.schema))
        self.data = Table(self.dim)
        self._nameIndexSeparator = "__"

    def getColumnNames(self) -> Set[str]:
        return self.schema.getColumnNames()

    def generateColumn(self, colType: ColType) -> Col:
        typeFunction = self.getTypeFunction(colType.type)
        return self._generateColumn(colType.name, typeFunction)

    def _generateColumn(self, name: str, func: Callable) -> Col:
        z3Object = func(
            " ".join(
                f"{name}{self._nameIndexSeparator}{index}"
                for index in range(self.dim.rows)
            )
        )
        return Col(z3Object)

    def _objToCol(
        self, obj: Union[z3.FuncDeclRef, z3.AstVector, None]
    ) -> Tuple[ColType, int]:
        colNameRaw = obj.name()  # pyright: ignore
        colName, index = colNameRaw.split(self._nameIndexSeparator)
        col = self.schema.getCol(colName)
        return col, int(index)

    def getTypeFunction(
        self, type: Type[int | str | datetime | bool]
    ) -> Callable[[str], z3.colType]:
        if type is int:
            return z3.Ints
        if type is str:
            return z3.Strings
        if type is datetime:
            return z3.Timestamps
        if type is bool:
            return z3.Bools
        else:
            raise ValueError(f"No matching function for type {type}")

    def getDataValues(
        self, col: ColType, value: Union[z3.FuncDeclRef, z3.AstVector, None]
    ) -> Union[int, str, datetime, bool]:
        if self.s.check() != z3.sat:
            raise RuntimeError("No solution to computation")

        _type = col.type
        if _type is int:
            return value.as_long()  # pyright: ignore
        if _type is str:
            return value.as_string()  # pyright: ignore
        if _type is datetime:
            return value.as_timestamp()  # pyright: ignore
        if _type is bool:
            return z3.is_true(value)  # pyright: ignore
        else:
            raise ValueError(f"No matching function for type {type}")

    def addCondition(self, col: Col, condition: Callable) -> None:
        dataPoints = col.getDataPoints()
        self.s.add(condition(dataPoints))

    def generateCol(self, colType: ColType) -> None:
        col = self.generateColumn(colType)
        # TODO: this should come from the query parser
        if colType.type is bool:
            condition = Condition.orBool
        else:
            condition = Condition.distinct
        self.addCondition(col, condition)

    def generate(self) -> None:
        self.s = z3.Solver()
        for colType in self.schema:
            self.generateCol(colType)

        if self.s.check() != z3.sat:
            raise RuntimeError("No solution to computation")
        model = self.s.model()
        for col in model:
            colType, index = self._objToCol(col)
            value = self.getDataValues(colType, model[col])
            self.data.addValue(colType.name, index, value)

    def getData(self):
        if self.data.table == {}:
            self.generate()
        return self.data
