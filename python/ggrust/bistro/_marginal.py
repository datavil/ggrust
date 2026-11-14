"""The ``marginal`` option of ``joint_plot`` and ``qq_plot``: ``"geom:sides[:size], ..."``.

Parses and checks the string (lets_plot's format) so a bad item fails at
construction with a ``SpecError`` at ``marginal``; the layers are built by the
callers.
"""

from __future__ import annotations

from typing import NamedTuple

from ggrust.errors import SpecError, did_you_mean

#: Accepted marginal geoms (lets_plot's names and their long forms).
MARGINAL_GEOMS: tuple[str, ...] = ("dens", "density", "hist", "histogram", "box", "boxplot")
_SIDES = "tblr"
#: ``ggmarginal``'s size bounds, as a share of the plot.
_SIZE_RANGE = (0.01, 0.95)


class MarginalItem(NamedTuple):
    """One parsed ``geom:sides[:size]`` item."""

    geom: str
    sides: str
    size: float | None


def parse_marginal(marginal: str, function: str) -> list[MarginalItem]:
    """Parse ``marginal`` into items; ``function`` names the caller in errors.

    Parameters
    ----------
    marginal : str
        Items separated by commas, each ``"geom:sides[:size]"`` (spaces allowed).
    function : str
        ``"joint_plot"`` or ``"qq_plot"``, for the error messages.

    Returns
    -------
    list of MarginalItem
        The items in order; empty for an empty string.

    Raises
    ------
    SpecError
        If an item lacks sides, names an unknown geom or side, or has a size that
        is not a number in 0.01..0.95.
    """
    prefix = f"{function}(marginal={marginal!r}):"
    items: list[MarginalItem] = []
    for item in filter(str.strip, marginal.split(",")):
        parts = [part.strip() for part in item.split(":")]
        if len(parts) < 2 or not parts[1]:
            message = (
                f"{prefix} item {item.strip()!r} needs a geom and sides, as "
                "'geom:sides[:size]', e.g. 'hist:tr' or 'dens:tr:0.3'"
            )
            raise SpecError(message, path="marginal")
        geom, sides = parts[0], parts[1]
        if geom not in MARGINAL_GEOMS:
            message = (
                f"{prefix} unknown marginal geom {geom!r}{did_you_mean(geom, MARGINAL_GEOMS)}; "
                f"expected one of {', '.join(repr(name) for name in MARGINAL_GEOMS)}"
            )
            raise SpecError(message, path="marginal")
        unknown = [side for side in sides if side not in _SIDES]
        if unknown:
            message = (
                f"{prefix} unknown side {unknown[0]!r} in {sides!r}; use any of "
                "'t', 'b', 'l', 'r' (top, bottom, left, right)"
            )
            raise SpecError(message, path="marginal")
        items.append(MarginalItem(geom, sides, _size(parts, prefix)))
    return items


def _size(parts: list[str], prefix: str) -> float | None:
    """Return the optional third field as a size in ``_SIZE_RANGE``."""
    if len(parts) < 3 or not parts[2]:
        return None
    low, high = _SIZE_RANGE
    try:
        size = float(parts[2])
    except ValueError:
        size = float("nan")
    if not low <= size <= high:
        message = (
            f"{prefix} marginal size {parts[2]!r} is not a number in [{low}, {high}] "
            "(a share of the plot, such as 0.2)"
        )
        raise SpecError(message, path="marginal")
    return size
