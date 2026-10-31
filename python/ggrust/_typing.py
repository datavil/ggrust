"""Type aliases shared by the public API signatures.

Only read by type checkers: import them under ``if TYPE_CHECKING:``, with
``from __future__ import annotations`` in the importing module. Not part of the
runtime API and not re-exported.
"""

from __future__ import annotations

import os
from collections.abc import Callable, Mapping, Sequence
from typing import IO, Any, Literal, TypeAlias

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

Breaks: TypeAlias = Sequence[Any] | dict[str, Any]
"""Scale breaks: a list of values, or a mapping of labels to values."""

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
