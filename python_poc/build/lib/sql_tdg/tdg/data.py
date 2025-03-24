from datetime import datetime
from typing import Callable, List, Tuple, Type, Union
from sql_tdg.tdg.conditions import Condition, Conditions
import sql_tdg.tdg.z3 as z3
from sql_tdg.tdg.types import (
    Col,
    ColType,
    Schema,
    Table,
    Dim,
)


class TDG:
    """Table Data Generator (TDG) for generating data based on a schema and constraints.

    Attributes:
        schema (Schema): The database schema defining column types.
        conditions (Conditions): Constraints applied to column values.
        dim (Dim): Dimensions defining the output data size.
        data (Table): Storage for generated table data.
    """

    def __init__(
        self, schema: Schema, conditions: Conditions, outputSize: int = 10
    ) -> None:
        """Initializes the TDG instance.

        Args:
            schema (Schema): The database schema defining column types.
            conditions (Conditions): The conditions to apply during data generation.
            outputSize (int, optional): The number of rows to generate. Defaults to 10.
        """
        self.schema = schema
        self.conditions = conditions
        self.dim = Dim(outputSize, len(self.schema))
        self.data = Table(self.schema, self.dim)
        self._nameIndexSeparator = "__"
        self.s: z3.Solver

    def getColumnNames(self) -> List[str]:
        """Retrieves the column names from the schema.

        Returns:
            List[str]: A list of column names.
        """
        return self.schema.getColumnNames()

    def generateColumn(self, colType: ColType) -> Col:
        """Generates a column based on its type.

        Args:
            colType (ColType): The column type to generate.

        Returns:
            Col: A generated column object.
        """
        typeFunction = self.getTypeFunction(colType.type)
        return self._generateColumn(colType.name, typeFunction)

    def _generateColumn(self, name: str, func: Callable) -> Col:
        """Internal method to generate a Z3 column.

        Args:
            name (str): The name of the column.
            func (Callable): The function to generate Z3 objects.

        Returns:
            Col: The generated column object.
        """
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
        """Maps a Z3 object to its corresponding column type and index.

        Args:
            obj (Union[z3.FuncDeclRef, z3.AstVector, None]): The Z3 object.

        Returns:
            Tuple[ColType, int]: The corresponding column type and row index.
        """
        colNameRaw = obj.name()  # pyright: ignore
        colName, index = colNameRaw.split(self._nameIndexSeparator)
        col = self.schema.getCol(colName)
        return col, int(index)

    def getTypeFunction(
        self, type: Type[Union[int, str, datetime, bool]]
    ) -> Callable[[str], z3.colType]:
        """Maps Python types to corresponding Z3 functions.

        Args:
            type (Type[Union[int, str, datetime, bool]]): The Python type.

        Returns:
            Callable[[str], z3.colType]: The Z3 function corresponding to the type.

        Raises:
            ValueError: If the type has no matching function.
        """
        if type is int:
            return z3.Ints
        if type is str:
            return z3.Strings
        if type is datetime:
            return z3.Timestamps
        if type is bool:
            return z3.Bools
        raise ValueError(f"No matching function for type {type}")

    def getDataValues(
        self, col: ColType, value: Union[z3.FuncDeclRef, z3.AstVector, None]
    ) -> Union[int, str, datetime, bool]:
        """Retrieves the computed value for a column.

        Args:
            col (ColType): The column type.
            value (Union[z3.FuncDeclRef, z3.AstVector, None]): The Z3 computed value.

        Returns:
            Union[int, str, datetime, bool]: The extracted value.

        Raises:
            RuntimeError: If no solution is found in the solver.
            ValueError: If the type is unsupported.
        """
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
        raise ValueError(f"No matching function for type {type}")

    def generate(self) -> None:
        """Generates table data based on schema and conditions.

        Raises:
            RuntimeError: If no valid solution exists for the given constraints.
        """
        self.s = z3.Solver()
        for colType in self.schema:
            colName = colType.name
            generatedCol = self.generateColumn(colType)
            dataPoints = generatedCol.get()

            # TODO: Handle boolean columns correctly
            if colType.type is bool:
                self.s.add(Condition.orBool(dataPoints))
                continue

            if colName in self.conditions.cols:
                conds = self.conditions.conds[colName]

                # Ensure uniqueness only if no equality conditions exist
                if not {True for cond in conds if cond.op == "eq"}:
                    self.s.add(Condition.distinct(dataPoints))

                for cond in conds:
                    self.s.add(cond.opFunc(dataPoints, cond.condition, False))
            else:
                self.s.add(Condition.distinct(dataPoints))

        if self.s.check() != z3.sat:
            raise RuntimeError("No solution to computation")

        model = self.s.model()
        for col in model:
            colType, index = self._objToCol(col)
            value = self.getDataValues(colType, model[col])
            self.data.addValue(colType.name, index, value)

    def getData(self) -> Table:
        """Retrieves the generated table data.

        Returns:
            Table: The table containing the generated data.
        """
        if not self.data.table:
            self.generate()
        return self.data
