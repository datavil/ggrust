"""Prototype: show a plot inline in a terminal through the Kitty graphics protocol.

Works in terminals that speak the Kitty graphics protocol (Ghostty, Kitty, WezTerm).
Does not detect support, fall back to text cells, or handle tmux passthrough yet.
Not part of the public API: this module is a prototype on its own branch.
"""

from __future__ import annotations

import base64
import io
import os
import sys
from typing import TYPE_CHECKING, TextIO

if TYPE_CHECKING:
    from ggrust.plot.core import PlotSpec
    from ggrust.plot.subplots import SupPlotsSpec

_CHUNK_SIZE = 4096  # the protocol caps each escape's payload at 4096 base64 bytes


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
) -> None:
    """Draw a plot inline in the terminal (Kitty graphics protocol).

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The plot or grid of plots.
    columns : int or None, default=None
        Width in terminal cells. None uses the terminal width, capped at 100.
    stream : TextIO or None, default=None
        Where to write; ``sys.stdout`` when None.
    """
    out = stream if stream is not None else sys.stdout
    if columns is None:
        try:
            columns = min(os.get_terminal_size(out.fileno()).columns, 100)
        except (OSError, ValueError, io.UnsupportedOperation):
            columns = 80
    buffer = io.BytesIO()
    plot.to_png(buffer)
    out.write(kitty_escape(buffer.getvalue(), columns) + "\n")
    out.flush()
