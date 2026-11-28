"""Prototype: the pages the terminal plot pane shows, and how text pages are laid out.

The pane's history holds pages: a plot, a data table (``show(frame)``) or a text
page (``help(geom_point)``). This module lays out tables and text as lines of
terminal cells, with colors, cut to the pane's width. It does no terminal I/O.
Not part of the public API: a prototype on its own branch.
"""

from __future__ import annotations

import inspect
import re
import unicodedata
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Literal

if TYPE_CHECKING:
    from collections.abc import Iterator

    import polars as pl

    from ggrust.plot.core import PlotSpec
    from ggrust.plot.subplots import SupPlotsSpec

PageKind = Literal["plot", "table", "text"]

# SGR codes for the few styles pages use.
BOLD = "1"
DIM = "2"
ACCENT = "36"  # cyan: code and signatures
PLAIN = ""

Segment = tuple[str, str]  # (SGR code, text)

_MAX_COLUMN_CELLS = 30  # widest a table column gets before its values are cut
_COLUMN_GAP = 2


@dataclass
class Page:
    """One entry of the pane's history.

    Attributes
    ----------
    kind : {"plot", "table", "text"}
        What the page holds.
    title : str
        Shown in the frame's top edge (already cleaned of control characters).
    plot : PlotSpec or SupPlotsSpec or None
        The plot, for ``kind="plot"``.
    frame : polars.DataFrame or None
        The rows the pane has, for ``kind="table"`` (the first rows of a larger frame).
    total_rows : int
        Rows in the whole frame, for ``kind="table"``.
    text : str
        The text, for ``kind="text"``.
    top : int
        First row (table) or line (text) on screen.
    left : int
        First column on screen, for tables.
    """

    kind: PageKind
    title: str
    plot: PlotSpec | SupPlotsSpec | None = None
    frame: pl.DataFrame | None = None
    total_rows: int = 0
    text: str = ""
    top: int = 0
    left: int = 0
    widths: dict[int, int] = field(default_factory=dict)  # table column widths, as drawn
    failed: bool = False  # a plot that could not be drawn


def _clean_label(text: str) -> str:
    """``text`` on one line with control characters removed, so it is safe to print.

    Runs of whitespace become one space; C0, DEL and C1 characters (ESC among them)
    and format characters (the right-to-left override among them) are dropped, so
    a title cannot send escape sequences to the terminal or reorder the text.
    """
    words = " ".join(text.split())
    return "".join(
        character for character in words if unicodedata.category(character) not in ("Cc", "Cf")
    )


def _clean_text(text: str) -> str:
    """``text`` with control characters removed but line breaks kept (tabs become spaces)."""
    lines = text.expandtabs(4).splitlines()
    return "\n".join(
        "".join(c for c in line if unicodedata.category(c) not in ("Cc", "Cf")) for line in lines
    )


def _cell_width(character: str) -> int:
    """Terminal cells ``character`` takes alone: 2 for wide, 0 for combining and control."""
    if unicodedata.combining(character) or unicodedata.category(character) in (
        "Mn",
        "Me",
        "Cf",
        "Cc",
    ):
        return 0
    return 2 if unicodedata.east_asian_width(character) in ("W", "F") else 1


_ZERO_WIDTH_JOINER = "\u200d"
_EMOJI_PRESENTATION = "\ufe0f"  # VS16: the character before it is drawn as a 2-cell emoji


def _is_skin_tone(character: str) -> bool:
    return 0x1F3FB <= ord(character) <= 0x1F3FF


def _is_regional_indicator(character: str) -> bool:
    return 0x1F1E6 <= ord(character) <= 0x1F1FF


def _widths(text: str) -> Iterator[int]:
    """Yield the cells each character of ``text`` adds, as a terminal draws emoji sequences.

    VS16 widens a 1-cell character before it to 2 (``❤️``); a character joined by a
    ZWJ, or a skin tone after an emoji, adds nothing (``👩‍💻``, ``👍🏽``); a pair of
    regional indicators is one 2-cell flag, counted on its first half so a cut never
    splits it.
    """
    previous = 0  # cells of the character before, once VS16 is applied
    joined = False  # the character before was a ZWJ
    open_flag = False  # the character before started a pair of regional indicators
    for character in text:
        if character == _EMOJI_PRESENTATION:
            width = 1 if previous == 1 else 0
            previous = 2 if previous else 0
        elif _is_regional_indicator(character):
            width = 0 if open_flag else 2
            open_flag = not open_flag
            previous = 2
        else:
            joins = joined or (_is_skin_tone(character) and previous == 2)
            width = 0 if joins else _cell_width(character)
            previous = width or previous
            open_flag = False
        joined = character == _ZERO_WIDTH_JOINER
        yield width


def _cells(text: str) -> int:
    """Terminal cells ``text`` takes."""
    return sum(_widths(text))


def _cut(text: str, cells: int) -> str:
    """Return the longest start of ``text`` that fits in ``cells`` terminal cells."""
    used = 0
    for index, width in enumerate(_widths(text)):
        used += width
        if used > cells:
            return text[:index]
    return text


def _fit(text: str, cells: int, *, right: bool = False) -> str:
    """``text`` cut (with an ellipsis) or padded with spaces to exactly ``cells`` cells."""
    if _cells(text) > cells:
        text = _cut(text, cells - 1) + "…" if cells >= 1 else ""
    padding = " " * (cells - _cells(text))
    return padding + text if right else text + padding


def render_line(segments: list[Segment], cells: int) -> str:
    """Join styled ``segments`` into one line that takes at most ``cells`` terminal cells.

    Parameters
    ----------
    segments : list of (str, str)
        ``(sgr_code, text)`` pairs; an empty code is plain text. The text must hold
        no control characters.
    cells : int
        The width available.

    Returns
    -------
    str
        The line with SGR escapes, cut at ``cells``.
    """
    parts: list[str] = []
    left = cells
    for code, text in segments:
        if left <= 0:
            break
        piece = _cut(text, left)
        left -= _cells(piece)
        parts.append(f"\x1b[{code}m{piece}\x1b[0m" if code and piece else piece)
    return "".join(parts)


# --- tables ---------------------------------------------------------------------


def _format_value(value: object) -> str:
    """One table cell's text: floats to 6 significant digits, nulls as ``null``."""
    if value is None:
        return "null"
    if isinstance(value, float):
        text = f"{value:.6g}"
        return text if any(c in text for c in ".ein") else text + ".0"  # 2 -> 2.0, as polars
    return _clean_label(str(value))


def _dtype_name(dtype: object) -> str:
    """Return a column type as polars prints it in table headers (``f64``, ``str``)."""
    short = getattr(dtype, "_string_repr", None)
    try:
        return str(short()) if callable(short) else str(dtype)
    except Exception:  # a private polars method: fall back to the long name
        return str(dtype)


def _column_width(page: Page, column: int) -> int:
    """Cells column ``column`` needs for its name, its type and its values (capped).

    Computed the first time the column is drawn and kept in ``page.widths``, so a wide
    frame costs only the columns that reach the screen. Values are measured on their
    first ``_MAX_COLUMN_CELLS + 1`` characters, which is all the cap can use.
    """
    if column not in page.widths:
        assert page.frame is not None
        series = page.frame.to_series(column)
        limit = _MAX_COLUMN_CELLS + 1
        needed = max(
            _cells(_clean_label(series.name)[:limit]),
            _cells(_dtype_name(series.dtype)),
            *(_cells(_format_value(value)[:limit]) for value in series.to_list()),
        )
        page.widths[column] = min(needed, _MAX_COLUMN_CELLS)
    return page.widths[column]


def visible_columns(page: Page, cells: int) -> range:
    """Return the columns of a table page that fit in ``cells``, starting at ``page.left``."""
    assert page.frame is not None
    index_cells = len(str(page.total_rows)) + _COLUMN_GAP
    used = index_cells
    end = page.left
    while end < page.frame.width:
        used += _column_width(page, end) + (_COLUMN_GAP if end > page.left else 0)
        if used > cells and end > page.left:
            break
        end += 1
    return range(page.left, end)


def table_lines(page: Page, rows: int, cells: int) -> list[str]:
    """Lay out a table page: header, types, a rule, then the rows from ``page.top``.

    Parameters
    ----------
    page : Page
        A ``kind="table"`` page.
    rows : int
        Lines available.
    cells : int
        Width available.

    Returns
    -------
    list of str
        At most ``rows`` lines, each at most ``cells`` cells, with SGR escapes.
    """
    assert page.frame is not None
    frame = page.frame
    columns = visible_columns(page, cells)
    index_cells = len(str(page.total_rows))
    names = [frame.columns[i] for i in columns]
    numeric = [frame.get_column(name).dtype.is_numeric() for name in names]
    gap = " " * _COLUMN_GAP

    def line(index_text: str, texts: list[str], code: str) -> str:
        segments: list[Segment] = [(DIM, _fit(index_text, index_cells, right=True) + gap)]
        for position, (column, text) in enumerate(zip(columns, texts, strict=True)):
            if position:
                segments.append((PLAIN, gap))
            segments.append((code, _fit(text, page.widths[column], right=numeric[position])))
        return render_line(segments, cells)

    lines = [
        line("", [_clean_label(name) for name in names], BOLD),
        line("", [_dtype_name(frame.get_column(name).dtype) for name in names], DIM),
        render_line([(DIM, "─" * cells)], cells),
    ]
    body_rows = max(rows - len(lines), 0)
    chunk = frame.slice(page.top, body_rows).select(names)
    values = [chunk.get_column(name).to_list() for name in names]
    for offset in range(chunk.height):
        texts = [_format_value(column_values[offset]) for column_values in values]
        lines.append(line(str(page.top + offset + 1), texts, PLAIN))
    return lines[:rows]


def table_rows_on_screen(rows: int) -> int:
    """Return the data rows a table page shows in ``rows`` lines (header, types and rule take 3)."""
    return max(rows - 3, 1)


# --- text pages -----------------------------------------------------------------


def help_text(target: object) -> tuple[str, str]:
    """Return a title and the help text for ``target``: its signature, then its docstring.

    Parameters
    ----------
    target : object
        A function, class or other object.

    Returns
    -------
    tuple of (str, str)
        The page title (the object's name) and the text.
    """
    name = getattr(target, "__qualname__", None) or getattr(target, "__name__", None)
    if not isinstance(name, str):
        name = type(target).__name__
    try:
        full = inspect.signature(target)  # ty: ignore[invalid-argument-type]
    except (TypeError, ValueError):
        signature = ""
    else:  # without type hints, which the docstring's Parameters section gives
        empty = inspect.Parameter.empty
        parameters = [parameter.replace(annotation=empty) for parameter in full.parameters.values()]
        signature = str(full.replace(parameters=parameters, return_annotation=empty))
        signature = re.sub(r"\s*\n\s*", " ", signature)  # a default whose repr spans lines
    documentation = inspect.getdoc(target) or "No documentation."
    return name, f"{name}{signature}\n\n{documentation}"


_FIELD_SECTIONS = frozenset(
    {"Parameters", "Returns", "Yields", "Raises", "Warns", "Attributes", "Other Parameters"}
)

Block = tuple[str, int, str]  # (kind, indent, text): signature, header, field, code, prose, blank


def _blocks(text: str) -> list[Block]:
    """Split a docstring into blocks, joining the lines of each prose paragraph."""
    lines = _clean_text(text).split("\n")
    blocks: list[Block] = []
    in_fields = False
    index = 0
    while index < len(lines):
        line = lines[index]
        stripped = line.strip()
        indent = len(line) - len(line.lstrip(" "))
        following = lines[index + 1].strip() if index + 1 < len(lines) else ""
        if not blocks:  # the first line is the signature
            blocks.append(("signature", 0, stripped))
            index += 1
            continue
        if not stripped:
            blocks.append(("blank", 0, ""))
        elif len(following) >= 3 and set(following) == {"-"}:
            in_fields = stripped in _FIELD_SECTIONS
            blocks.append(("header", 0, stripped))
            index += 1  # skip the dashes
        elif stripped.startswith((">>> ", "... ")) or stripped in (">>>", "..."):
            blocks.append(("code", indent, line.rstrip()))
        elif in_fields and indent == 0:
            blocks.append(("field", 0, stripped))
        elif blocks and blocks[-1][0] == "prose" and blocks[-1][1] == indent:
            blocks[-1] = ("prose", indent, blocks[-1][2] + " " + stripped)
        else:
            blocks.append(("prose", indent, stripped))
        index += 1
    return blocks


_CODE_START, _CODE_END = "\x01", "\x02"  # markers for ``code`` spans (control chars are gone)


def _mark_code(text: str) -> str:
    """Replace RST ````code```` spans with markers, so they can be colored after wrapping."""
    parts = text.split("``")
    if len(parts) % 2 == 0:  # an unpaired ``: leave the text as it is
        return text
    return "".join(
        part if index % 2 == 0 else _CODE_START + part + _CODE_END
        for index, part in enumerate(parts)
    )


def _marked_segments(line: str, code: str, in_code: bool) -> tuple[list[Segment], bool]:
    """Turn a wrapped line with code markers into segments; returns whether a span is open."""
    segments: list[Segment] = []
    current = ""
    for character in line:
        if character in (_CODE_START, _CODE_END):
            if current:
                segments.append((ACCENT if in_code else code, current))
            current = ""
            in_code = character == _CODE_START
        else:
            current += character
    if current:
        segments.append((ACCENT if in_code else code, current))
    return segments, in_code


def _wrap(text: str, cells: int, first: str, later: str) -> list[str]:
    """Wrap ``text`` at spaces into lines of at most ``cells`` terminal cells.

    The first line starts with ``first``, later ones with ``later``; a word too
    long for a line is broken across lines. Widths count cells, not characters.
    """
    lines: list[str] = []
    current, used, has_word = first, _cells(first), False
    for word in text.split():
        while word:
            needed = _cells(word) + (1 if has_word else 0)
            if used + needed <= cells:
                current += (" " if has_word else "") + word
                used, has_word, word = used + needed, True, ""
            elif has_word:  # start a new line and try the word again
                lines.append(current)
                current, used, has_word = later, _cells(later), False
            else:  # the word alone is too long: break it
                piece = _cut(word, max(cells - used, 1)) or word[0]
                lines.append(current + piece)
                word = word[len(piece) :]
                current, used = later, _cells(later)
    if has_word or not lines:
        lines.append(current)
    return lines


def text_lines(text: str, cells: int) -> list[str]:
    """Lay out a text page: reflow and color a NumPy-style docstring to ``cells``.

    The first line (the signature) is colored; section headers (a line underlined
    with dashes) are bold and lose the dashes; field lines (``name : type``) in
    sections such as Parameters get a bold name; ``>>>`` examples are colored and
    cut, never wrapped; the lines of each prose paragraph are joined and rewrapped.

    Parameters
    ----------
    text : str
        The text; control characters are removed first.
    cells : int
        Width available.

    Returns
    -------
    list of str
        All lines, each at most ``cells`` cells, with SGR escapes.
    """
    width = max(cells, 1)
    lines: list[str] = []
    for kind, indent, body in _blocks(text):
        if kind == "blank":
            lines.append("")
        elif kind == "header":
            lines.append(render_line([(BOLD, body)], cells))
        elif kind == "code":
            lines.append(render_line([(ACCENT, body)], cells))
        elif kind == "field":
            name, separator, rest = body.partition(" : ")
            lines.append(
                render_line([(BOLD, name), (DIM, (separator + rest).replace("``", ""))], cells)
            )
        else:
            first = " " * min(indent, width // 2)
            later = " " * min(indent + (4 if kind == "signature" else 0), width // 2)
            code = ACCENT if kind == "signature" else PLAIN
            wrapped = _wrap(body if kind == "signature" else _mark_code(body), width, first, later)
            in_code = False
            for part in wrapped or [""]:
                segments, in_code = _marked_segments(part, code, in_code)
                lines.append(render_line(segments, cells))
    return lines
