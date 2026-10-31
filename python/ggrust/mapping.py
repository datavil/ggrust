"""``as_discrete``: mark a mapped column as discrete, optionally ordered.

Ported from lets-plot's ``lets_plot/mapping.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ggrust.errors import SpecError

if TYPE_CHECKING:
    from collections.abc import Sequence
    from typing import Any

__all__ = ["as_discrete"]


class MappingMeta:
    """Annotation of a mapped variable (for example ``as_discrete``)."""

    def __init__(
        self,
        variable: str,
        annotation: str,
        levels: Sequence[Any] | None,
        **parameters: object,
    ) -> None:
        if variable is None:
            raise SpecError("as_discrete(): 'variable' is None; pass a column name")
        if annotation is None:
            raise SpecError("MappingMeta(): 'annotation' is None; pass e.g. 'as_discrete'")
        self.variable = variable
        self.annotation = annotation
        self.levels = levels
        self.parameters = parameters


def as_discrete(
    variable: str | Sequence[Any],
    label: str | None = None,
    order_by: str | None = None,
    order: int | None = None,
    levels: Sequence[Any] | None = None,
) -> MappingMeta | Sequence[Any]:
    """Use a discrete scale for a mapped column, optionally ordering its values.

    Parameters
    ----------
    variable : str
        Column name.
    label : str, optional
        Scale name (axis or legend title).
    order_by : str, optional
        Column to order the categories by.
    order : int, optional
        1 for ascending, -1 for descending.
    levels : list, optional
        Explicit category order.

    Returns
    -------
    MappingMeta
        Variable annotation, to be used inside ``aes()``. Non-string ``variable``
        values (e.g. a list of values) are returned unchanged, as in lets_plot.
    """
    if isinstance(variable, str):
        label = variable if label is None else label
        return MappingMeta(
            variable, "as_discrete", levels=levels, label=label, order_by=order_by, order=order
        )
    return variable
