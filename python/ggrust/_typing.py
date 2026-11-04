"""Type aliases shared by the public API signatures.

Only read by type checkers: import them under ``if TYPE_CHECKING:``, with
``from __future__ import annotations`` in the importing module. Not part of the
runtime API and not re-exported.
"""

from __future__ import annotations

import os
from collections.abc import Callable, Iterable, Mapping, Sequence
from typing import IO, Any, Literal, TypeAlias, TypedDict

import pandas
import polars as pl

from ggrust._data import ArrowStreamExportable
from ggrust.plot.core import FeatureSpec

Number: TypeAlias = int | float
"""A plain number."""

PlotData: TypeAlias = (
    pl.DataFrame | pl.LazyFrame | pandas.DataFrame | ArrowStreamExportable | Mapping[str, Any]
)
"""Table-like data: polars, pandas, Arrow, or a mapping of column names to values."""

Breaks: TypeAlias = Iterable[Any] | dict[Any, Any]
"""Scale breaks: values (a list or an array), or a dict of labels to values."""

Labels: TypeAlias = Sequence[str] | dict[Any, str]
"""Scale labels: a list of texts, or a mapping of break values to texts."""

Limits: TypeAlias = Sequence[Any]
"""Scale limits: ``[low, high]`` for continuous scales (``None`` keeps a side), or
the levels of a discrete scale."""

Color: TypeAlias = str
"""A color name or hex code such as ``"red"`` or ``"#1f77b4"``."""

Shape: TypeAlias = int | str
"""A point shape code or name."""

Transform: TypeAlias = Literal["identity", "log10", "log2", "symlog", "sqrt", "reverse"]
"""A continuous scale transform."""

Oob: TypeAlias = Literal["censor", "squish"]
"""What a continuous color scale does with values outside its limits."""

Position: TypeAlias = str | FeatureSpec
"""A position name such as ``"dodge"``, or a ``position_*()`` spec."""

Guide: TypeAlias = str | FeatureSpec
"""A guide name such as ``"legend"`` or ``"none"``, or a ``guide_*()`` spec."""

ManualKey: TypeAlias = str | FeatureSpec
"""A manual legend key label, or a ``layer_key()`` spec."""

Sampling: TypeAlias = str | FeatureSpec
"""A lets_plot sampling spec (accepted and ignored: ggrust draws every row)."""

Tooltips: TypeAlias = Literal["none"] | FeatureSpec | Sequence[str]
"""``layer_tooltips()``, a list of column names, or ``"none"``."""

Path: TypeAlias = str | os.PathLike[str]
"""A file path."""

Target: TypeAlias = str | os.PathLike[str] | IO[bytes]
"""A file path, or a binary file-like object to write to."""

ThemeElement: TypeAlias = Mapping[str, Any] | Literal["blank"]
"""An ``element_*()`` value, or ``"blank"`` to hide the element."""

Margins: TypeAlias = Number | Sequence[Number]
"""Margins in pixels: one number, or 2 to 4 numbers as in CSS."""

Function: TypeAlias = Callable[[float], float]
"""A function of one number."""


class AxisScale(TypedDict):
    """One trained position scale of a panel (``PlotSpec.build()``)."""

    discrete: bool
    """Whether the axis shows categories; their positions are then 0, 1, 2, ..."""
    domain: tuple[float, float]
    """The range the panel spans, after limits and expansion, in transformed units."""
    breaks: list[float]
    """Break positions drawn on the axis, in transformed units."""
    labels: list[str]
    """Break labels as drawn, one per break (a superscript written as ``10^6``)."""


class PanelScales(TypedDict):
    """The data x and y scales of one panel (``x`` is the data x under coord_flip)."""

    x: AxisScale
    y: AxisScale


class PlotBuild(TypedDict):
    """What ``PlotSpec.build()`` returns."""

    layers: list[pl.DataFrame]
    """One frame per layer (as ``PlotSpec.layer_data(i)``)."""
    panels: list[PanelScales]
    """One entry per panel, in panel order (one for an unfaceted plot)."""
