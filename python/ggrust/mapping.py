"""``as_discrete``: mark a mapped column as discrete, optionally ordered; ``after_stat``.

Ported from lets-plot's ``lets_plot/mapping.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

import re
from typing import TYPE_CHECKING

from ggrust.errors import SpecError, UnsupportedValueError

if TYPE_CHECKING:
    from collections.abc import Sequence
    from typing import Any

__all__ = ["after_stat", "as_discrete"]

# ggdist's ``.width`` keeps its leading dot.
_COMPUTED_NAME = re.compile(r"\.?[A-Za-z_][A-Za-z0-9_]*")


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

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"cylinders": [4, 6, 8, 4, 6, 8], "mpg": [30, 21, 15, 28, 20, 14]}
    >>> plot = ggplot(data, aes(x=as_discrete("cylinders"), y="mpg")) + geom_boxplot()
    """
    if isinstance(variable, str):
        label = variable if label is None else label
        return MappingMeta(
            variable, "as_discrete", levels=levels, label=label, order_by=order_by, order=order
        )
    return variable


def after_stat(name: str) -> str:
    """Map a variable the stat computes (ggplot2 ``after_stat()``).

    Another spelling of lets-plot's ``"..name.."``: ``aes(y=after_stat("density"))``
    is ``aes(y="..density..")``. Only a variable name is accepted, not an
    expression such as ggplot2's ``after_stat(count / max(count))``.

    Parameters
    ----------
    name : str
        A computed variable of the layer's stat, e.g. ``"count"``, ``"density"``,
        ``"level"`` (``".width"`` for ggdist intervals).

    Returns
    -------
    str
        ``"..name.."``, for use in ``aes()``.

    Raises
    ------
    UnsupportedValueError
        If ``name`` is not a string.
    SpecError
        If ``name`` is not a plain variable name (an expression, or empty).

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"height": [150, 152, 158, 160, 161, 165, 170, 171, 178, 185]}
    >>> plot = ggplot(data, aes("height", after_stat("density"))) + geom_histogram(bins=5)
    """
    if not isinstance(name, str):
        reason = f"expected a computed variable name, got {type(name).__name__}"
        raise UnsupportedValueError(f"after_stat({name!r}): {reason}", path="name", reason=reason)
    dotted = len(name) > 4 and name.startswith("..") and name.endswith("..")
    bare = name[2:-2] if dotted else name
    if not _COMPUTED_NAME.fullmatch(bare):
        reason = (
            "expected a computed variable name such as 'count' or 'density'; "
            "expressions are not supported, compute them in the data instead"
        )
        raise SpecError(f"after_stat({name!r}): {reason}", path="name", reason=reason)
    return f"..{bare}.."
