"""Exceptions raised by ggrust.

Every class subclasses ``GgrustError`` and the builtin that matches the problem
(``ValueError`` for invalid values, ``TypeError`` for unsupported types), so
``except ValueError`` keeps working as with lets_plot. Errors from the Rust engine
arrive as these classes with structured attributes (``path``, ``column``, ...); the
message always says what was wrong, where, what is accepted and how to fix it.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Iterable

__all__ = [
    "ColumnNotFoundError",
    "DataError",
    "DatasetNotFoundError",
    "ExportError",
    "FontError",
    "FontNotFoundError",
    "GgrustError",
    "GuideNotFoundError",
    "PlotFileError",
    "PlotIndexError",
    "RenderError",
    "SpecError",
    "UnsupportedColumnTypeError",
    "UnsupportedDataError",
    "UnsupportedValueError",
]


class GgrustError(Exception):
    """Base class of every ggrust error.

    Examples
    --------
    >>> from ggrust import *
    >>> from ggrust.errors import GgrustError
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> chart = ggplot(data, aes("weight", "mpg")) + geom_point(size="big")
    >>> try:
    ...     svg = chart.to_svg()
    ... except GgrustError as error:
    ...     print(type(error).__name__)
    SpecError
    """


class SpecError(GgrustError, ValueError):
    """Raised when the plot specification holds an invalid value.

    Parameters
    ----------
    message : str
        The full message, including the location.
    path : str, default=""
        Spec path of the offending value, e.g. ``"layers[0].size"`` or
        ``"theme.axis_text.size"``; empty when the error concerns the whole spec.
    reason : str, default=""
        What was wrong and what is accepted, without the location.

    Examples
    --------
    >>> from ggrust import *
    >>> from ggrust.errors import SpecError
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> chart = ggplot(data, aes("weight", "mpg")) + geom_point(shape="star")
    >>> try:
    ...     svg = chart.to_svg()
    ... except SpecError as error:
    ...     print(error.path)
    layers[0].shape
    """

    def __init__(self, message: str, *, path: str = "", reason: str = "") -> None:
        super().__init__(message)
        self.path = path
        self.reason = reason or message


class UnsupportedValueError(SpecError, TypeError):
    """Raised when a spec holds a value of a type the engine cannot receive.

    Parameters
    ----------
    message : str
        The full message, including the location.
    path : str, default=""
        Spec path of the value.
    reason : str, default=""
        What was wrong, without the location.

    Examples
    --------
    >>> from ggrust import *
    >>> from ggrust.errors import UnsupportedValueError
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> chart = ggplot(data, aes("weight", "mpg")) + geom_point(size=object())
    >>> try:
    ...     svg = chart.to_svg()
    ... except UnsupportedValueError as error:
    ...     print(error.path)
    layers[0].size
    """


class DataError(GgrustError, ValueError):
    """Raised when the plot data cannot be read (malformed or inconsistent columns).

    Parameters
    ----------
    message : str
        The full message.

    Examples
    --------
    >>> from ggrust import *
    >>> from ggrust.errors import DataError
    >>> data = {"weight": [2.6, 2.9, 3.2], "mpg": [31, 28]}  # columns of different lengths
    >>> chart = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> try:
    ...     svg = chart.to_svg()
    ... except DataError:
    ...     print("the columns do not line up")
    the columns do not line up
    """

    def __init__(self, message: str) -> None:
        super().__init__(message)


class UnsupportedDataError(DataError, TypeError):
    """Raised when the data object is not a kind ggrust reads (e.g. a Series).

    Parameters
    ----------
    message : str
        The full message, listing the accepted kinds.

    Examples
    --------
    >>> from ggrust import *
    >>> import polars as pl
    >>> from ggrust.errors import UnsupportedDataError
    >>> series = pl.Series("mpg", [31, 28, 24])
    >>> chart = ggplot(series, aes("mpg")) + geom_histogram()
    >>> try:
    ...     svg = chart.to_svg()
    ... except UnsupportedDataError:
    ...     chart = ggplot(series.to_frame(), aes("mpg")) + geom_histogram()
    """


class ColumnNotFoundError(DataError):
    """Raised when a mapping refers to a column that is not in the data.

    Parameters
    ----------
    message : str
        The full message, listing the available columns and a close match.
    column : str
        The missing column.
    available : list of str
        Names of the columns the data has.
    path : str, default=""
        Spec path of the mapping that names the column, e.g. ``"layers[0].mapping.x"``.

    Examples
    --------
    >>> from ggrust import *
    >>> from ggrust.errors import ColumnNotFoundError
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> chart = ggplot(data, aes("wieght", "mpg")) + geom_point()
    >>> try:
    ...     svg = chart.to_svg()
    ... except ColumnNotFoundError as error:
    ...     print(error.column, error.available)
    wieght ['weight', 'mpg']
    """

    def __init__(self, message: str, *, column: str, available: list[str], path: str = "") -> None:
        super().__init__(message)
        self.column = column
        self.available = available
        self.path = path


class UnsupportedColumnTypeError(DataError, TypeError):
    """Raised when a mapped column has a type ggrust cannot plot.

    Parameters
    ----------
    message : str
        The full message.
    column : str
        The column.
    data_type : str
        Its Arrow type, e.g. ``"Duration(ms)"``.
    path : str, default=""
        Spec path of the mapping that reads the column.

    Examples
    --------
    >>> from ggrust import *
    >>> import polars as pl
    >>> from ggrust.errors import UnsupportedColumnTypeError
    >>> data = pl.DataFrame({"tags": [["a"], ["b", "c"]], "count": [1, 2]})
    >>> chart = ggplot(data, aes("tags", "count")) + geom_point()
    >>> try:
    ...     svg = chart.to_svg()
    ... except UnsupportedColumnTypeError as error:
    ...     print(error.column)
    tags
    """

    def __init__(self, message: str, *, column: str, data_type: str, path: str = "") -> None:
        super().__init__(message)
        self.column = column
        self.data_type = data_type
        self.path = path


class RenderError(GgrustError, ValueError):
    """Raised when drawing the output fails, e.g. for an image too large to allocate.

    Parameters
    ----------
    message : str
        The full message.

    Examples
    --------
    >>> from ggrust import *
    >>> from ggrust.errors import RenderError
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> chart = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> try:
    ...     png = chart.to_png(None, w=100_000, h=100_000, unit="px")
    ... except RenderError:
    ...     print("too large to draw")
    too large to draw
    """

    def __init__(self, message: str) -> None:
        super().__init__(message)


class ExportError(GgrustError, ValueError):
    """Raised when export options are invalid (file name, unit, size, dpi, scale).

    Parameters
    ----------
    message : str
        The full message.

    Examples
    --------
    >>> from ggrust import *
    >>> from ggrust.errors import ExportError
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> chart = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> try:
    ...     ggsave(chart, "fuel.xyz")
    ... except ExportError:
    ...     print("use .svg, .png, .pdf or .html")
    use .svg, .png, .pdf or .html
    """

    def __init__(self, message: str) -> None:
        super().__init__(message)


class FontNotFoundError(GgrustError, FileNotFoundError):
    """Raised when a font file or directory to register does not exist.

    Parameters
    ----------
    message : str
        The full message.
    path : str
        The path that was given.

    Examples
    --------
    >>> from ggrust import register_font
    >>> from ggrust.errors import FontNotFoundError
    >>> try:
    ...     register_font("missing/Arial.ttf")
    ... except FontNotFoundError as error:
    ...     print(error.path)
    missing/Arial.ttf
    """

    def __init__(self, message: str, *, path: str) -> None:
        super().__init__(message)
        self.path = path


class FontError(GgrustError, ValueError):
    """Raised when a font cannot be registered or embedded.

    The file is not a TrueType or OpenType font, has no family name (pass
    ``family=``), or its license forbids embedding it in a PDF.

    Parameters
    ----------
    message : str
        The full message.
    path : str
        The font file, or the family name when embedding fails.

    Examples
    --------
    >>> import tempfile
    >>> from pathlib import Path
    >>> from ggrust import register_font
    >>> from ggrust.errors import FontError
    >>> notes = Path(tempfile.mkdtemp()) / "notes.ttf"
    >>> _ = notes.write_text("not a font")
    >>> try:
    ...     register_font(notes)
    ... except FontError:
    ...     print("not a font file")
    not a font file
    """

    def __init__(self, message: str, *, path: str) -> None:
        super().__init__(message)
        self.path = path


class PlotFileError(GgrustError, ValueError):
    """Raised when a ``.ggrs`` plot file cannot be opened.

    The file is not a ggrust plot file, was written by a newer ggrust, or is damaged:
    an entry is missing, too large, or does not match the SHA-256 hash recorded when
    it was saved. The message names the file and the entry.

    Parameters
    ----------
    message : str
        The full message.

    Examples
    --------
    >>> import os, tempfile
    >>> from ggrust import ggload
    >>> from ggrust.errors import PlotFileError
    >>> path = os.path.join(tempfile.mkdtemp(), "notes.ggrs")
    >>> with open(path, "w") as file:
    ...     _ = file.write("not a plot")
    >>> try:
    ...     ggload(path)
    ... except PlotFileError:
    ...     print("not a plot file")
    not a plot file
    """

    def __init__(self, message: str) -> None:
        super().__init__(message)


class GuideNotFoundError(GgrustError, LookupError):
    """Raised when a plot shows no guide for the asked aesthetic.

    Parameters
    ----------
    message : str
        The full message: the aesthetic, the guides the plot shows and a close match.
    aesthetic : str
        The aesthetic that was asked for.
    available : list of str
        The aesthetics that have a guide.

    Examples
    --------
    >>> from ggrust import *
    >>> from ggrust.errors import GuideNotFoundError
    >>> data = {"mpg": [31, 22], "origin": ["EU", "US"]}
    >>> chart = ggplot(data, aes("mpg", "mpg", color="origin")) + geom_point()
    >>> try:
    ...     keys = chart.guide_data("fill")
    ... except GuideNotFoundError as error:
    ...     print(error.available)
    ['color']
    """

    def __init__(self, message: str, *, aesthetic: str, available: list[str]) -> None:
        super().__init__(message)
        self.aesthetic = aesthetic
        self.available = available


class DatasetNotFoundError(GgrustError, AttributeError):
    """Raised when ``ggrust.data`` has no dataset of the asked name.

    It is an ``AttributeError``, so ``from ggrust.data import name`` raises
    ``ImportError`` as for any missing module attribute.

    Parameters
    ----------
    message : str
        The full message: the name, the available datasets and a close match.
    name : str
        The name that was asked for.
    available : list of str
        The dataset names.

    Examples
    --------
    >>> import ggrust.data
    >>> from ggrust.errors import DatasetNotFoundError
    >>> try:
    ...     frame = ggrust.data.pengiuns
    ... except DatasetNotFoundError as error:
    ...     print(error.available)
    ['economics', 'iris', 'mpg', 'penguins']
    """

    def __init__(self, message: str, *, name: str, available: list[str]) -> None:
        super().__init__(message, name=name)
        self.available = available


class PlotIndexError(GgrustError, IndexError):
    """Raised when a layer or grid cell index is out of range or malformed.

    Parameters
    ----------
    message : str
        The full message: the index, what it indexes and the valid range.
    index : object
        The index that was asked for.
    length : int
        How many layers (or grid cells) there are.

    Examples
    --------
    >>> from ggrust import *
    >>> from ggrust.errors import PlotIndexError
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> chart = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> try:
    ...     frame = chart.layer_data(3)
    ... except PlotIndexError as error:
    ...     print(error.length)
    1
    """

    def __init__(self, message: str, *, index: object, length: int) -> None:
        super().__init__(message)
        self.index = index
        self.length = length


def did_you_mean(word: str, candidates: Iterable[str]) -> str:
    """Return ``"; did you mean 'x'?"`` for the closest candidate, or ``""``.

    Same rule as the engine's suggestions: about one edit (insertion, deletion,
    substitution or swap of neighbours) per three letters, none for one- and
    two-letter words; case differences alone always match. No suggestion when
    several candidates are equally close or ``word`` starts several of them.

    Parameters
    ----------
    word : str
        The misspelt name.
    candidates : iterable of str
        The accepted names.

    Returns
    -------
    str
        The suggestion, ready to append to a message.
    """
    lowered = word.lower()
    limit = len(word) // 3
    best: tuple[int, str] | None = None
    tied = False
    extensions = 0
    for candidate in candidates:
        if candidate == word:
            continue
        candidate_lowered = candidate.lower()
        if candidate_lowered.startswith(lowered) and candidate_lowered != lowered:
            extensions += 1
        distance = _distance(lowered, candidate_lowered)
        if distance > limit or (best is not None and distance > best[0]):
            continue
        tied = best is not None and distance == best[0]
        if not tied:
            best = (distance, candidate)
    if best is None or tied or extensions > 1:
        return ""
    return f"; did you mean {best[1]!r}?"


def _distance(left: str, right: str) -> int:
    """Optimal string alignment distance (Levenshtein plus adjacent swaps)."""
    rows = [[0] * (len(right) + 1) for _ in range(len(left) + 1)]
    for column in range(len(right) + 1):
        rows[0][column] = column
    for row in range(1, len(left) + 1):
        rows[row][0] = row
        for column in range(1, len(right) + 1):
            cost = int(left[row - 1] != right[column - 1])
            best = min(
                rows[row - 1][column] + 1,
                rows[row][column - 1] + 1,
                rows[row - 1][column - 1] + cost,
            )
            if (
                row > 1
                and column > 1
                and left[row - 1] == right[column - 2]
                and left[row - 2] == right[column - 1]
            ):
                best = min(best, rows[row - 2][column - 2] + 1)
            rows[row][column] = best
    return rows[len(left)][len(right)]
