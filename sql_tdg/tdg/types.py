from typing import Set, Type
from datetime import datetime


# Schema
class Col:
    def __init__(self, name: str, type: Type[int | str | datetime | bool]) -> None:
        self.name = name
        self.type = type

    def __repr__(self) -> str:
        return f"Col(name='{self.name}', type={self.type.__name__})"

    def __str__(self) -> str:
        return f"{self.name} ({self.type.__name__})"

    def __hash__(self) -> int:
        return hash((self.name, self.type))

    def __eq__(self, other: object) -> bool:
        """
        Checks if two Col instances are equal.

        Two Col instances are considered equal if and only if their name and type attributes are equal.

        Args:
            other (object): The other object to compare with.

        Returns:
            bool: True if the objects are equal, False otherwise.
        """
        return isinstance(other, Col) and (
            self.name == other.name and self.type == other.type
        )


class Schema:
    def __init__(self, cols: Set[Col]) -> None:
        self.cols = cols

    def getColumnNames(self) -> Set[str]:
        return {c.name for c in self.cols}

    def getColumnTypes(self) -> Set[Type[int | str | datetime | bool]]:
        return {c.type for c in self.cols}

    def __repr__(self) -> str:
        pair_reprs = [repr(pair) for pair in self.cols]
        return f"Schema(cols={{{', '.join(pair_reprs)}}})"

    def __str__(self) -> str:
        pair_strs = [str(pair) for pair in self.cols]
        return f"Schema with cols: {', '.join(pair_strs)}"

    def __len__(self):
        return len(self.cols)


__all__ = [
    "Col",
    "Schema",
]
