"""``annotate()``: a one-off layer from constants, without a data frame.

Follows ggplot2's ``annotate()``. lets_plot has no such function.
"""

from __future__ import annotations

from collections.abc import Iterable, Mapping
from typing import TYPE_CHECKING, Any

from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.core import FeatureSpecArray, LayerSpec, aes

if TYPE_CHECKING:
    from typing import TypeAlias

    from ggrust._typing import Number
    from ggrust.plot.core import FeatureSpec

    _Position: TypeAlias = Number | str | Iterable[Number | str]

__all__ = ["annotate"]

_POSITIONS = ("x", "y", "xmin", "xmax", "ymin", "ymax", "xend", "yend")
"""Position aesthetics: they become a small data frame and train the scales."""

_CONNECTED_GEOMS = ("line", "path", "polygon", "area", "ribbon", "step")
"""Geoms whose rows form one shape: they cannot be split into a layer per row."""


def _vector(value: object) -> list[Any] | None:
    """Return a sequence, array or series as a list, ``None`` for a single value."""
    if isinstance(value, str | bytes | Mapping) or not isinstance(value, Iterable):
        return None
    return list(value)


def annotate(
    geom: str,
    x: _Position | None = None,
    y: _Position | None = None,
    xmin: _Position | None = None,
    xmax: _Position | None = None,
    ymin: _Position | None = None,
    ymax: _Position | None = None,
    xend: _Position | None = None,
    yend: _Position | None = None,
    **other_args: Any,
) -> FeatureSpec:
    """Add a one-off layer from constants, without a data frame.

    The positions train the axes, as data would. The layer inherits nothing from
    ``ggplot()``, shows in no legend and has no tooltips unless ``tooltips`` is given.

    Parameters
    ----------
    geom : str
        The geom name, without ``geom_``: ``"text"``, ``"rect"``, ``"segment"``, ...
    x, y, xmin, xmax, ymin, ymax, xend, yend : number, str or sequence, optional
        Positions. Single values repeat to the length of the sequences.
    **other_args
        Other aesthetics and parameters of the geom, for example ``label``, ``color``
        or ``size``. A sequence gives one value per row.

    Returns
    -------
    FeatureSpec
        A layer, or several (one per row) when a sequence is given for a style
        aesthetic such as ``color``.

    Raises
    ------
    SpecError
        If ``geom`` is unknown, ``stat``, ``position``, ``data`` or ``mapping`` is given,
        sequences differ in length, or a connected geom gets a style sequence.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"day": [1, 2, 3, 4], "sales": [10, 14, 9, 17]}
    >>> plot = ggplot(data, aes("day", "sales")) + geom_line()
    >>> plot = plot + annotate("rect", xmin=2, xmax=3, ymin=8, ymax=18, alpha=0.2)
    >>> plot = plot + annotate("text", x=4, y=17, label="record", vjust=-1)
    """
    import ggrust.plot as plot_module

    function = getattr(plot_module, f"geom_{geom}", None) if isinstance(geom, str) else None
    if function is None:
        names = [
            name.removeprefix("geom_") for name in dir(plot_module) if name.startswith("geom_")
        ]
        raise SpecError(
            f"annotate: unknown geom {geom!r}; pass a geom name without 'geom_', "
            f"such as 'text', 'rect' or 'segment'{did_you_mean(str(geom), names)}"
        )
    for name in ("stat", "position", "data", "mapping"):
        if name in other_args:
            raise SpecError(
                f"annotate: {name}= is not accepted, an annotation is drawn as given; "
                f"use geom_{geom}({name}=...) for that"
            )

    given = (x, y, xmin, xmax, ymin, ymax, xend, yend)
    positions = {
        name: value for name, value in zip(_POSITIONS, given, strict=True) if value is not None
    }
    columns = {name: _vector(value) for name, value in positions.items()}
    styles = {name: _vector(value) for name, value in other_args.items()}
    style_names = [name for name, value in styles.items() if value is not None and name != "label"]
    vectors = {name: len(value) for name, value in (columns | styles).items() if value is not None}
    lengths = {length for length in vectors.values() if length != 1}
    if len(lengths) > 1:
        details = ", ".join(f"{name} ({length})" for name, length in vectors.items())
        raise SpecError(f"annotate: sequences differ in length: {details}; make them equal")
    row_count = lengths.pop() if lengths else 1
    if style_names and geom in _CONNECTED_GEOMS:
        raise SpecError(
            f"annotate: {', '.join(style_names)} must be single values for geom '{geom}', "
            "whose rows form one shape; use several annotate() layers"
        )

    def column(name: str, source: dict[str, Any], vector: list[Any] | None) -> list[Any]:
        if vector is None:
            return [source[name]] * row_count
        return vector * row_count if len(vector) == 1 else vector

    frame = {name: column(name, positions, columns[name]) for name in positions}
    constants = {name: value for name, value in other_args.items() if styles[name] is None}
    if styles.get("label") is not None:
        frame["label"] = column("label", other_args, styles["label"])
    constants.setdefault("tooltips", "none")

    def layer(rows: dict[str, list[Any]], options: dict[str, Any]) -> LayerSpec:
        return function(
            aes(**{name: name for name in rows}) if rows else None,
            data=rows or None,
            inherit_aes=False,
            show_legend=False,
            **options,
        )

    if not style_names:
        return layer(frame, constants)
    layers = []
    for row in range(row_count):
        row_styles = {name: column(name, other_args, styles[name])[row] for name in style_names}
        one_row = {name: [values[row]] for name, values in frame.items()}
        layers.append(layer(one_row, constants | row_styles))
    return FeatureSpecArray(*layers)
