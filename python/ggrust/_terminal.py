"""Prototype: show a plot inline in a terminal.

Terminals that speak the Kitty graphics protocol (Ghostty, Kitty, WezTerm) get the
full PNG. Every other terminal gets coloured braille text drawn by the Rust core.
Detection reads environment variables only (no terminal query), and tmux
passthrough is not handled. Not part of the public API: a prototype on its own branch.
"""

from __future__ import annotations

import base64
import io
import os
import sys
import warnings
from typing import TYPE_CHECKING, Literal, TextIO

from ggrust._ggrust import render
from ggrust._render import GgrustWarning, _engine_input
from ggrust.errors import SpecError

if TYPE_CHECKING:
    from ggrust.plot.core import PlotSpec
    from ggrust.plot.subplots import SupPlotsSpec

_CHUNK_SIZE = 4096  # the protocol caps each escape's payload at 4096 base64 bytes

TerminalMode = Literal["auto", "kitty", "braille"]


def detect_mode() -> Literal["kitty", "braille"]:
    """Guess whether this terminal draws Kitty graphics, from its environment.

    Returns
    -------
    {"kitty", "braille"}
        ``"kitty"`` in Ghostty, Kitty and WezTerm outside tmux or screen, else
        ``"braille"``.
    """
    if os.environ.get("TMUX") or os.environ.get("TERM", "").startswith("screen"):
        return "braille"
    program = os.environ.get("TERM_PROGRAM", "").lower()
    term = os.environ.get("TERM", "").lower()
    if program in {"ghostty", "wezterm"} or "kitty" in term or "ghostty" in term:
        return "kitty"
    if os.environ.get("KITTY_WINDOW_ID"):
        return "kitty"
    return "braille"


def braille_text(plot: PlotSpec | SupPlotsSpec, columns: int) -> str:
    """Render a plot as coloured braille text (24-bit ANSI colour escapes).

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The plot or grid of plots.
    columns : int
        Width in terminal cells, 1 to 1000.

    Returns
    -------
    str
        One line per cell row, each ending in a colour reset and a newline.
    """
    spec_json, exported, functions = _engine_input(plot, None, False)
    encoded, messages = render(
        spec_json, exported, "braille", 1.0, functions=functions, columns=columns
    )
    for message in messages:
        warnings.warn(message, GgrustWarning, stacklevel=3)
    return encoded.decode("utf-8")


def kitty_escape(png: bytes, columns: int | None = None) -> str:
    """Encode PNG bytes as Kitty graphics escapes that display the image at the cursor.

    Parameters
    ----------
    png : bytes
        The PNG file contents.
    columns : int or None, default=None
        Width in terminal cells; the height follows the image's aspect ratio.
        None draws the image at its pixel size.

    Returns
    -------
    str
        The escape sequence, ready to write to the terminal.
    """
    payload = base64.standard_b64encode(png).decode("ascii")
    chunks = [payload[i : i + _CHUNK_SIZE] for i in range(0, len(payload), _CHUNK_SIZE)] or [""]
    size = f",c={columns}" if columns is not None else ""
    parts: list[str] = []
    for index, chunk in enumerate(chunks):
        more = 1 if index < len(chunks) - 1 else 0
        keys = f"a=T,f=100,q=2{size},m={more}" if index == 0 else f"m={more}"
        parts.append(f"\x1b_G{keys};{chunk}\x1b\\")
    return "".join(parts)


def show_in_terminal(
    plot: PlotSpec | SupPlotsSpec,
    columns: int | None = None,
    stream: TextIO | None = None,
    mode: TerminalMode = "auto",
) -> None:
    """Draw a plot inline in the terminal: an image, or braille text as a fallback.

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The plot or grid of plots.
    columns : int or None, default=None
        Width in terminal cells. None uses the terminal width, capped at 100.
    stream : TextIO or None, default=None
        Where to write; ``sys.stdout`` when None.
    mode : {"auto", "kitty", "braille"}, default="auto"
        ``"kitty"`` draws the PNG with the Kitty graphics protocol, ``"braille"``
        draws text, ``"auto"`` picks by terminal (see ``detect_mode``).

    Raises
    ------
    SpecError
        If ``mode`` is not one of the accepted values.
    """
    if mode not in ("auto", "kitty", "braille"):
        raise SpecError(
            f"show_in_terminal(mode={mode!r}) is not a mode; expected 'auto', 'kitty' or 'braille'"
        )
    out = stream if stream is not None else sys.stdout
    if columns is None:
        try:
            columns = min(os.get_terminal_size(out.fileno()).columns, 100)
        except (OSError, ValueError, io.UnsupportedOperation):
            columns = 80
    chosen = detect_mode() if mode == "auto" else mode
    if chosen == "braille":
        out.write(braille_text(plot, columns))
    else:
        buffer = io.BytesIO()
        plot.to_png(buffer)
        out.write(kitty_escape(buffer.getvalue(), columns) + "\n")
    out.flush()
