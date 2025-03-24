from z3 import (
    # Types
    Int,
    Ints,
    String,
    Strings,
    Bool,
    Bools,
    Probe,
    # Expressions
    And,
    Length,
    Or,
    Not,
    Distinct,
    # Core
    is_true,
    ModelRef,
    AstVector,
    FuncDeclRef,
    IntNumRef,
    SeqRef,
    ArithRef,
    BoolRef,
    Solver,
    sat,
)
from datetime import datetime
from typing import List, Union

colType = Union[List[ArithRef], List[SeqRef], List[BoolRef]]
valTypeBool = Union[ArithRef, SeqRef, BoolRef]
valTypeNum = Union[ArithRef, SeqRef]
valTypeOrConst = Union[valTypeBool, int, str, datetime, bool, float]


# Timestamp addition
class TimestampRef(IntNumRef):
    """Timestamp values."""
    def __init__(self, ast, ctx=None):
        super().__init__(ast, ctx)


def Timestamp(name: str, ctx=None) -> ArithRef:
    """Return a Timestamp constant named `name`. If `ctx=None`, then the global context is used.

    >>> x = Timestamp('x')
    """
    return Int(name, ctx)


def Timestamps(names, ctx=None) -> list[ArithRef]:
    """Return a tuple of Timestamp constants.

    >>> x, y, z = Timestamps('x y z')
    >>> Sum(x, y, z)
    x + y + z
    """
    return Ints(names, ctx)


def as_timestamp(self) -> datetime:
    """Return a Z3 integer numeral as a Python datetime.

    >>> s = z3.Solver()
    >>> v = Timestamp(1735729932)
    >>> model = s.model()
    >>> value = model[v]
    >>> value.as_timestamp()
    2025-01-01 12:12:12
    """
    return datetime.fromtimestamp(self.as_long())


IntNumRef.as_timestamp = as_timestamp  # pyright: ignore


def to_timestamp(timestamp: str):
    """Return a unix timestamp integer from a string.
    >>> v = "2025-01-01 12:12:12"
    >>> to_timestamp(v)
    1735729932
    """
    return int(datetime.fromisoformat(timestamp).timestamp())


__all__ = [
    # Types
    "Int",
    "Ints",
    "String",
    "Strings",
    "Bool",
    "Bools",
    # Expressions
    "And",
    "Length",
    "Or",
    "Not",
    "Distinct",
    # Core
    "is_true",
    "ModelRef",
    "AstVector",
    "FuncDeclRef",
    "IntNumRef",
    "SeqRef",
    "BoolRef",
    "ArithRef",
    "Solver",
    "sat",
    # Patches
    "TimestampRef",
    "Timestamp",
    "Timestamps",
    "as_timestamp",
    "to_timestamp",
    # Types
    "colType",
    "valTypeBool",
    "valTypeOrConst",
    "Probe",
]
