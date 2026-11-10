"""``funky_heatmap``: a table of bars, circles, rounded squares, pies, text and images.

Column groups, row groups and legends included.

Ports the R package funkyheatmap 0.5 (MIT, see THIRD_PARTY_NOTICES.md): the
argument model (``data``, ``column_info``, ``row_info``, ``column_groups``,
``row_groups``, ``palettes``, ``legends``, ``position_arguments()``), the cell
geometry (``calculate_*_positions``, ``score_to_funky_rectangle``), the default
palettes and the legend layouts. funkyheatmap draws with ggplot2 + ggforce +
patchwork; here every shape becomes an ordinary ggrust layer (``geom_rect``,
``geom_polygon``, ``geom_segment``, ``geom_text``, image layers) on one
``coord_fixed`` canvas, so the result is a normal ``PlotSpec`` that composes with
``+``. One data unit is 0.25 inch, as in funkyheatmap (24 px at 96 dpi).

What it does not do: measure text (column widths come from ``width``, as in
funkyheatmap), or read image formats other than PNG. Differences from
funkyheatmap are listed in deviations.md.
"""

from __future__ import annotations

import base64
import math
import re
import struct
import urllib.request
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from pathlib import Path
from typing import TYPE_CHECKING, Any, Literal

import polars as pl

from ggrust._ggrust import text_width
from ggrust.errors import ColumnNotFoundError, SpecError, did_you_mean
from ggrust.plot.coord import coord_fixed
from ggrust.plot.core import aes
from ggrust.plot.geom import _geom, geom_polygon, geom_rect, geom_segment, geom_text
from ggrust.plot.plot import MAX_HEIGHT, MAX_WIDTH, ggplot, ggsize
from ggrust.plot.scale_identity import scale_color_identity, scale_fill_identity
from ggrust.plot.theme import theme, theme_void

if TYPE_CHECKING:
    from ggrust.plot.core import LayerSpec, PlotSpec

__all__ = ["PositionArguments", "funky_heatmap", "position_arguments"]

FunkyGeom = Literal["funkyrect", "circle", "rect", "bar", "pie", "text", "image"]
"""Cell geoms accepted in ``column_info``."""

PaletteValue = str | Sequence[str] | Mapping[str, str]
"""A palette: a default palette name, a list of colors, or category -> color."""

TableLike = pl.DataFrame | Sequence[Mapping[str, Any]] | Mapping[str, Sequence[Any]]
"""A table given as a polars DataFrame, a list of row dicts or a dict of columns."""

_GEOMS: tuple[str, ...] = ("funkyrect", "circle", "rect", "bar", "pie", "text", "image")
_LEGEND_GEOMS: tuple[str, ...] = ("funkyrect", "circle", "rect", "bar", "pie", "text", "image")

# One funkyheatmap data unit is 0.25 inch (its figure width is range / 4 inches).
_UNIT_PX = 24.0
# ggplot2's .pt: points per mm. Sizes in funkyheatmap are ggplot2 sizes (mm).
_PT = 72.27 / 25.4
_MISSING_COLOR = "#444444FF"
_ROW_BACKGROUND = "#DDDDDD"
# Room around the canvas so glyphs at its edge are not cut (not in funkyheatmap).
_CANVAS_PADDING = 0.2
_MAX_SIZE_PX = min(MAX_WIDTH, MAX_HEIGHT)
_CIRCLE_VERTICES = 72
_CORNER_VERTICES = 12


# --------------------------------------------------------------------------- #
# Position arguments                                                           #
# --------------------------------------------------------------------------- #


@dataclass(frozen=True)
class PositionArguments:
    """Spacing and placement settings of a ``funky_heatmap``; see ``position_arguments``."""

    row_height: float = 1.0
    row_space: float = 0.1
    row_bigspace: float = 1.2
    col_width: float = 1.0
    col_space: float = 0.1
    col_bigspace: float = 0.5
    col_annot_offset: float = 3.0
    col_annot_angle: float = 30.0
    expand_xmin: float = 0.0
    expand_xmax: float = 2.0
    expand_ymin: float = 0.0
    expand_ymax: float = 0.0
    col_annot_position: Literal["top", "bottom"] = "top"
    col_group_radius: float = 0.0
    legend_position: Literal["bottom", "right", "none"] = "bottom"


def position_arguments(
    row_height: float = 1.0,
    row_space: float = 0.1,
    row_bigspace: float = 1.2,
    col_width: float = 1.0,
    col_space: float = 0.1,
    col_bigspace: float = 0.5,
    col_annot_offset: float = 3.0,
    col_annot_angle: float = 30.0,
    expand_xmin: float = 0.0,
    expand_xmax: float = 2.0,
    expand_ymin: float = 0.0,
    expand_ymax: float = 0.0,
    col_annot_position: Literal["top", "bottom"] = "top",
    col_group_radius: float = 0.0,
    legend_position: Literal["bottom", "right", "none"] = "bottom",
) -> PositionArguments:
    """Spacing and placement settings for ``funky_heatmap``, in data units (0.25 inch).

    The first twelve arguments are funkyheatmap's ``position_arguments()``, with the
    same defaults. The last three are ggrust additions.

    Parameters
    ----------
    row_height : float, default=1.0
        Height of a row.
    row_space : float, default=0.1
        Space between rows.
    row_bigspace : float, default=1.2
        Space between row groups.
    col_width : float, default=1.0
        Kept for funkyheatmap compatibility; column widths come from ``column_info``.
    col_space : float, default=0.1
        Space between columns.
    col_bigspace : float, default=0.5
        Space between column groups.
    col_annot_offset : float, default=3.0
        Room reserved for the column names.
    col_annot_angle : float, default=30.0
        Angle of the column names, in degrees.
    expand_xmin, expand_xmax, expand_ymin, expand_ymax : float
        Extra room around the table (defaults 0, 2, 0, 0), e.g. for long column names.
    col_annot_position : {"top", "bottom"}, default="top"
        Where the column names go. ``"bottom"`` writes them below the table, ending at
        the table, and keeps the column groups on top.
    col_group_radius : float, default=0.0
        Corner radius of the column group headers; half the header height (0.5)
        draws pills.
    legend_position : {"bottom", "right", "none"}, default="bottom"
        Where the legends go: in a row below the table (funkyheatmap), in a column to
        its right, or nowhere.

    Returns
    -------
    PositionArguments
        The settings, for ``funky_heatmap(position_args=...)``.

    Raises
    ------
    SpecError
        For a negative size, or an unknown ``col_annot_position`` / ``legend_position``.

    Examples
    --------
    >>> from ggrust.bistro import position_arguments
    >>> position_arguments(col_annot_angle=90, legend_position="right").legend_position
    'right'
    """
    sizes = {
        "row_height": row_height,
        "row_space": row_space,
        "row_bigspace": row_bigspace,
        "col_width": col_width,
        "col_space": col_space,
        "col_bigspace": col_bigspace,
        "col_annot_offset": col_annot_offset,
        "expand_xmin": expand_xmin,
        "expand_xmax": expand_xmax,
        "expand_ymin": expand_ymin,
        "expand_ymax": expand_ymax,
        "col_group_radius": col_group_radius,
    }
    for name, value in [*sizes.items(), ("col_annot_angle", col_annot_angle)]:
        if (
            isinstance(value, bool)
            or not isinstance(value, int | float)
            or not math.isfinite(value)
        ):
            message = f"position_arguments({name}={value!r}): expected a finite number"
            raise SpecError(message, path=name)
    for name, value in sizes.items():
        if value < 0:
            message = f"position_arguments({name}={value!r}): expected a number >= 0"
            raise SpecError(message, path=name)
    if row_height <= 0:
        message = f"position_arguments(row_height={row_height!r}): expected a number > 0"
        raise SpecError(message, path="row_height")
    _check_choice("position_arguments", "col_annot_position", col_annot_position, ("top", "bottom"))
    _check_choice(
        "position_arguments", "legend_position", legend_position, ("bottom", "right", "none")
    )
    return PositionArguments(
        row_height=float(row_height),
        row_space=float(row_space),
        row_bigspace=float(row_bigspace),
        col_width=float(col_width),
        col_space=float(col_space),
        col_bigspace=float(col_bigspace),
        col_annot_offset=float(col_annot_offset),
        col_annot_angle=float(col_annot_angle),
        expand_xmin=float(expand_xmin),
        expand_xmax=float(expand_xmax),
        expand_ymin=float(expand_ymin),
        expand_ymax=float(expand_ymax),
        col_annot_position=col_annot_position,
        col_group_radius=float(col_group_radius),
        legend_position=legend_position,
    )


def _check_choice(function: str, name: str, value: object, accepted: Sequence[str]) -> None:
    """Raise ``SpecError`` when ``value`` is not one of ``accepted``."""
    if isinstance(value, str) and value in accepted:
        return
    suggestion = did_you_mean(str(value), accepted)
    quoted = ", ".join(repr(choice) for choice in accepted)
    message = f"{function}({name}={value!r}): expected one of {quoted}{suggestion}"
    raise SpecError(message, path=name)


# --------------------------------------------------------------------------- #
# Palettes                                                                     #
# --------------------------------------------------------------------------- #


def _hexes(codes: str) -> list[str]:
    """Turn space-separated ``RRGGBB`` codes into ``#RRGGBB`` colors."""
    return ["#" + code for code in codes.split()]


_BREWER: dict[str, list[str]] = {
    "Blues": _hexes("F7FBFF DEEBF7 C6DBEF 9ECAE1 6BAED6 4292C6 2171B5 08519C 08306B"),
    "Reds": _hexes("FFF5F0 FEE0D2 FCBBA1 FC9272 FB6A4A EF3B2C CB181D A50F15 67000D"),
    "YlOrBr": _hexes("FFFFE5 FFF7BC FEE391 FEC44F FE9929 EC7014 CC4C02 993404 662506"),
    "Greens": _hexes("F7FCF5 E5F5E0 C7E9C0 A1D99B 74C476 41AB5D 238B45 006D2C 00441B"),
    "Greys": _hexes("FFFFFF F0F0F0 D9D9D9 BDBDBD 969696 737373 525252 252525 000000"),
    "Set3": _hexes(
        "8DD3C7 FFFFB3 BEBADA FB8072 80B1D3 FDB462 B3DE69 FCCDE5 D9D9D9 BC80BD CCEBC5 FFED6F"
    ),
    "Set1": _hexes("E41A1C 377EB8 4DAF4A 984EA3 FF7F00 FFFF33 A65628 F781BF 999999"),
    "Set2": _hexes("66C2A5 FC8D62 8DA0CB E78AC3 A6D854 FFD92F E5C494 B3B3B3"),
    "Dark2": _hexes("1B9E77 D95F02 7570B3 E7298A 66A61E E6AB02 A6761D 666666"),
}


def _smear(colors: Sequence[str], count: int = 101) -> list[str]:
    """R's ``colorRampPalette(colors)(count)``: linear RGB interpolation, rounded half up."""
    channels = [_hex_rgb(color) for color in colors]
    last = len(channels) - 1
    result = []
    for index in range(count):
        position = index / (count - 1) * last
        low = min(math.floor(position), last - 1) if last > 0 else 0
        weight = position - low
        high = min(low + 1, last)
        rgb = [
            channels[low][channel] * (1 - weight) + channels[high][channel] * weight
            for channel in range(3)
        ]
        result.append("#" + "".join(f"{int(value + 0.5):02X}" for value in rgb))
    return result


def _hex_rgb(color: str) -> tuple[int, int, int]:
    """``#RRGGBB`` -> (red, green, blue)."""
    return int(color[1:3], 16), int(color[3:5], 16), int(color[5:7], 16)


_NUMERICAL_PALETTES: dict[str, list[str]] = {
    "Blues": _smear([*_BREWER["Blues"], "#011636"][::-1]),
    "Reds": _smear(_BREWER["Reds"][:7][::-1]),
    "YlOrBr": _smear(_BREWER["YlOrBr"][:6][::-1]),
    "Greens": _smear([*_BREWER["Greens"][1:], "#00250F"][::-1]),
    "Greys": _smear(_BREWER["Greys"][1:][::-1]),
}
_NUMERICAL_PALETTES["Grays"] = _NUMERICAL_PALETTES["Greys"]
_CATEGORICAL_PALETTES: dict[str, list[str]] = {
    name: _BREWER[name] for name in ("Set3", "Set1", "Set2", "Dark2")
}
_NUMERICAL_ROTATION = ["Blues", "Reds", "YlOrBr", "Greens", "Greys"]
_CATEGORICAL_ROTATION = ["Set3", "Set1", "Set2", "Dark2"]


@dataclass(frozen=True)
class _Palette:
    """A resolved palette: its colors, and the category of each color when named."""

    colors: tuple[str, ...]
    names: tuple[str, ...] | None = None

    def lookup(self, value: object) -> str:
        """Return the color of a cell value (category name or number in 0..1)."""
        if value is None:
            return _MISSING_COLOR
        if isinstance(value, str):
            if self.names is None or value not in self.names:
                return _MISSING_COLOR
            return self.colors[self.names.index(value)]
        if isinstance(value, bool) or not isinstance(value, int | float):
            return _MISSING_COLOR
        if not math.isfinite(value):
            return _MISSING_COLOR
        index = round(value * (len(self.colors) - 1))  # R round(): half to even
        if not 0 <= index < len(self.colors):
            return _MISSING_COLOR
        return self.colors[index]

    def middle(self) -> str:
        """Funkyheatmap's ``palette_mids``: the color at ``ceiling(n / 2)``."""
        return self.colors[math.ceil(len(self.colors) / 2) - 1]


# --------------------------------------------------------------------------- #
# Column, row and group information                                            #
# --------------------------------------------------------------------------- #


@dataclass
class _Column:
    """One ``column_info`` row, with funkyheatmap's defaults filled in."""

    id: str
    name: str | None
    geom: str
    group: str | None
    palette: str | None
    width: float
    overlay: bool
    legend: bool
    draw_outline: bool
    id_color: str | None
    id_size: str | None
    options: dict[str, Any] = field(default_factory=dict)
    # Filled by the column layout.
    xmin: float = 0.0
    xmax: float = 0.0
    xwidth: float = 0.0
    x: float = 0.0


@dataclass
class _Row:
    """One ``row_info`` row and its position."""

    id: str
    group: str | None
    y: float = 0.0
    ymin: float = 0.0
    ymax: float = 0.0
    colour_background: bool = False


def _as_records(value: TableLike, argument: str) -> list[dict[str, Any]]:
    """Rows of a table argument as dicts (keys in column order)."""
    if isinstance(value, pl.DataFrame):
        return value.to_dicts()
    if isinstance(value, Mapping):
        try:
            return pl.DataFrame(dict(value)).to_dicts()
        except Exception as error:  # polars raises several types for ragged input
            message = f"funky_heatmap({argument}=...): could not read the dict of columns: {error}"
            raise SpecError(message, path=argument) from error
    if isinstance(value, Sequence) and not isinstance(value, str | bytes):
        records = []
        for index, row in enumerate(value):
            if not isinstance(row, Mapping):
                message = (
                    f"funky_heatmap({argument}[{index}]): expected a dict, got "
                    f"{type(row).__name__}; pass a polars DataFrame or a list of dicts"
                )
                raise SpecError(message, path=f"{argument}[{index}]")
            records.append(dict(row))
        return records
    message = (
        f"funky_heatmap({argument}=...): expected a polars DataFrame, a list of dicts or a "
        f"dict of columns, got {type(value).__name__}"
    )
    raise SpecError(message, path=argument)


def _key_order(records: Sequence[Mapping[str, Any]]) -> list[str]:
    """Every key of ``records``, in order of first appearance."""
    keys: dict[str, None] = {}
    for record in records:
        for key in record:
            keys[key] = None
    return list(keys)


def _title_case(text: str) -> str:
    """Stringr's ``str_to_title`` for plain words: capital first letter, rest lower."""
    return re.sub(
        r"[^\W_]+(?:'[^\W_]+)*", lambda word: word[0][:1].upper() + word[0][1:].lower(), text
    )


def _infer_geom(dtype: pl.DataType) -> str | None:
    """Funkyheatmap's geom guess from a column type."""
    if dtype.is_numeric():
        return "funkyrect"
    if isinstance(dtype, pl.String | pl.Categorical | pl.Enum | pl.Boolean):
        return "text"
    if isinstance(dtype, pl.Struct):
        return "pie"
    return None


def _is_color(value: str) -> bool:
    """Tell whether ``value`` is ``#RRGGBB`` (funkyheatmap's ``is_color``, no R names)."""
    return (
        len(value) == 7
        and value.startswith("#")
        and all(character in "0123456789abcdefABCDEF" for character in value[1:])
    )


def _missing_column(column: str, frame: pl.DataFrame, path: str, what: str) -> ColumnNotFoundError:
    """Build a ``ColumnNotFoundError`` for ``column`` with a close match."""
    suggestion = did_you_mean(column, frame.columns)
    message = (
        f"funky_heatmap({path}={column!r}): {what} {column!r} is not a column of `data`"
        f"{suggestion}; available: {frame.columns}"
    )
    return ColumnNotFoundError(message, column=column, available=list(frame.columns), path=path)


def _columns(column_info: TableLike | None, frame: pl.DataFrame) -> list[_Column]:
    """``verify_column_info``: read ``column_info`` and fill funkyheatmap's defaults."""
    if column_info is None:
        records: list[dict[str, Any]] = [{"id": column} for column in frame.columns]
    else:
        records = _as_records(column_info, "column_info")
    if not records:
        message = "funky_heatmap(column_info=...): expected at least one column"
        raise SpecError(message, path="column_info")
    # Unnest `options` into the row (funkyheatmap's unnest(options)).
    flat = []
    for index, record in enumerate(records):
        options = record.pop("options", None) or {}
        if not isinstance(options, Mapping):
            message = (
                f"funky_heatmap(column_info[{index}].options): expected a dict, got "
                f"{type(options).__name__}"
            )
            raise SpecError(message, path=f"column_info[{index}].options")
        flat.append({**record, **options})
    if any("id_colour" in record for record in flat):
        for record in flat:
            if "id_colour" in record:
                record["id_color"] = record.pop("id_colour")
    # A key missing from an entry gets funkyheatmap's default for an absent column;
    # an explicit None is R's NA. (A DataFrame gives every entry every key.)
    has_group = any(record.get("group") not in (None, "") for record in flat)
    columns = []
    for index, record in enumerate(flat):
        path = f"column_info[{index}]"
        column_id = record.get("id")
        if not isinstance(column_id, str):
            message = f"funky_heatmap({path}.id): expected a column name (str), got {column_id!r}"
            raise SpecError(message, path=f"{path}.id")
        if column_id not in frame.columns:
            raise _missing_column(column_id, frame, f"{path}.id", "column")
        name = record["name"] if "name" in record else _title_case(column_id.replace("_", " "))
        geom = record.get("geom")
        if "geom" not in record:
            geom = _infer_geom(frame.schema[column_id])
        if geom not in _GEOMS:
            suggestion = did_you_mean(str(geom), _GEOMS)
            message = (
                f"funky_heatmap({path}.geom={geom!r}) for column {column_id!r}: expected one of "
                f"{', '.join(repr(value) for value in _GEOMS)}{suggestion}"
            )
            raise SpecError(message, path=f"{path}.geom")
        id_color = record.get("id_color")
        if id_color is not None:
            if not isinstance(id_color, str):
                message = (
                    f"funky_heatmap({path}.id_color): expected a column name, got {id_color!r}"
                )
                raise SpecError(message, path=f"{path}.id_color")
            if id_color not in frame.columns and not _is_color(id_color):
                raise _missing_column(id_color, frame, f"{path}.id_color", "color column")
        elif geom not in ("text", "image"):
            id_color = column_id
        id_size = record.get("id_size")
        if id_size is not None:
            if not isinstance(id_size, str) or id_size not in frame.columns:
                raise _missing_column(str(id_size), frame, f"{path}.id_size", "size column")
        elif geom not in ("text", "image", "rect"):
            id_size = column_id
        group = record.get("group") if has_group else None
        if group == "":
            group = None
        palette = record.get("palette")
        if "palette" not in record:
            palette = {
                "text": None,
                "image": None,
                "pie": "categorical_palette",
            }.get(geom, "numerical_palette")
        if "width" in record:
            width = record["width"]
            width = 1.0 if width is None else width
        else:
            width = {"text": 6.0, "bar": 4.0}.get(geom, 1.0)
        if (
            isinstance(width, bool)
            or not isinstance(width, int | float)
            or not math.isfinite(width)
        ):
            message = f"funky_heatmap({path}.width={width!r}): expected a number"
            raise SpecError(message, path=f"{path}.width")
        overlay = _flag(record.get("overlay"), default=False, path=f"{path}.overlay")
        legend = _flag(record.get("legend"), default=geom != "text", path=f"{path}.legend")
        draw_outline = _flag(record.get("draw_outline"), default=True, path=f"{path}.draw_outline")
        reserved = {
            "id", "name", "geom", "group", "palette", "width", "overlay", "legend",
            "draw_outline", "id_color", "id_size",
        }  # fmt: skip
        options = {key: value for key, value in record.items() if key not in reserved}
        label = options.get("label")
        if label is not None and (not isinstance(label, str) or label not in frame.columns):
            raise _missing_column(str(label), frame, f"{path}.label", "label column")
        columns.append(
            _Column(
                id=column_id,
                name=name if name is None else str(name),
                geom=geom,
                group=group if group is None else str(group),
                palette=palette if palette is None else str(palette),
                width=float(width),
                overlay=overlay,
                legend=legend,
                draw_outline=draw_outline,
                id_color=id_color,
                id_size=id_size,
                options=options,
            )
        )
    return columns


def _flag(value: object, *, default: bool, path: str) -> bool:
    """Read a boolean option, ``default`` when missing."""
    if value is None:
        return default
    if not isinstance(value, bool):
        message = f"funky_heatmap({path}={value!r}): expected True or False"
        raise SpecError(message, path=path)
    return value


def _rows(row_info: TableLike | None, frame: pl.DataFrame, ids: list[str]) -> list[_Row]:
    """``verify_row_info``: the rows to draw, in order."""
    if row_info is None:
        return [_Row(id=row_id, group=None) for row_id in ids]
    records = _as_records(row_info, "row_info")
    known = set(ids)
    rows = []
    for index, record in enumerate(records):
        row_id = record.get("id")
        if row_id is None:
            message = f"funky_heatmap(row_info[{index}]): missing 'id'"
            raise SpecError(message, path=f"row_info[{index}].id")
        row_id = str(row_id)
        if row_id not in known:
            suggestion = did_you_mean(row_id, ids)
            message = (
                f"funky_heatmap(row_info[{index}].id={row_id!r}): no row of `data` has this id"
                f"{suggestion}"
            )
            raise SpecError(message, path=f"row_info[{index}].id")
        group = record.get("group")
        rows.append(_Row(id=row_id, group=None if group is None else str(group)))
    if not rows:
        message = "funky_heatmap(row_info=...): expected at least one row"
        raise SpecError(message, path="row_info")
    return rows


def _groups(
    groups: TableLike | None, used: list[str | None], argument: str
) -> tuple[list[dict[str, Any]], list[str]] | None:
    """``verify_column_groups`` / ``verify_row_groups``: group records and level keys.

    Returns the records and every key in order (levels are the keys other than
    ``group`` and ``palette``), or None when nothing is grouped.
    """
    used_groups = [group for group in dict.fromkeys(used) if group is not None]
    if groups is None:
        if not used_groups:
            return None
        records = [{"group": group} for group in used_groups]
    else:
        records = _as_records(groups, argument)
    keys = _key_order(records)
    if "group" not in keys:
        message = f"funky_heatmap({argument}=...): every row needs a 'group' key"
        raise SpecError(message, path=argument)
    known = [str(record.get("group")) for record in records]
    for group in used_groups:
        if group not in known:
            suggestion = did_you_mean(group, known)
            message = (
                f"funky_heatmap({argument}=...): group {group!r} is used but not described"
                f"{suggestion}; described groups: {known}"
            )
            raise SpecError(message, path=argument)
    if len([key for key in keys if key not in ("group", "palette")]) == 0:
        for record in records:
            record["level1"] = _title_case(str(record["group"]))
        keys.append("level1")
    return records, keys


# --------------------------------------------------------------------------- #
# Geometry records                                                             #
# --------------------------------------------------------------------------- #


@dataclass
class _Shapes:
    """Shapes in data units, before they become layers (funkyheatmap's geom data)."""

    backgrounds: list[tuple[float, float, float, float]] = field(default_factory=list)
    segments: list[dict[str, Any]] = field(default_factory=list)
    rects: list[dict[str, Any]] = field(default_factory=list)
    circles: list[dict[str, Any]] = field(default_factory=list)
    funkyrects: list[dict[str, Any]] = field(default_factory=list)
    pies: list[dict[str, Any]] = field(default_factory=list)
    images: list[dict[str, Any]] = field(default_factory=list)
    texts: list[dict[str, Any]] = field(default_factory=list)

    def shift(self, dx: float, dy: float) -> _Shapes:
        """Return a copy moved by (dx, dy)."""
        moved = _Shapes(
            backgrounds=[
                (xmin + dx, xmax + dx, ymin + dy, ymax + dy)
                for xmin, xmax, ymin, ymax in self.backgrounds
            ]
        )
        for name in ("segments", "rects", "circles", "funkyrects", "pies", "images", "texts"):
            records = []
            for record in getattr(self, name):
                record = dict(record)
                for key in ("x", "x0", "xend", "xmin", "xmax"):
                    if key in record:
                        record[key] += dx
                for key in ("y", "y0", "yend", "ymin", "ymax"):
                    if key in record:
                        record[key] += dy
                records.append(record)
            setattr(moved, name, records)
        return moved

    def extend(self, other: _Shapes) -> None:
        """Append ``other``'s shapes (drawn above the current ones of the same kind)."""
        for name in (
            "backgrounds", "segments", "rects", "circles", "funkyrects", "pies", "images", "texts",
        ):  # fmt: skip
            getattr(self, name).extend(getattr(other, name))

    def bounds(self) -> tuple[float, float, float, float] | None:
        """Funkyheatmap's ``compute_bounds``: (xmin, xmax, ymin, ymax) of every shape.

        Text counts by its box and, unlike funkyheatmap, also by its measured extent,
        so labels longer than their box are not cut at the canvas edge.
        """
        xs: list[float] = []
        ys: list[float] = []
        for xmin, xmax, ymin, ymax in self.backgrounds:
            xs += [xmin, xmax]
            ys += [ymin, ymax]
        for record in self.segments:
            xs += [record["x"], record["xend"]]
            ys += [record["y"], record["yend"]]
        for record in [*self.rects, *self.funkyrects, *self.texts, *self.images, *self.pies]:
            xs += [record["xmin"], record["xmax"]]
            ys += [record["ymin"], record["ymax"]]
        for record in self.circles:
            xs += [record["x"] - record["r"], record["x"] + record["r"]]
            ys += [record["y"] - record["r"], record["y"] + record["r"]]
        for record in self.texts:
            for x, y in _text_corners(record):
                xs.append(x)
                ys.append(y)
        xs = [value for value in xs if math.isfinite(value)]
        ys = [value for value in ys if math.isfinite(value)]
        if not xs or not ys:
            return None
        return min(xs), max(xs), min(ys), max(ys)


def _funky_rectangle(
    xmin: float, xmax: float, ymin: float, ymax: float, size_value: float
) -> tuple[float, float, float, float, float]:
    """``score_to_funky_rectangle``: a rounded square that becomes a circle for low values."""
    midpoint = 0.8
    if size_value >= midpoint:
        trans = (size_value - midpoint) / (1 - midpoint) / 2 + 0.5
        corner_size = (0.9 - 0.8 * trans) * min(xmax - xmin, ymax - ymin)
        return xmin, xmax, ymin, ymax, corner_size
    x = xmin / 2 + xmax / 2
    y = ymin / 2 + ymax / 2
    trans = size_value / midpoint
    width = (trans * 0.9 + 0.1) * min(xmax - xmin, ymax - ymin)
    return x - width / 2, x + width / 2, y - width / 2, y + width / 2, 0.5


def _number(value: object) -> float | None:
    """Return a finite float, or None for missing / non-numeric values."""
    if value is None or isinstance(value, bool) or not isinstance(value, int | float):
        return None
    value = float(value)
    return value if math.isfinite(value) else None


def _label_text(value: object) -> str | None:
    """R's ``as.character`` for a cell value; None for missing."""
    if value is None:
        return None
    if isinstance(value, bool):
        return "TRUE" if value else "FALSE"
    if isinstance(value, float):
        if math.isnan(value):
            return None
        return f"{value:.15g}"
    return str(value)


def _minmax(values: list[Any]) -> list[Any]:
    """Funkyheatmap's ``scale_minmax`` over the finite numbers of a column."""
    numbers = [number for number in (_number(value) for value in values) if number is not None]
    if not numbers:
        return values
    low, high = min(numbers), max(numbers)
    span = high - low
    scaled: list[Any] = []
    for value in values:
        number = _number(value)
        if number is None:
            scaled.append(None)
        elif span == 0:
            scaled.append(None)  # R: 0 / 0 is NaN, nothing is drawn
        else:
            scaled.append((number - low) / span)
    return scaled


def _column_positions(columns: list[_Column], positions: PositionArguments) -> None:
    """``calculate_column_positions``: x extents of every column, in place."""
    cumulative = 0.0
    previous: _Column | None = None
    for column in columns:
        # R: diff(as.integer(factor(group))) is NA next to an NA group, case_when -> col_space.
        do_spacing = (
            previous is not None
            and column.group is not None
            and previous.group is not None
            and column.group != previous.group
        )
        if column.overlay:
            xsep = -previous.width if previous is not None else 0.0
        elif do_spacing:
            xsep = positions.col_bigspace
        else:
            xsep = positions.col_space
        if column.overlay and column.width < 0:
            xwidth = column.width - xsep
        elif column.overlay:
            xwidth = -xsep
        else:
            xwidth = column.width
        cumulative += xwidth + xsep
        column.xmax = cumulative
        column.xmin = cumulative - xwidth
        column.xwidth = xwidth
        column.x = column.xmin + xwidth / 2
        previous = column


def _row_positions(rows: list[_Row], positions: PositionArguments) -> None:
    """``calculate_row_positions``: y extents of every row, in place."""
    group_counts: dict[str | None, int] = {}
    cumulative_space = 0.0
    previous: _Row | None = None
    for index, row in enumerate(rows, start=1):
        group_counts[row.group] = group_counts.get(row.group, 0) + 1
        row.colour_background = group_counts[row.group] % 2 == 1
        do_spacing = previous is not None and row.group != previous.group
        cumulative_space += positions.row_bigspace if do_spacing else positions.row_space
        row.y = -(index * positions.row_height + cumulative_space)
        row.ymin = row.y - positions.row_height / 2
        row.ymax = row.y + positions.row_height / 2
        previous = row


# --------------------------------------------------------------------------- #
# Images                                                                       #
# --------------------------------------------------------------------------- #

_PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


@dataclass(frozen=True)
class _Image:
    """A PNG file as a data URL, with its pixel size."""

    href: str
    width: int
    height: int


def _read_png(source: str, where: str, cache: dict[str, _Image]) -> _Image:
    """Read a PNG file or ``http(s)`` URL once; raise ``SpecError`` when it fails."""
    if source in cache:
        return cache[source]
    try:
        if source.startswith(("http://", "https://")):
            with urllib.request.urlopen(source, timeout=30) as response:
                content = response.read()
        else:
            content = Path(source).read_bytes()
    except (OSError, ValueError) as error:
        message = f"funky_heatmap({where}): cannot read image {source!r}: {error}"
        raise SpecError(message, path=where) from error
    if not content.startswith(_PNG_SIGNATURE) or len(content) < 24:
        message = (
            f"funky_heatmap({where}): {source!r} is not a PNG image; image cells accept PNG "
            "file paths and URLs only"
        )
        raise SpecError(message, path=where)
    width, height = struct.unpack(">II", content[16:24])
    image = _Image(
        href="data:image/png;base64," + base64.b64encode(content).decode("ascii"),
        width=width,
        height=height,
    )
    cache[source] = image
    return image


def _image_path(value: object, options: Mapping[str, Any]) -> str | None:
    """Return a cell's image path: ``directory`` / value . ``extension``, as funkyheatmap."""
    if value is None:
        return None
    if isinstance(value, Path):
        text = str(value)
    elif isinstance(value, str):
        text = value
    else:
        return None
    directory = options.get("directory")
    extension = options.get("extension")
    if directory is not None:
        text = f"{directory}/{text}"
    if extension is not None:
        text = f"{text}.{extension}"
    return text


# --------------------------------------------------------------------------- #
# The table                                                                    #
# --------------------------------------------------------------------------- #


def _cell_records(
    column: _Column,
    rows: list[_Row],
    frame: pl.DataFrame,
    index_of: dict[str, int],
    scale_column: bool,
) -> list[dict[str, Any]]:
    """``make_geom_data_processor`` for one column: one record per drawn row."""

    def values(name: str | None) -> list[Any] | None:
        if name is None or name not in frame.columns:
            return None
        series = frame.get_column(name)
        if isinstance(series.dtype, pl.Categorical | pl.Enum):
            series = series.cast(pl.String)
        return series.to_list()

    value = values(column.id) or []
    color_value = values(column.id_color)
    size_value = values(column.id_size)
    label_value = values(column.options.get("label") or column.id) or []
    numeric = frame.schema[column.id].is_numeric()
    if scale_column and column.options.get("scale", True) is not False:
        if numeric:
            value = _minmax(value)
        if (
            color_value is not None
            and column.id_color is not None
            and frame.schema[column.id_color].is_numeric()
        ):
            color_value = _minmax(color_value)
        if (
            size_value is not None
            and column.id_size is not None
            and frame.schema[column.id_size].is_numeric()
        ):
            size_value = _minmax(size_value)
    records = []
    for row in rows:
        data_index = index_of[row.id]
        records.append(
            {
                "row": row,
                "value": value[data_index],
                "color_value": None if color_value is None else color_value[data_index],
                "size_value": None if size_value is None else size_value[data_index],
                "label_value": label_value[data_index],
            }
        )
    return records


def _fill(column: _Column, palettes: Mapping[str, _Palette], color_value: object) -> str | None:
    """Return the fill of a cell: from the palette, a literal ``id_color``, or None."""
    if column.id_color is not None and _is_color(column.id_color):
        return column.id_color
    if column.palette is None:
        return None
    return palettes[column.palette].lookup(color_value)


def _table_shapes(
    columns: list[_Column],
    rows: list[_Row],
    frame: pl.DataFrame,
    palettes: Mapping[str, _Palette],
    positions: PositionArguments,
    scale_column: bool,
    image_cache: dict[str, _Image],
) -> _Shapes:
    """``calculate_geom_positions`` (cells only): the shapes of every cell."""
    shapes = _Shapes()
    index_of = {
        row_id: index
        for index, row_id in enumerate(frame.get_column("id").cast(pl.String).to_list())
    }
    half_height = positions.row_height / 2
    for column in columns:
        records = _cell_records(column, rows, frame, index_of, scale_column)
        options = column.options
        if column.geom == "bar" and column.draw_outline:
            for x in (column.xmin, column.xmax):
                for row in rows:
                    shapes.segments.append(
                        {"x": x, "xend": x, "y": row.ymin, "yend": row.ymax,
                         "colour": "black", "size": 0.5, "linetype": "dashed"}
                    )  # fmt: skip
        for record in records:
            row: _Row = record["row"]
            cell = {"xmin": column.xmin, "xmax": column.xmax, "ymin": row.ymin, "ymax": row.ymax}
            fill = _fill(column, palettes, record["color_value"])
            size = _number(record["size_value"])
            if column.geom == "circle":
                if size is None:
                    continue
                shapes.circles.append(
                    {"x": column.x, "y": row.y, "r": half_height * size, "fill": fill}
                )
            elif column.geom == "rect":
                shapes.rects.append({**cell, "fill": fill, "border": True, "alpha": 1.0})
            elif column.geom == "funkyrect":
                if size is None:
                    continue
                xmin, xmax, ymin, ymax, corner = _funky_rectangle(
                    column.xmin, column.xmax, row.ymin, row.ymax, size
                )
                shapes.funkyrects.append(
                    {"xmin": xmin, "xmax": xmax, "ymin": ymin, "ymax": ymax,
                     "radius": corner, "fill": fill}
                )  # fmt: skip
            elif column.geom == "bar":
                if size is None:
                    continue
                hjust = _number(options.get("hjust"))
                hjust = 0.0 if hjust is None else hjust
                shapes.rects.append(
                    {
                        **cell,
                        "xmin": column.xmin + (1 - size) * column.xwidth * hjust,
                        "xmax": column.xmax - (1 - size) * column.xwidth * (1 - hjust),
                        "fill": fill,
                        "border": True,
                        "alpha": 1.0,
                    }
                )
            elif column.geom == "text":
                label = _label_text(record["label_value"])
                if label is None or label == "":
                    continue
                shapes.texts.append(
                    {
                        **cell,
                        "label": label,
                        "colour": "black" if column.palette is None else (fill or "black"),
                        "hjust": options.get("hjust"),
                        "vjust": options.get("vjust"),
                        "size": options.get("size"),
                        "fontface": options.get("fontface"),
                        "angle": options.get("angle"),
                    }
                )
            elif column.geom == "pie":
                shapes.pies.extend(
                    _pie_wedges(column, row, record["size_value"], palettes, half_height)
                )
            elif column.geom == "image":
                path = _image_path(record["value"], options)
                if path is None:
                    continue
                where = f"data[{record['row'].id!r}].{column.id}"
                image = _read_png(path, where, image_cache)
                shapes.images.append(
                    {"xmin": column.xmin, "xmax": column.xmin + positions.row_height,
                     "ymin": row.ymin, "ymax": row.ymin + positions.row_height, "image": image}
                )  # fmt: skip
    return shapes


def _pie_wedges(
    column: _Column,
    row: _Row,
    value: object,
    palettes: Mapping[str, _Palette],
    radius: float,
) -> list[dict[str, Any]]:
    """Pie slices of one cell (a struct / dict of category -> amount)."""
    if not isinstance(value, Mapping):
        return []
    amounts = [(str(name), _number(amount) or 0.0) for name, amount in value.items()]
    total = sum(amount for _, amount in amounts)
    if total <= 0:
        return []
    wedges = []
    end = 0.0
    for name, amount in amounts:
        pct = amount / total
        start, end = end, end + pct * 2 * math.pi
        if end == start or pct < 1e-10:
            continue
        fill = palettes[column.palette].lookup(name) if column.palette is not None else None
        wedges.append(
            {"x0": column.x, "y0": row.y, "r": radius, "start": start, "end": end, "fill": fill,
             "xmin": column.x - radius, "xmax": column.x + radius,
             "ymin": row.y - radius, "ymax": row.y + radius}
        )  # fmt: skip
    return wedges


def _annotation_shapes(
    columns: list[_Column],
    rows: list[_Row],
    column_groups: tuple[list[dict[str, Any]], list[str]] | None,
    row_groups: tuple[list[dict[str, Any]], list[str]] | None,
    palettes: Mapping[str, _Palette],
    positions: PositionArguments,
    add_abc: bool,
) -> _Shapes:
    """Row backgrounds, row group labels, column group headers and column names."""
    shapes = _Shapes()
    left = min(column.xmin for column in columns) - 0.25
    right = max(column.xmax for column in columns) + 0.25
    half_space = positions.row_space / 2
    for row in rows:
        if row.colour_background:
            shapes.backgrounds.append((left, right, row.ymin - half_space, row.ymax + half_space))

    if row_groups is not None:
        records, keys = row_groups
        levels = [key for key in keys if key != "group"]
        extents: dict[str, list[float]] = {}
        for level in levels:
            for record in records:
                name = record.get(level)
                group_rows = [row for row in rows if row.group == str(record["group"])]
                if name is None or name == "" or not group_rows:
                    continue
                extent = extents.setdefault(str(name), [math.inf, -math.inf])
                extent[0] = min(extent[0], *(row.ymin for row in group_rows))
                extent[1] = max(extent[1], *(row.ymax for row in group_rows))
        for name, (_, ymax) in sorted(extents.items()):
            ymin = ymax + positions.row_space
            shapes.texts.append(
                {"xmin": -0.5, "xmax": 5.0, "ymin": ymin, "ymax": ymin + positions.row_height,
                 "label": name.replace("\n", " "), "hjust": 0.0, "vjust": 0.5,
                 "fontface": "bold", "colour": "black"}
            )  # fmt: skip

    names_at_bottom = positions.col_annot_position == "bottom"
    table_bottom = min(row.ymin for row in rows)
    if column_groups is not None:
        records, keys = column_groups
        levels = [key for key in keys if key not in ("group", "palette")]
        text_pct = 0.9
        base = 0.0 if names_at_bottom else positions.col_annot_offset
        level_boxes: dict[str, tuple[float, float]] = {}
        cumulative = 0.0
        for level in sorted(levels, key=keys.index, reverse=True):
            newlines = max(str(record.get(level) or "").count("\n") for record in records)
            height = (newlines + 1) * text_pct + (1 - text_pct)
            cumulative += height + positions.row_space
            ymax = base + cumulative - positions.row_space
            level_boxes[level] = (ymax - height, ymax)
        headers = []
        for level in levels:
            level_match = keys.index(level) + 1
            spans: dict[tuple[str, str | None], list[float]] = {}
            for record in records:
                name = record.get(level)
                group_columns = [
                    column for column in columns if column.group == str(record["group"])
                ]
                if name is None or not group_columns or not any(ch.isalpha() for ch in str(name)):
                    continue
                palette = record.get("palette")
                key = (str(name), None if palette is None else str(palette))
                span = spans.setdefault(key, [math.inf, -math.inf])
                span[0] = min(span[0], *(column.xmin for column in group_columns))
                span[1] = max(span[1], *(column.xmax for column in group_columns))
            for (name, palette), (xmin, xmax) in sorted(spans.items()):
                if palette is not None and palette not in palettes:
                    suggestion = did_you_mean(palette, list(palettes))
                    message = (
                        f"funky_heatmap(column_groups=...): palette {palette!r} of group "
                        f"header {name!r} is not defined{suggestion}; add it to `palettes`"
                    )
                    raise SpecError(message, path="column_groups.palette")
                colour = None if palette is None else palettes[palette].middle()
                ymin, ymax = level_boxes[level]
                headers.append((xmin, xmax, ymin, ymax, name, colour, level_match))
        for xmin, xmax, ymin, ymax, name, colour, level_match in headers:
            box = {"xmin": xmin, "xmax": xmax, "ymin": ymin, "ymax": ymax}
            alpha = 1.0 if level_match == 1 else 0.25
            if colour is None:
                pass  # R: palette_mids[NA] is an NA fill, nothing drawn
            elif positions.col_group_radius > 0:
                radius = min(positions.col_group_radius, (xmax - xmin) / 2, (ymax - ymin) / 2)
                shapes.funkyrects.append(
                    {**box, "radius": radius, "fill": colour, "alpha": alpha, "border": False}
                )
            else:
                shapes.rects.append({**box, "fill": colour, "alpha": alpha, "border": False})
            shapes.texts.append(
                {**box, "label": name, "hjust": 0.5, "vjust": 0.5,
                 "fontface": "bold" if level_match == 1 else None,
                 "colour": "white" if level_match == 1 else "black"}
            )  # fmt: skip
        if add_abc:
            first_level = [header for header in headers if header[6] == 1]
            first_level.sort(key=lambda header: (header[0] + header[1]) / 2)
            for index, (xmin, xmax, ymin, ymax, _, _, _) in enumerate(first_level):
                shapes.texts.append(
                    {"xmin": xmin + positions.col_space, "xmax": xmax - positions.col_space,
                     "ymin": ymin, "ymax": ymax, "label": f"{_letter(index)})",
                     "hjust": 0.0, "vjust": 0.5, "fontface": "bold", "colour": "white"}
                )  # fmt: skip

    for column in columns:
        if column.name is None or column.name == "":
            continue
        if names_at_bottom:
            top = table_bottom - 0.1
            shapes.segments.append(
                {"x": column.x, "xend": column.x, "y": table_bottom + 0.3 - 0.6,
                 "yend": table_bottom + 0.5 - 0.6, "size": 0.5}
            )  # fmt: skip
            shapes.texts.append(
                {"xmin": column.xmin, "xmax": column.xmax,
                 "ymin": top - 0.5 - positions.col_annot_offset, "ymax": top - 0.5,
                 "angle": positions.col_annot_angle, "vjust": 0.5, "hjust": 1.0,
                 "label": column.name}
            )  # fmt: skip
        else:
            shapes.segments.append(
                {"x": column.x, "xend": column.x, "y": -0.3, "yend": -0.1, "size": 0.5}
            )
            shapes.texts.append(
                {"xmin": column.xmin, "xmax": column.xmax, "ymin": 0.0,
                 "ymax": positions.col_annot_offset, "angle": positions.col_annot_angle,
                 "vjust": 0.0, "hjust": 0.0, "label": column.name}
            )  # fmt: skip
    return shapes


def _letter(index: int) -> str:
    """R's ``letters[index + 1]`` (NA past z, as R)."""
    return "abcdefghijklmnopqrstuvwxyz"[index] if index < 26 else "NA"


# --------------------------------------------------------------------------- #
# Palettes and legends                                                         #
# --------------------------------------------------------------------------- #


def _palettes(
    palettes: Mapping[str, PaletteValue] | None,
    columns: list[_Column],
    frame: pl.DataFrame,
) -> dict[str, _Palette]:
    """``verify_palettes``: resolve every palette, assigning defaults to missing ones."""
    given: dict[str, PaletteValue] = dict(palettes or {})
    if not isinstance(palettes, Mapping | None):
        message = (
            f"funky_heatmap(palettes=...): expected a dict of palette name -> colors, got "
            f"{type(palettes).__name__}"
        )
        raise SpecError(message, path="palettes")
    rotation = {"numerical": 0, "categorical": 0}
    for column in columns:
        palette_id = column.palette
        if palette_id is None or palette_id in given:
            continue
        if column.geom == "pie":
            kind = "categorical"
        elif frame.schema[column.id].is_numeric():
            kind = "numerical"
        else:
            kind = "categorical"
        names = _NUMERICAL_ROTATION if kind == "numerical" else _CATEGORICAL_ROTATION
        given[palette_id] = names[rotation[kind] % len(names)]
        rotation[kind] += 1
    resolved = {}
    for palette_id, value in given.items():
        resolved[palette_id] = _resolve_palette(palette_id, value, columns, frame)
    return resolved


def _resolve_palette(
    palette_id: str, value: PaletteValue, columns: list[_Column], frame: pl.DataFrame
) -> _Palette:
    """One palette from a default name, a list of colors or a category -> color dict."""
    path = f"palettes[{palette_id!r}]"
    if isinstance(value, str):
        if value in _NUMERICAL_PALETTES:
            return _Palette(tuple(_NUMERICAL_PALETTES[value]))
        if value in _CATEGORICAL_PALETTES:
            categories: list[str] = []
            for column in columns:
                if column.palette != palette_id:
                    continue
                series = frame.get_column(column.id)
                if column.geom == "pie" and isinstance(series.dtype, pl.Struct):
                    names = [field.name for field in series.dtype.fields]
                else:
                    names = [str(item) for item in series.drop_nulls().cast(pl.String).to_list()]
                categories.extend(name for name in names if name not in categories)
            colors = _CATEGORICAL_PALETTES[value]
            if len(categories) > len(colors):
                message = (
                    f"funky_heatmap({path}={value!r}): {len(categories)} categories but the "
                    f"palette has {len(colors)} colors; pick another palette or give a "
                    "dict of category -> color"
                )
                raise SpecError(message, path=path)
            return _Palette(tuple(colors[: len(categories)]), tuple(categories))
        known = [*_NUMERICAL_PALETTES, *_CATEGORICAL_PALETTES]
        suggestion = did_you_mean(value, known)
        message = (
            f"funky_heatmap({path}={value!r}): unknown palette name{suggestion}; accepted: "
            f"{known}, a list of colors, or a dict of category -> color"
        )
        raise SpecError(message, path=path)
    if isinstance(value, Mapping):
        if not value:
            message = f"funky_heatmap({path}): expected at least one color"
            raise SpecError(message, path=path)
        return _Palette(
            tuple(str(color) for color in value.values()), tuple(str(name) for name in value)
        )
    if isinstance(value, Sequence) and len(value) > 0:
        return _Palette(tuple(str(color) for color in value))
    message = (
        f"funky_heatmap({path}={value!r}): expected a palette name, a list of colors or a dict "
        "of category -> color"
    )
    raise SpecError(message, path=path)


def _r_round(value: float) -> int:
    """R's ``round`` (half to even), as an int."""
    return round(value)


def _legend_specs(
    legends: Sequence[Mapping[str, Any]] | None,
    palettes: Mapping[str, _Palette],
    columns: list[_Column],
) -> list[dict[str, Any]]:
    """``verify_legends``: fill each legend's defaults, adding one per unlisted palette."""
    if legends is not None and (
        not isinstance(legends, Sequence) or isinstance(legends, str | bytes)
    ):
        message = (
            f"funky_heatmap(legends=...): expected a list of dicts, got {type(legends).__name__}"
        )
        raise SpecError(message, path="legends")
    specs = [dict(legend) for legend in (legends or [])]
    for index, legend in enumerate(specs):
        if not isinstance(legend, dict):
            message = f"funky_heatmap(legends[{index}]): expected a dict"
            raise SpecError(message, path=f"legends[{index}]")
    used = [column.palette for column in columns if column.palette is not None]
    listed = {legend.get("palette") for legend in specs}
    for palette in dict.fromkeys(used):
        if palette in palettes and palette not in listed:
            specs.append({"title": palette, "palette": palette, "enabled": True})
    result = []
    for index, legend in enumerate(specs):
        path = f"legends[{index}]"
        palette = legend.get("palette")
        if palette is not None and palette not in palettes:
            suggestion = did_you_mean(str(palette), list(palettes))
            message = f"funky_heatmap({path}.palette={palette!r}): unknown palette{suggestion}"
            raise SpecError(message, path=f"{path}.palette")
        if legend.get("enabled", True) is False:
            continue
        if "title" not in legend and palette is not None:
            legend["title"] = palette
        if not isinstance(legend.get("title"), str):
            message = f"funky_heatmap({path}): a legend needs a 'title' (str)"
            raise SpecError(message, path=f"{path}.title")
        if "geom" not in legend and palette is not None:
            geoms = [column.geom for column in columns if column.palette == palette]
            if not geoms:
                message = (
                    f"funky_heatmap({path}): no column uses palette {palette!r}; give the "
                    "legend a 'geom'"
                )
                raise SpecError(message, path=f"{path}.geom")
            legend["geom"] = geoms[0]
        geom = legend.get("geom")
        if geom not in _LEGEND_GEOMS:
            suggestion = did_you_mean(str(geom), _LEGEND_GEOMS)
            message = (
                f"funky_heatmap({path}.geom={geom!r}): expected one of "
                f"{', '.join(repr(value) for value in _LEGEND_GEOMS)}{suggestion}"
            )
            raise SpecError(message, path=f"{path}.geom")
        if "labels" not in legend:
            if geom == "pie" and palette is not None and palettes[palette].names is not None:
                legend["labels"] = list(palettes[palette].names or ())
            elif geom in ("circle", "funkyrect", "rect", "bar"):
                legend["labels"] = ["0", "", "0.2", "", "0.4", "", "0.6", "", "0.8", "", "1"]
            else:
                continue  # text and image legends without labels are disabled
        labels = [str(label) for label in legend["labels"]]
        legend["labels"] = labels
        count = len(labels)
        if count == 0:
            continue
        size = legend.get("size")
        if size is None:
            size = (
                [index / (count - 1) if count > 1 else 0.0 for index in range(count)]
                if geom in ("circle", "funkyrect", "rect", "bar")
                else [3.88]
            )
        legend["size"] = _recycle(size, count, f"{path}.size")
        if "colour" in legend:
            legend["color"] = legend.pop("colour")
        color = legend.get("color")
        if color is None:
            if palette is not None:
                colors = palettes[palette].colors
                if count == 1:
                    picks = [1]
                else:
                    picks = [
                        _r_round(1 + (len(colors) - 1) * index / (count - 1))
                        for index in range(count)
                    ]
                color = [colors[pick - 1] for pick in picks]
            else:
                color = ["black"]
        legend["color"] = _recycle(color, count, f"{path}.color")
        hjust = legend.get("label_hjust", 0.5)
        legend["label_hjust"] = _recycle(hjust, count, f"{path}.label_hjust")
        legend.setdefault("label_width", 2.0 if geom == "pie" else 1.0)
        legend.setdefault("value_width", 2.0)
        if geom in ("text", "image"):
            values = legend.get("values")
            if values is None:
                message = f"funky_heatmap({path}): a {geom} legend needs 'values' (one per label)"
                raise SpecError(message, path=f"{path}.values")
            legend["values"] = _recycle(values, count, f"{path}.values")
        result.append(legend)
    return result


def _recycle(value: object, count: int, path: str) -> list[Any]:
    """Repeat a scalar ``count`` times, or check a list has exactly ``count`` items."""
    if isinstance(value, str) or not isinstance(value, Sequence):
        return [value] * count
    items = list(value)
    if len(items) == 1:
        return items * count
    if len(items) != count:
        message = (
            f"funky_heatmap({path}): expected 1 or {count} values (one per label), got {len(items)}"
        )
        raise SpecError(message, path=path)
    return items


def _legend_shapes(
    legend: Mapping[str, Any], positions: PositionArguments, image_cache: dict[str, _Image]
) -> _Shapes:
    """``create_*_legend``: one legend's shapes, with its title's top-left at (0, 0)."""
    geom = legend["geom"]
    if geom in ("circle", "rect", "funkyrect"):
        return _generic_legend(legend)
    if geom == "bar":
        return _bar_legend(legend)
    if geom == "pie":
        return _pie_legend(legend, positions)
    return _text_legend(legend, positions, image_cache)


def _generic_legend(legend: Mapping[str, Any]) -> _Shapes:
    """``create_generic_geom_legend``: circles, rects or funky rects in a row."""
    shapes = _Shapes()
    space = 0.2
    cursor = 0.0
    boxes = []
    for label, size, color, hjust in zip(
        legend["labels"], legend["size"], legend["color"], legend["label_hjust"], strict=True
    ):
        size = float(size)
        if legend["geom"] == "funkyrect":
            xmin, xmax, ymin, ymax, corner = _funky_rectangle(
                -size / 2, size / 2, -size / 2, size / 2, size
            )
        else:
            xmin, xmax, ymin, ymax, corner = -size / 2, size / 2, -size / 2, size / 2, 0.0
        boxes.append((xmin, xmax, ymin, ymax, corner, label, size, color, hjust))
    placed = []
    for xmin, xmax, ymin, ymax, corner, label, size, color, hjust in boxes:
        width, height = xmax - xmin, ymax - ymin
        left = cursor
        cursor += width + space
        placed.append((left, left + width, -2.5, -2.5 + height, corner, label, size, color, hjust))
    shift = 0.0
    maximum_x = max(box[1] for box in placed)
    shapes.texts.append(
        {"xmin": 0.0, "xmax": maximum_x, "ymin": -1.5, "ymax": -0.5, "label": legend["title"],
         "hjust": 0.0, "vjust": 1.0, "fontface": "bold"}
    )  # fmt: skip
    for xmin, xmax, ymin, ymax, corner, label, size, color, hjust in placed:
        xmin, xmax = xmin + shift, xmax + shift
        x, y = (xmin + xmax) / 2, (ymin + ymax) / 2
        if legend["geom"] == "circle":
            shapes.circles.append({"x": x, "y": y, "r": size / 2, "fill": color})
        elif legend["geom"] == "rect":
            shapes.rects.append(
                {"xmin": xmin, "xmax": xmax, "ymin": ymin, "ymax": ymax, "fill": color,
                 "border": True, "alpha": 1.0}
            )  # fmt: skip
        else:
            shapes.funkyrects.append(
                {"xmin": xmin, "xmax": xmax, "ymin": ymin, "ymax": ymax, "radius": corner,
                 "fill": color}
            )  # fmt: skip
        if label != "":
            shapes.texts.append(
                {"xmin": xmin, "xmax": xmax, "ymin": ymin - 1, "ymax": ymin, "label": label,
                 "hjust": float(hjust), "vjust": 0.0}
            )  # fmt: skip
    return shapes


def _bar_legend(legend: Mapping[str, Any]) -> _Shapes:
    """``create_bar_legend``: a gradient bar with labels below."""
    shapes = _Shapes()
    legend_width, space = 5.0, 0.2
    shapes.texts.append(
        {"xmin": 0.0, "xmax": legend_width, "ymin": -1.5, "ymax": -0.5, "label": legend["title"],
         "hjust": 0.0, "vjust": 1.0, "fontface": "bold", "colour": "black"}
    )  # fmt: skip
    labels = legend["labels"]
    width = legend_width / len(labels) - space
    for index, (label, hjust) in enumerate(zip(labels, legend["label_hjust"], strict=True)):
        xmin = index * (width + space)
        if label.strip():
            shapes.texts.append(
                {"xmin": xmin, "xmax": xmin + width, "ymin": -3.5, "ymax": -2.5, "label": label,
                 "hjust": float(hjust), "vjust": 0.0, "colour": "black"}
            )  # fmt: skip
    given = [str(color) for color in legend["color"]]
    if all(_is_color(color[:7]) and len(color) in (7, 9) for color in given):
        colors = (
            _smear([color[:7].upper() for color in given], 500) if len(given) > 1 else given * 500
        )
    else:  # named colors cannot be interpolated here: equal blocks instead
        colors = [given[index * len(given) // 500] for index in range(500)]
    step = legend_width / 500
    for index, color in enumerate(colors):
        xmin = legend_width * index / 499
        shapes.rects.append(
            {"xmin": xmin, "xmax": xmin + step, "ymin": -2.5, "ymax": -1.5, "fill": color,
             "border": False, "alpha": 1.0}
        )  # fmt: skip
    shapes.rects.append(
        {"xmin": 0.0, "xmax": legend_width, "ymin": -2.5, "ymax": -1.5, "fill": None,
         "border": True, "alpha": 1.0}
    )  # fmt: skip
    return shapes


def _pie_legend(legend: Mapping[str, Any], positions: PositionArguments) -> _Shapes:
    """``create_pie_legend``: a half pie with one label per slice."""
    shapes = _Shapes()
    row_height = positions.row_height
    labels = legend["labels"]
    count = len(labels)
    label_width = float(legend["label_width"])
    shapes.texts.append(
        {"xmin": 0.0, "xmax": label_width, "ymin": -1 - row_height, "ymax": -1.0,
         "label": legend["title"], "hjust": 0.0, "vjust": 1.0, "fontface": "bold",
         "colour": "black"}
    )  # fmt: skip
    centres = [(index + 0.5) * math.pi / count for index in range(count)]
    first, last = centres[0], centres[-1]
    for index, (label, color) in enumerate(zip(labels, legend["color"], strict=True)):
        start, end = index * math.pi / count, (index + 1) * math.pi / count
        angle = centres[index]
        shapes.pies.append(
            {"x0": 0.0, "y0": -2.75, "r": row_height * 0.75, "start": start, "end": end,
             "fill": color, "xmin": -row_height * 0.75, "xmax": row_height * 0.75,
             "ymin": -2.75 - row_height * 0.75, "ymax": -2.75 + row_height * 0.75}
        )  # fmt: skip
        xpt, ypt = row_height * math.sin(angle), row_height * math.cos(angle)
        shapes.segments.append(
            {"x": xpt * 0.85, "xend": xpt * 1.1, "y": -2.75 + ypt * 0.85,
             "yend": -2.75 + ypt * 1.1, "size": 0.5}
        )  # fmt: skip
        high = row_height * (math.cos(first) + 0.2)
        low = row_height * (math.cos(last) - 0.2)
        lab_y = high + (low - high) * (index / (count - 1) if count > 1 else 0.0)
        x = 0.5 + row_height * math.sin(angle)
        y = -2.75 + lab_y
        shapes.texts.append(
            {"xmin": x, "xmax": x + label_width, "ymin": y - row_height / 2,
             "ymax": y + row_height / 2, "label": label, "hjust": 0.0, "vjust": 0.5,
             "colour": "black"}
        )  # fmt: skip
    return shapes


def _text_legend(
    legend: Mapping[str, Any], positions: PositionArguments, image_cache: dict[str, _Image]
) -> _Shapes:
    """``create_text_legend`` / ``create_image_legend``: label -> value rows."""
    shapes = _Shapes()
    row_height = positions.row_height
    label_width = float(legend["label_width"])
    value_width = float(legend["value_width"])
    xwidth = 2 * 0.5 + label_width + value_width
    shapes.texts.append(
        {"xmin": 0.0, "xmax": xwidth, "ymin": -1 - row_height, "ymax": -1.0,
         "label": legend["title"], "hjust": 0.0, "vjust": 1.0, "fontface": "bold",
         "colour": "black"}
    )  # fmt: skip
    for index, (label, value, color) in enumerate(
        zip(legend["labels"], legend["values"], legend["color"], strict=True)
    ):
        y = -2 - row_height * index
        if legend["geom"] == "image":
            width = 2 * 0.5 + label_width
            image = _read_png(str(label), f"legends[{legend['title']!r}].labels", image_cache)
            shapes.images.append(
                {"xmin": 0.0, "xmax": width, "ymin": y - row_height / 2,
                 "ymax": y + row_height / 2, "image": image}
            )  # fmt: skip
        else:
            shapes.texts.append(
                {"xmin": 0.5, "xmax": 0.5 + xwidth, "ymin": y - row_height / 2,
                 "ymax": y + row_height / 2, "label": str(label), "hjust": 0.0, "vjust": 0.5,
                 "colour": str(color), "size": legend.get("text_size")}
            )  # fmt: skip
        x = 2 * 0.5 + label_width
        shapes.texts.append(
            {"xmin": x, "xmax": x + xwidth, "ymin": y - row_height / 2, "ymax": y + row_height / 2,
             "label": str(value), "hjust": 0.0, "vjust": 0.5, "colour": str(color)}
        )  # fmt: skip
    return shapes


def _place_legends(
    main: tuple[float, float, float, float],
    legend_shapes: list[_Shapes],
    position: str,
) -> _Shapes:
    """Lay the legends out below the table (in a row) or to its right (in a column)."""
    placed = _Shapes()
    gap = 1.0
    main_xmin, main_xmax, main_ymin, main_ymax = main
    cursor_x = main_xmin
    cursor_y = main_ymin - 0.1 * 4  # funkyheatmap's 0.1 inch spacer, in data units
    if position == "right":
        cursor_x = main_xmax
        cursor_y = main_ymax
    for shapes in legend_shapes:
        bounds = shapes.bounds()
        if bounds is None:
            continue
        xmin, xmax, ymin, ymax = bounds
        if position == "right":
            placed.extend(shapes.shift(cursor_x - xmin, cursor_y - ymax))
            cursor_y -= (ymax - ymin) + gap
        else:
            placed.extend(shapes.shift(cursor_x - xmin, cursor_y - ymax))
            cursor_x += (xmax - xmin) + gap
    return placed


# --------------------------------------------------------------------------- #
# Layers                                                                       #
# --------------------------------------------------------------------------- #


def _text_size(size_mm: float) -> float:
    """Convert a ggplot2 text size (mm) to a ggrust text size (font px = 2 x size)."""
    return size_mm * _PT * 96 / 72 / 2 * (_UNIT_PX / 24.0)


def _line_size(width_mm: float) -> float:
    """Convert a ggplot2 line width (mm) to a ggrust line size (px = 2.2 x size)."""
    return width_mm * _PT * 96 / 72 / 2.2 * (_UNIT_PX / 24.0)


def _rounded_rectangle(
    xmin: float, xmax: float, ymin: float, ymax: float, radius: float
) -> list[tuple[float, float]]:
    """Polygon vertices of a rectangle with circular corners of ``radius``."""
    radius = max(0.0, min(radius, (xmax - xmin) / 2, (ymax - ymin) / 2))
    if radius == 0:
        return [(xmin, ymin), (xmax, ymin), (xmax, ymax), (xmin, ymax)]
    corners = [
        (xmax - radius, ymin + radius, -math.pi / 2),
        (xmax - radius, ymax - radius, 0.0),
        (xmin + radius, ymax - radius, math.pi / 2),
        (xmin + radius, ymin + radius, math.pi),
    ]
    points = []
    for cx, cy, start in corners:
        for step in range(_CORNER_VERTICES + 1):
            angle = start + step / _CORNER_VERTICES * math.pi / 2
            points.append((cx + radius * math.cos(angle), cy + radius * math.sin(angle)))
    return points


def _layers(shapes: _Shapes) -> list[LayerSpec]:
    """Turn the shapes into layers, in funkyheatmap's drawing order."""
    layers: list[LayerSpec] = []
    if shapes.backgrounds:
        frame = pl.DataFrame(
            shapes.backgrounds, schema=["xmin", "xmax", "ymin", "ymax"], orient="row"
        )
        layers.append(
            geom_rect(aes(xmin="xmin", xmax="xmax", ymin="ymin", ymax="ymax"), data=frame,
                      fill=_ROW_BACKGROUND, color=None, size=0)
        )  # fmt: skip
    layers += _segment_layers(shapes.segments)
    layers += _rect_layers(shapes.rects)
    polygons: list[tuple[list[tuple[float, float]], str | None, float, bool]] = []
    for circle in shapes.circles:
        points = [
            (
                circle["x"] + circle["r"] * math.cos(2 * math.pi * step / _CIRCLE_VERTICES),
                circle["y"] + circle["r"] * math.sin(2 * math.pi * step / _CIRCLE_VERTICES),
            )
            for step in range(_CIRCLE_VERTICES)
        ]
        polygons.append((points, circle["fill"], 1.0, True))
    layers += _polygon_layers(polygons)
    polygons = []
    for rect in shapes.funkyrects:
        points = _rounded_rectangle(
            rect["xmin"], rect["xmax"], rect["ymin"], rect["ymax"], rect["radius"]
        )
        polygons.append((points, rect["fill"], rect.get("alpha", 1.0), rect.get("border", True)))
    layers += _polygon_layers(polygons)
    polygons = []
    for pie in shapes.pies:
        steps = max(2, math.ceil((pie["end"] - pie["start"]) / (2 * math.pi) * _CIRCLE_VERTICES))
        # ggforce arcs start at 12 o'clock and run clockwise.
        # A full slice is a circle (funkyheatmap draws 100% pies with geom_circle).
        full = pie["end"] - pie["start"] >= 2 * math.pi * (1 - 1e-10)
        centre = [] if full else [(pie["x0"], pie["y0"])]
        points = centre + [
            (
                pie["x0"]
                + pie["r"] * math.sin(pie["start"] + (pie["end"] - pie["start"]) * step / steps),
                pie["y0"]
                + pie["r"] * math.cos(pie["start"] + (pie["end"] - pie["start"]) * step / steps),
            )
            for step in range(steps + 1)
        ]
        polygons.append((points, pie["fill"], 1.0, True))
    layers += _polygon_layers(polygons)
    for image in shapes.images:
        layers.append(_image_layer(image))
    layers += _text_layers(shapes.texts)
    return layers


def _segment_layers(segments: list[dict[str, Any]]) -> list[LayerSpec]:
    """Segments, one layer per (color, width, line type)."""
    styles: dict[tuple[str, float, str], list[dict[str, Any]]] = {}
    for segment in segments:
        key = (segment.get("colour") or "black", segment.get("size") or 0.5,
               segment.get("linetype") or "solid")  # fmt: skip
        styles.setdefault(key, []).append(segment)
    layers = []
    for (colour, size, linetype), records in styles.items():
        frame = pl.DataFrame(
            {
                name: [float(record[name]) for record in records]
                for name in ("x", "xend", "y", "yend")
            }
        )
        layers.append(
            geom_segment(aes(x="x", y="y", xend="xend", yend="yend"), data=frame,
                         color=colour, size=_line_size(size), linetype=linetype)
        )  # fmt: skip
    return layers


def _rect_layers(rects: list[dict[str, Any]]) -> list[LayerSpec]:
    """Rectangles (cells, bars, headers), one layer per (border, alpha) in draw order."""
    layers = []
    batch: list[dict[str, Any]] = []
    key: tuple[bool, float] | None = None

    def flush() -> None:
        if not batch or key is None:
            return
        border, alpha = key
        frame = pl.DataFrame(
            {
                **{
                    name: [float(record[name]) for record in batch]
                    for name in ("xmin", "xmax", "ymin", "ymax")
                },
                "fill": [record["fill"] or "#00000000" for record in batch],
            }
        )
        layers.append(
            geom_rect(aes(xmin="xmin", xmax="xmax", ymin="ymin", ymax="ymax", fill="fill"),
                      data=frame, color="black" if border else None,
                      size=_line_size(0.25) if border else 0, alpha=_alpha(alpha))
        )  # fmt: skip

    for rect in rects:
        record_key = (bool(rect.get("border", True)), float(rect.get("alpha", 1.0)))
        if record_key != key:
            flush()
            batch = []
            key = record_key
        batch.append(rect)
    flush()
    return layers


def _alpha(alpha: float) -> float | None:
    """Return ``alpha`` only when translucent, else None (no ``alpha=`` in the spec).

    A constant alpha replaces the fill's own alpha (lets-plot), which would make the
    transparent ``#00000000`` fill opaque black.
    """
    return None if alpha >= 1 else alpha


def _polygon_layers(
    polygons: list[tuple[list[tuple[float, float]], str | None, float, bool]],
) -> list[LayerSpec]:
    """Draw filled polygons with a thin black outline, one layer per (alpha, border)."""
    styles: dict[tuple[float, bool], list[tuple[list[tuple[float, float]], str | None]]] = {}
    for points, fill, alpha, border in polygons:
        styles.setdefault((alpha, border), []).append((points, fill))
    layers = []
    for (alpha, border), items in styles.items():
        xs: list[float] = []
        ys: list[float] = []
        groups: list[int] = []
        fills: list[str] = []
        for group, (points, fill) in enumerate(items):
            for x, y in points:
                xs.append(x)
                ys.append(y)
                groups.append(group)
                fills.append(fill or "#00000000")
        frame = pl.DataFrame({"x": xs, "y": ys, "group": groups, "fill": fills})
        layers.append(
            geom_polygon(aes(x="x", y="y", group="group", fill="fill"), data=frame,
                         color="black" if border else None,
                         size=_line_size(0.25) if border else 0, alpha=_alpha(alpha))
        )  # fmt: skip
    return layers


def _image_layer(record: Mapping[str, Any]) -> LayerSpec:
    """Draw a PNG inside its box, keeping its aspect ratio and centred (cowplot)."""
    image: _Image = record["image"]
    xmin, xmax, ymin, ymax = record["xmin"], record["xmax"], record["ymin"], record["ymax"]
    box_width, box_height = xmax - xmin, ymax - ymin
    scale = min(box_width / max(image.width, 1), box_height / max(image.height, 1))
    width, height = image.width * scale, image.height * scale
    x0 = xmin + (box_width - width) / 2
    y0 = ymin + (box_height - height) / 2
    return _geom(
        "image",
        href=image.href,
        xmin=x0,
        xmax=x0 + width,
        ymin=y0,
        ymax=y0 + height,
        show_legend=False,
        inherit_aes=False,
    )


def _text_anchor(text: Mapping[str, Any]) -> tuple[float, float, float, float, float, float, str]:
    """Place a text in its box by funkyheatmap's rule.

    Returns (x, y, hjust, vjust, size in mm, angle, fontface).
    """
    hjust = 0.5 if text.get("hjust") is None else float(text["hjust"])
    vjust = 0.5 if text.get("vjust") is None else float(text["vjust"])
    size = 4.0 if text.get("size") is None else float(text["size"])
    angle = 0.0 if text.get("angle") is None else float(text["angle"])
    fontface = str(text.get("fontface") or "plain")
    radians = angle / 360 * 2 * math.pi
    cosa = round(math.cos(radians), 2)
    sina = round(math.sin(radians), 2)
    alphax = (1 - hjust if cosa < 0 else hjust) * abs(cosa) + (
        1 - vjust if sina > 0 else vjust
    ) * abs(sina)
    alphay = (1 - hjust if sina < 0 else hjust) * abs(sina) + (
        1 - vjust if cosa < 0 else vjust
    ) * abs(cosa)
    x = (1 - alphax) * text["xmin"] + alphax * text["xmax"]
    y = (1 - alphay) * text["ymin"] + alphay * text["ymax"]
    return x, y, hjust, vjust, size, angle, fontface


def _text_corners(text: Mapping[str, Any]) -> list[tuple[float, float]]:
    """Corners of a text's drawn extent in data units (measured width, line height)."""
    x, y, hjust, vjust, size, angle, fontface = _text_anchor(text)
    font_px = _text_size(size) * 2
    lines = str(text["label"]).split("\n")
    width = max(text_width(line, font_px, fontface) for line in lines) / _UNIT_PX
    em = font_px / _UNIT_PX
    extra = (len(lines) - 1) * em  # lines are one font size apart; pad both ways
    # lets-plot: vjust moves the baseline 0.7 em; glyphs reach ~0.3 em below it and
    # ~0.95 em above it.
    baseline = -vjust * 0.7 * em
    local = [
        (-hjust * width, baseline - 0.3 * em - extra),
        ((1 - hjust) * width, baseline - 0.3 * em - extra),
        ((1 - hjust) * width, baseline + 0.95 * em + extra),
        (-hjust * width, baseline + 0.95 * em + extra),
    ]
    radians = math.radians(angle)
    cos, sin = math.cos(radians), math.sin(radians)
    return [(x + dx * cos - dy * sin, y + dx * sin + dy * cos) for dx, dy in local]


def _text_layers(texts: list[dict[str, Any]]) -> list[LayerSpec]:
    """Text placed in its box by funkyheatmap's rule; one layer per text style."""
    styles: dict[tuple[float, float, float, float, str], list[dict[str, Any]]] = {}
    for text in texts:
        x, y, hjust, vjust, size, angle, fontface = _text_anchor(text)
        key = (hjust, vjust, size, angle, fontface)
        styles.setdefault(key, []).append(
            {"x": x, "y": y, "label": text["label"], "colour": text.get("colour") or "black"}
        )
    layers = []
    for (hjust, vjust, size, angle, fontface), records in styles.items():
        frame = pl.DataFrame(
            {
                "x": [record["x"] for record in records],
                "y": [record["y"] for record in records],
                "label": [record["label"] for record in records],
                "colour": [record["colour"] for record in records],
            }
        )
        layers.append(
            geom_text(aes(x="x", y="y", label="label", color="colour"), data=frame,
                      hjust=hjust, vjust=vjust, size=_text_size(size), angle=angle,
                      fontface=fontface)
        )  # fmt: skip
    return layers


# --------------------------------------------------------------------------- #
# Entry point                                                                  #
# --------------------------------------------------------------------------- #


def _frame(data: object) -> pl.DataFrame:
    """Read ``data`` as a polars DataFrame with a unique ``id`` column."""
    if isinstance(data, pl.DataFrame):
        frame = data
    elif isinstance(data, Mapping):
        try:
            frame = pl.DataFrame(dict(data))
        except Exception as error:  # ragged columns raise several polars types
            message = f"funky_heatmap(data=...): could not read the dict of columns: {error}"
            raise SpecError(message, path="data") from error
    elif type(data).__module__.startswith("pandas"):
        converted = pl.from_pandas(data)
        if not isinstance(converted, pl.DataFrame):
            message = "funky_heatmap(data=...): expected a DataFrame, got a Series"
            raise SpecError(message, path="data")
        frame = converted
    else:
        message = (
            f"funky_heatmap(data=...): expected a polars DataFrame, a pandas DataFrame or a dict "
            f"of columns, got {type(data).__name__}"
        )
        raise SpecError(message, path="data")
    if frame.height == 0 or frame.width == 0:
        message = "funky_heatmap(data=...): expected at least one row and one column"
        raise SpecError(message, path="data")
    if "id" not in frame.columns:
        message = (
            "funky_heatmap(data=...): `data` needs an 'id' column naming each row; add one with "
            "frame.with_row_index('id') or frame.with_columns(id=...)"
        )
        raise ColumnNotFoundError(message, column="id", available=list(frame.columns), path="data")
    ids = frame.get_column("id").cast(pl.String)
    if ids.null_count() > 0 or ids.n_unique() != frame.height:
        duplicated = ids.filter(ids.is_duplicated() | ids.is_null()).unique().to_list()[:5]
        message = (
            f"funky_heatmap(data=...): the 'id' column must be unique and non-null; "
            f"repeated or missing: {duplicated}"
        )
        raise SpecError(message, path="data.id")
    return frame


def funky_heatmap(
    data: pl.DataFrame | Mapping[str, Sequence[Any]],
    column_info: TableLike | None = None,
    row_info: TableLike | None = None,
    column_groups: TableLike | None = None,
    row_groups: TableLike | None = None,
    palettes: Mapping[str, PaletteValue] | None = None,
    legends: Sequence[Mapping[str, Any]] | None = None,
    position_args: PositionArguments | None = None,
    scale_column: bool = True,
    add_abc: bool = True,
) -> PlotSpec:
    """Draw a funky heatmap: a table of bars, circles, rounded squares, pies, text and images.

    Column groups, row groups and legends are drawn around the cells.

    Follows the R package funkyheatmap: the same arguments, defaults and layout. The
    result is an ordinary plot built from rect, polygon, segment, text and image
    layers on a ``coord_fixed`` canvas (one unit is 0.25 inch), so ``+`` adds layers in
    the same coordinates: row ``i`` (1-based) of an ungrouped table is centred at
    ``y = -i * (row_height + row_space)``.

    Parameters
    ----------
    data : polars.DataFrame or dict of columns
        One row per table row, with a unique ``id`` column. Pie cells are struct (dict)
        values of category -> amount; image cells are PNG file paths or URLs.
    column_info : DataFrame or list of dict, optional
        One entry per column to draw, in order. Keys: ``id`` (data column, required),
        ``name`` (header text; None for none), ``geom`` (``"funkyrect"``, ``"circle"``,
        ``"rect"``, ``"bar"``, ``"pie"``, ``"text"`` or ``"image"``), ``group``,
        ``palette``, ``width``, ``overlay`` (draw over the previous column), ``legend``,
        ``draw_outline`` (dashed guides around bars), ``id_color`` and ``id_size``
        (columns giving the color and the size, default ``id``), and ``options``: a dict
        merged into the entry (``hjust``, ``vjust``, ``size``, ``fontface``, ``label``,
        ``scale``, ``directory``, ``extension``). A key left out of an entry takes
        funkyheatmap's default; None is R's NA (``"name": None`` draws no name).
        Default: every data column, geoms guessed from the types.
    row_info : DataFrame or list of dict, optional
        Rows to draw, in order: ``id`` and an optional ``group``. Default: every row.
    column_groups : DataFrame or list of dict, optional
        One entry per column group: ``group``, ``palette`` (its header color is the
        palette's middle color) and one or more level keys with the header texts. The
        first key's level is drawn solid with white bold text, others translucent.
    row_groups : DataFrame or list of dict, optional
        One entry per row group: ``group`` and level keys with the group titles.
    palettes : dict, optional
        Palette name -> a default palette name (``"Blues"``, ``"Reds"``, ``"YlOrBr"``,
        ``"Greens"``, ``"Greys"``, ``"Set1"``, ``"Set2"``, ``"Set3"``, ``"Dark2"``), a
        list of colors (numbers in 0..1 pick along it), or a dict of category -> color.
        Missing palettes get defaults in turn.
    legends : list of dict, optional
        Legends to draw: ``title``, ``geom``, ``palette``, ``labels``, ``size``,
        ``color``, ``values`` (text and image legends), ``label_hjust``,
        ``label_width``, ``enabled``. Every used palette without a legend gets one.
    position_args : PositionArguments, optional
        Spacing and placement from ``position_arguments()``.
    scale_column : bool, default=True
        Min-max scale each numeric column to 0..1 (per column option ``scale``).
    add_abc : bool, default=True
        Prefix the first-level column group headers with ``a)``, ``b)``, ...

    Returns
    -------
    PlotSpec
        The heatmap, sized to its content.

    Raises
    ------
    ColumnNotFoundError
        When ``data`` has no ``id`` column, or ``column_info`` names a missing column.
    SpecError
        For an unknown geom or palette, an undescribed group, a non-PNG image, or a
        malformed argument; the message names the argument and the accepted values.

    Examples
    --------
    >>> import polars as pl
    >>> from ggrust import *
    >>> from ggrust.bistro import funky_heatmap
    >>> scores = pl.DataFrame(
    ...     {
    ...         "id": ["Alpha", "Beta", "Gamma", "Delta"],
    ...         "speed": [0.9, 0.6, 0.4, 0.1],
    ...         "accuracy": [0.7, 0.8, 0.3, 0.5],
    ...         "memory": [0.2, 0.5, 0.9, 0.7],
    ...     }
    ... )
    >>> column_info = [
    ...     {"id": "id", "name": "Method", "geom": "text", "group": "method",
    ...      "options": {"hjust": 0, "width": 3}},
    ...     {"id": "speed", "name": "Speed", "geom": "bar", "group": "scores",
    ...      "palette": "scores", "options": {"width": 3}},
    ...     {"id": "accuracy", "name": "Accuracy", "geom": "funkyrect", "group": "scores",
    ...      "palette": "scores"},
    ...     {"id": "memory", "name": "Memory", "geom": "circle", "group": "scores",
    ...      "palette": "scores"},
    ... ]
    >>> column_groups = [
    ...     {"Category": "Method", "group": "method", "palette": "black"},
    ...     {"Category": "Scores", "group": "scores", "palette": "scores"},
    ... ]
    >>> plot = funky_heatmap(
    ...     scores,
    ...     column_info=column_info,
    ...     column_groups=column_groups,
    ...     palettes={"scores": "Blues", "black": ["black", "black"]},
    ... )
    """
    positions = position_args if position_args is not None else position_arguments()
    if not isinstance(positions, PositionArguments):
        message = (
            f"funky_heatmap(position_args=...): expected position_arguments(...), got "
            f"{type(positions).__name__}"
        )
        raise SpecError(message, path="position_args")
    frame = _frame(data)
    columns = _columns(column_info, frame)
    ids = frame.get_column("id").cast(pl.String).to_list()
    rows = _rows(row_info, frame, ids)
    column_group_info = _groups(
        column_groups, [column.group for column in columns], "column_groups"
    )
    if column_group_info is None:
        for column in columns:
            column.group = None
    row_group_info = _groups(row_groups, [row.group for row in rows], "row_groups")
    if row_group_info is None:
        for row in rows:
            row.group = None
    resolved_palettes = _palettes(palettes, columns, frame)
    legend_specs = _legend_specs(legends, resolved_palettes, columns)

    _row_positions(rows, positions)
    _column_positions(columns, positions)
    image_cache: dict[str, _Image] = {}
    shapes = _annotation_shapes(
        columns, rows, column_group_info, row_group_info, resolved_palettes, positions, add_abc
    )
    shapes.extend(
        _table_shapes(columns, rows, frame, resolved_palettes, positions, scale_column, image_cache)
    )
    bounds = shapes.bounds()
    if bounds is None:  # pragma: no cover - rows always give bounds
        bounds = (0.0, 1.0, -1.0, 0.0)
    xmin, xmax, ymin, ymax = bounds
    main = (
        xmin - positions.expand_xmin,
        xmax + positions.expand_xmax,
        ymin - positions.expand_ymin,
        ymax + positions.expand_ymax,
    )
    if positions.legend_position != "none" and legend_specs:
        legend_shapes = [_legend_shapes(legend, positions, image_cache) for legend in legend_specs]
        shapes.extend(_place_legends(main, legend_shapes, positions.legend_position))
    every = shapes.bounds() or main
    padding = _CANVAS_PADDING
    xlim = (min(main[0], every[0]) - padding, max(main[1], every[1]) + padding)
    ylim = (min(main[2], every[2]) - padding, max(main[3], every[3]) + padding)

    plot = ggplot()
    for layer in _layers(shapes):
        plot += layer
    width_px = (xlim[1] - xlim[0]) * _UNIT_PX
    height_px = (ylim[1] - ylim[0]) * _UNIT_PX
    if width_px > _MAX_SIZE_PX or height_px > _MAX_SIZE_PX:
        message = (
            f"funky_heatmap(): the table needs {round(width_px)} x {round(height_px)} px "
            f"({len(rows)} rows, {len(columns)} columns) but plots are at most "
            f"{_MAX_SIZE_PX} px; show fewer rows with row_info or narrow the columns"
        )
        raise SpecError(message, path="data")
    return (
        plot
        + scale_fill_identity()
        + scale_color_identity()
        + coord_fixed(ratio=1, xlim=xlim, ylim=ylim)
        + theme_void()
        + theme(plot_margin=0, plot_inset=0, legend_position="none")
        + ggsize(round(width_px), round(height_px))
    )
