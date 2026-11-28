"""Prototype: an RStudio-style plot pane for the terminal.

A viewer process (``python -m ggrust._viewer``) owns a terminal pane and keeps a
history of pages: plots, rendered to fill the pane (and re-rendered when it is
resized) with the Kitty graphics protocol (Ghostty, Kitty, WezTerm); data tables;
and help pages. A REPL in another pane talks to it over a Unix socket: each message
is a pickled ``(protocol, kind, body)`` tuple, and the pane answers with a ``Reply``
(the page's number, or why it failed). ``ggrust._pane_repl`` is that REPL side.
Left and right arrows (or ``h`` / ``l``) page through the history, up and down (``j``
/ ``k``, space, ``b``) scroll tables and help, ``[`` / ``]`` scroll table columns,
``o`` opens a plot in the browser, ``q`` quits.

``ggrust`` opens a Ghostty window (AppleScript, macOS) with a Python REPL on the left
and the viewer on the right.

Plots that set no flavor of their own get ``flavor_high_contrast_dark()`` (or the
light one, after asking the terminal for its background) and ``flavor_transparent()``,
so they sit on the terminal's background. Messages travel pickled over a socket only
the user can open (in a 0700 directory of the user's own). Does not: work without
Kitty graphics, or handle tmux. Not part of the public API: a prototype on its own branch.
"""

from __future__ import annotations

import codecs
import contextlib
import fcntl
import io
import os
import pickle
import re
import secrets
import select
import selectors
import shlex
import shutil
import signal
import socket
import stat
import struct
import subprocess
import sys
import tempfile
import termios
import threading
import time
import tty
import warnings
import webbrowser
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Literal

from ggrust._pane_pages import (
    Page,
    _cells,
    _clean_label,
    _cut,
    table_lines,
    table_rows_on_screen,
    text_lines,
    visible_columns,
)
from ggrust._terminal import kitty_escape

if TYPE_CHECKING:
    import polars as pl

    from ggrust.plot.core import PlotSpec
    from ggrust.plot.subplots import SupPlotsSpec

PaneStyle = Literal["dark", "light", "none"]

_SCALE = 2.0  # device pixels per layout pixel (retina)
_FOOTER_ROWS = 1
_FRAME = 1  # cells of box border on each side of the plot
_RESIZE_PAUSE = 0.15  # seconds without resize events before re-rendering
_RECEIVE_TIMEOUT = 3.0  # seconds a client may take to send one plot
_MAX_PAYLOAD = 1 << 30  # bytes; larger messages are refused unread
_MAX_SOCKET_PATH = 103  # bytes; macOS caps AF_UNIX paths at 104 with the NUL
_FALLBACK_CELL = (10, 20)  # device pixels per cell when the terminal reports none
_FALLBACK_SIZE = (24, 80)  # rows and columns when the terminal reports none
_MAX_PENDING = 4096  # characters of an unfinished escape sequence kept between reads
_REPLY_TIMEOUT = 120.0  # seconds a client waits for the pane to draw and answer
_PROTOCOL = "ggrust-pane/1"  # first item of every message tuple
_MAX_TABLE_ROWS = 10_000  # rows of a frame sent to the pane; the rest is only counted
_MAX_PLOT_SIDE = 20_000  # layout pixels; ``pane.size`` refuses larger sides


class PaneError(RuntimeError):
    """The plot pane cannot start: no terminal, a live pane on the socket, or a bad path."""


class MessageError(ValueError):
    """A message on the pane's socket is not usable: too big, too slow, cut short or junk."""


MessageKind = Literal["plot", "table", "text", "control"]


@dataclass
class Reply:
    """The pane's answer to one message.

    Attributes
    ----------
    index : int
        1-based position of the page the message added (or that is on screen), 0 if none.
    total : int
        Pages in the history.
    error : str or None
        Why the message failed, or why a plot could not be drawn.
    value : object
        What a ``control`` message asked for (``last``, ``state``), else None.
    """

    index: int = 0
    total: int = 0
    error: str | None = None
    value: object = None


def _runtime_directory() -> Path:
    """Return the user's private ggrust directory in the temp directory (not created here)."""
    return Path(tempfile.gettempdir()) / f"ggrust-{os.getuid()}"


def _private_directory() -> Path:
    """Create (or check) the user's 0700 runtime directory and return it.

    Raises
    ------
    PaneError
        When the path exists but is a symlink, not a directory, or someone else's.
    """
    directory = _runtime_directory()
    with contextlib.suppress(FileExistsError):
        directory.mkdir(mode=0o700)
    info = directory.lstat()
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid():
        raise PaneError(
            f"{directory} is not a directory owned by you; remove it, or set "
            "GGRUST_VIEWER_SOCKET to a socket path in a directory of your own"
        )
    if info.st_mode & 0o077:
        directory.chmod(0o700)
    return directory


def socket_path() -> Path:
    """Where the viewer listens: ``$GGRUST_VIEWER_SOCKET``, else the user's private directory.

    Returns
    -------
    Path
        The Unix socket path.
    """
    override = os.environ.get("GGRUST_VIEWER_SOCKET")
    if override:
        return Path(override)
    return _runtime_directory() / "viewer.sock"


def _pane_size() -> tuple[int, int, int, int]:
    """Rows, columns, width and height in pixels of the terminal on stdout."""
    try:
        packed = fcntl.ioctl(sys.stdout.fileno(), termios.TIOCGWINSZ, b"\0" * 8)
    except (OSError, ValueError):  # no terminal: the caller falls back to a default size
        return 0, 0, 0, 0
    rows, columns, width, height = struct.unpack("HHHH", packed)
    return rows, columns, width, height


def _plot_area() -> tuple[int, int, int, int]:
    """Rows, columns and pixel size of the pane above the footer.

    Terminals that report no size get 24 x 80 cells; ones that report no pixel
    size get ``_FALLBACK_CELL`` pixels per cell, so a plot still renders.
    """
    rows, columns, width, height = _pane_size()
    if rows <= 0 or columns <= 0:
        rows, columns = _FALLBACK_SIZE
    if width <= 0 or height <= 0:
        width, height = columns * _FALLBACK_CELL[0], rows * _FALLBACK_CELL[1]
    plot_rows = max(rows - _FOOTER_ROWS, 1)
    plot_height = height * plot_rows // max(rows, 1)
    return plot_rows, columns, width, plot_height


class _Viewer:
    """The pane: a history of pages (plots, tables, text), rendering and drawing."""

    def __init__(self, style: PaneStyle = "none") -> None:
        self.pages: list[Page] = []
        self.index = -1
        self.notice = ""  # one-off message shown in the footer
        # page identity, width, height, style, png
        self.cache: tuple[int, int, int, str, bytes] | None = None
        self.style = style
        self.fixed_size: tuple[float, float] | None = None  # layout px set by ``pane.size``
        self.render_error: str | None = None  # why the last plot drawn failed, if it did
        self.html_directory: Path | None = None  # made on the first ``o``, removed at exit

    @property
    def page(self) -> Page | None:
        """The page on screen, or None when the history is empty."""
        return self.pages[self.index] if 0 <= self.index < len(self.pages) else None

    def add(self, page: Page) -> None:
        """Append ``page`` to the history and show it; if drawing it fails, take it back out."""
        previous = self.index
        self.pages.append(page)
        self.index = len(self.pages) - 1
        try:
            self.draw()
        except Exception:
            self.pages.pop()
            self.index = previous
            with contextlib.suppress(Exception):  # put the page that was on screen back
                self.draw()
            raise

    def render(self, width: int, height: int) -> bytes:
        """Render the current plot as PNG bytes of ``width`` x ``height`` pixels (cached).

        With a fixed size (``pane.size``) the plot is rendered at that size instead,
        and drawn as large as fits in the frame.
        """
        page = self.page
        assert page is not None and page.plot is not None
        if self.fixed_size is not None:
            width, height = (round(side * _SCALE) for side in self.fixed_size)
        key = (id(page), width, height, self.style)
        if self.cache is not None and self.cache[:4] == key:
            return self.cache[4]
        buffer = io.BytesIO()
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            styled(page.plot, self.style).to_png(
                buffer, w=width / _SCALE, h=height / _SCALE, unit="px", scale=_SCALE
            )
        png = buffer.getvalue()
        self.cache = (*key, png)
        return png

    def label(self) -> str:
        """Return the frame's title: what the page is, its place in the history, its title."""
        page = self.page
        assert page is not None
        word = {"plot": "plot", "table": "data", "text": "help"}[page.kind]
        label = f"{word} {self.index + 1}/{len(self.pages)}"
        if page.kind == "table" and page.frame is not None:
            label += f" \u00b7 {page.title} \u00b7 {page.total_rows:,} \u00d7 {page.frame.width}"
        elif page.title:
            label += f" \u00b7 {page.title}"
        return label

    def draw(self) -> None:
        """Clear the pane and draw the current page in its frame, then the footer."""
        out = sys.stdout
        out.write("\x1b_Ga=d,q=2\x1b\\\x1b[2J\x1b[H")  # delete images, clear, home
        rows, columns, width, height = _plot_area()
        page = self.page
        self.render_error = None
        if page is None:
            out.write("ggrust plots: waiting for a plot...\r\n")
            self.write_footer()
            return
        cell_width = width / max(columns, 1)
        cell_height = height / max(rows, 1)
        framed = rows > 2 * _FRAME and columns > 2 * _FRAME
        inner_rows, inner_columns = rows, columns
        if framed:
            out.write(_frame(rows, columns, self.label()))
            inner_rows, inner_columns = rows - 2 * _FRAME, columns - 2 * _FRAME
        top = 1 + _FRAME if framed else 1
        left = 1 + _FRAME if framed else 1
        if page.kind == "plot":
            out.write(f"\x1b[{top};{left}H")
            try:
                png = self.render(
                    round(inner_columns * cell_width), round(inner_rows * cell_height)
                )
            except Exception as error:  # show any render failure in the pane
                self.render_error = f"{type(error).__name__}: {error}"
                page.failed = True
                message = f"could not draw plot {self.index + 1}: {self.render_error}"
                out.write(_cut(_clean_label(message), inner_columns))
            else:
                page.failed = False
                out.write(_fit_escape(png, inner_rows, inner_columns, cell_width, cell_height))
        else:  # one cell of padding inside the frame on both sides
            pad = 1 if inner_columns > 4 else 0
            lines = self.text_page_lines(page, inner_rows, inner_columns - 2 * pad)
            for offset, line in enumerate(lines):
                out.write(f"\x1b[{top + offset};{left + pad}H{line}")
        self.write_footer()

    def text_page_lines(self, page: Page, rows: int, columns: int) -> list[str]:
        """Return the lines of a table or text page that are on screen."""
        if page.kind == "table":
            if page.frame is not None:  # the last row stays at the bottom, as the pane resizes
                page.top = max(min(page.top, page.frame.height - table_rows_on_screen(rows)), 0)
            return table_lines(page, rows, columns)
        lines = text_lines(page.text, columns)
        page.top = max(min(page.top, len(lines) - rows), 0)
        return lines[page.top : page.top + rows]

    def footer(self) -> str:
        """Return the footer text for the current page: where you are and the keys that apply."""
        page = self.page
        history = "\u2190 \u2192 pages   q quit"
        if page is None:
            return f"style {self.style}   q quit"
        if page.kind == "plot":
            size = (
                ""
                if self.fixed_size is None
                else f"   size {self.fixed_size[0]:g}x{self.fixed_size[1]:g}"
            )
            return f"style {self.style}{size}   {history}   o browser"
        rows, columns, _, _ = _plot_area()
        inner_rows, inner_columns = rows - 2 * _FRAME, columns - 2 * _FRAME
        if page.kind == "table":
            assert page.frame is not None
            shown = table_rows_on_screen(inner_rows)
            last_row = min(page.top + shown, page.frame.height)
            visible = visible_columns(page, inner_columns - 2)
            more = page.frame.height < page.total_rows
            where = f"rows {page.top + 1}-{last_row}/{page.frame.height:,}"
            where += f" (of {page.total_rows:,})" if more else ""
            where += f"  cols {visible.start + 1}-{visible.stop}/{page.frame.width}"
            return f"{where}   \u2191\u2193 scroll  [ ] cols  \u2190 \u2192 pages  q quit"
        return f"\u2191\u2193 space scroll   {history}"

    def write_footer(self) -> None:
        """Write the footer row: a pending notice, else where you are and the keys."""
        rows, columns, _, _ = _plot_area()
        text = _cut(" " + _clean_label(self.notice or self.footer()), columns)
        sys.stdout.write(f"\x1b[{rows + 1};1H\x1b[2K\x1b[2m{text}\x1b[0m")
        sys.stdout.flush()
        self.notice = ""

    def open_in_browser(self) -> None:
        """Save the current plot as interactive HTML and open it in the default browser."""
        page = self.page
        if page is None or page.plot is None:
            self.notice = "o opens plots in the browser; this page is not a plot"
            self.write_footer()
            return
        try:
            if self.html_directory is None:
                self.html_directory = Path(tempfile.mkdtemp(prefix="ggrust-pane-"))
            path = self.html_directory / f"plot-{self.index + 1}.html"
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                page.plot.to_html(path)  # as sent: browsers have a light page
            opened = webbrowser.open(path.as_uri())
        except Exception as error:  # report any failure in the footer
            self.notice = f"could not open plot {self.index + 1} in a browser: {error}"
        else:
            self.notice = (
                f"opened plot {self.index + 1} in the browser"
                if opened
                else f"no browser found; the plot is saved at {path}"
            )
        self.write_footer()

    def cleanup(self) -> None:
        """Remove the HTML files saved for the browser (at exit, once it has loaded them)."""
        if self.html_directory is not None:
            shutil.rmtree(self.html_directory, ignore_errors=True)
            self.html_directory = None

    def receive(self, connection: socket.socket) -> None:
        """Read one message from ``connection``, act on it, and send the reply back.

        A connection that closes before sending anything (a probe) is ignored, and so
        is a client that hangs up before reading the reply.

        Raises
        ------
        MessageError
            When the message is too big, too slow, cut short, or not one the pane knows.
        """
        deadline = time.monotonic() + _RECEIVE_TIMEOUT
        failure: Exception | None = None
        try:
            header = _read_exact(connection, 8, deadline)
            if header is None:
                connection.close()
                return
            (length,) = struct.unpack(">Q", header)
            if length > _MAX_PAYLOAD:
                raise MessageError(f"refused a {length}-byte message; the limit is {_MAX_PAYLOAD}")
            payload = _read_exact(connection, length, deadline)
            if payload is None:
                raise MessageError(f"the sender hung up before sending all {length} bytes")
            reply = self.handle(*_load_message(payload))
        except Exception as error:  # tell the sender why, then report it here too
            message = str(error) if isinstance(error, MessageError) else _describe(error)
            reply, failure = self.reply(error=message), error
        answer = pickle.dumps(reply)
        _send_reply(connection, struct.pack(">Q", len(answer)) + answer)
        if failure is not None:
            raise failure

    def handle(self, kind: MessageKind, body: object) -> Reply:
        """Act on one message and return the reply.

        Raises
        ------
        MessageError
            When the body does not fit the kind.
        """
        if kind == "plot":
            plot = _as_plot(body)
            try:
                title = _title(plot)
            except Exception as error:  # a plot whose spec cannot be read
                raise MessageError(f"could not read the plot's spec: {error}") from error
            self.add(Page("plot", title, plot=plot))
            return self.reply(error=self.render_error)
        if kind == "table":
            name, frame, total_rows = _as_table(body)
            self.add(Page("table", _clean_label(name), frame=frame, total_rows=total_rows))
            return self.reply()
        if kind == "text":
            if not (
                isinstance(body, tuple) and len(body) == 2 and all(isinstance(x, str) for x in body)
            ):
                raise MessageError("a text message holds a (title, text) pair of strings")
            title, text = body
            self.add(Page("text", _clean_label(title), text=text))
            return self.reply()
        return self.control(body)

    def reply(self, error: str | None = None, value: object = None) -> Reply:
        """Return a reply naming the page on screen."""
        return Reply(self.index + 1, len(self.pages), error, value)

    def last_plot(self) -> PlotSpec | SupPlotsSpec | None:
        """Return the plot on screen, else the nearest one before it; failed plots are skipped."""
        for page in reversed(self.pages[: self.index + 1]):
            if page.plot is not None and not page.failed:
                return page.plot
        return None

    def control(self, body: object) -> Reply:
        """Act on a ``control`` message: ``(name, argument)``.

        Names: ``last`` (the plot on screen), ``state`` (history position and the
        plot area's size in layout pixels), ``style``, ``size`` and ``clear``.

        Raises
        ------
        MessageError
            When the name is unknown or the argument does not fit it.
        """
        if not (isinstance(body, tuple) and len(body) == 2 and isinstance(body[0], str)):
            raise MessageError("a control message holds a (name, argument) pair")
        name, argument = body
        if name == "last":
            return self.reply(value=self.last_plot())
        if name == "state":
            rows, columns, width, height = _plot_area()
            inner = (
                width * (columns - 2 * _FRAME) / max(columns, 1) / _SCALE,
                height * (rows - 2 * _FRAME) / max(rows, 1) / _SCALE,
            )
            return self.reply(value={"size": self.fixed_size or inner, "style": self.style})
        if name == "style":
            if argument not in ("dark", "light", "none"):
                raise MessageError(f"unknown style {argument!r}; use 'dark', 'light' or 'none'")
            self.style = argument
        elif name == "size":
            if argument is not None:
                if not (isinstance(argument, tuple) and len(argument) == 2):
                    raise MessageError("size takes None or a (width, height) pair")
                problem = size_problem(*argument)
                if problem is not None:
                    raise MessageError(problem)
            self.fixed_size = None if argument is None else (float(argument[0]), float(argument[1]))
        elif name == "clear":
            self.pages.clear()
            self.index = -1
            self.cache = None
        else:
            raise MessageError(f"unknown control {name!r}")
        self.draw()
        return self.reply()

    def scroll(self, rows: int = 0, columns: int = 0, to: int | None = None) -> None:
        """Scroll a table or text page by ``rows`` / ``columns`` (or to row ``to``)."""
        page = self.page
        if page is None or page.kind == "plot":
            return
        if page.kind == "table" and page.frame is not None:
            limit = max(page.frame.height - table_rows_on_screen(self.inner_rows()), 0)
            page.left = max(min(page.left + columns, page.frame.width - 1), 0)
        else:
            limit = 1 << 30  # text pages clamp when drawn, once the wrapping is known
        page.top = max(min(page.top + rows if to is None else to, limit), 0)
        self.draw()

    def inner_rows(self) -> int:
        """Return the lines inside the frame (the whole plot area when it is too small for one)."""
        rows, columns = _plot_area()[:2]
        return rows - 2 * _FRAME if rows > 2 * _FRAME and columns > 2 * _FRAME else max(rows, 1)

    def page_step(self) -> int:
        """Return how many rows or lines a page-down moves: a screenful less one."""
        rows = self.inner_rows()
        if self.page is not None and self.page.kind == "table":
            rows = table_rows_on_screen(rows)
        return max(rows - 1, 1)

    def key(self, key: str) -> bool:
        """Handle a key; False means quit."""
        if key == "q":
            return False
        if key == "o":
            self.open_in_browser()
        elif key in ("\x1b[D", "\x1bOD", "h") and self.index > 0:
            self.index -= 1
            self.draw()
        elif key in ("\x1b[C", "\x1bOC", "l") and self.index < len(self.pages) - 1:
            self.index += 1
            self.draw()
        elif key in ("\x1b[A", "\x1bOA", "k"):
            self.scroll(rows=-1)
        elif key in ("\x1b[B", "\x1bOB", "j"):
            self.scroll(rows=1)
        elif key in (" ", "\x1b[6~"):
            self.scroll(rows=self.page_step())
        elif key in ("b", "\x1b[5~"):
            self.scroll(rows=-self.page_step())
        elif key in ("g", "\x1b[H"):
            self.scroll(to=0)
        elif key in ("G", "\x1b[F"):
            self.scroll(to=1 << 30)
        elif key == "]":
            self.scroll(columns=1)
        elif key == "[":
            self.scroll(columns=-1)
        return True


def _describe(error: Exception) -> str:
    """Return ``error`` as one line for a reply or the footer: ``TypeError: ...``."""
    return f"{type(error).__name__}: {error}"


def _send_reply(connection: socket.socket, data: bytes) -> None:
    """Send ``data`` and close ``connection`` without making the pane wait on the client.

    What fits in the socket's buffer goes at once; the rest (a large ``last`` plot to
    a client that does not read) is finished by a background thread, which gives up
    after ``_RECEIVE_TIMEOUT`` seconds without progress.
    """
    try:
        connection.setblocking(False)
        sent = connection.send(data)
    except BlockingIOError:
        sent = 0
    except OSError:  # the sender stopped listening: fine
        connection.close()
        return
    if sent >= len(data):
        connection.close()
        return

    def finish() -> None:
        with connection, contextlib.suppress(OSError):
            connection.settimeout(_RECEIVE_TIMEOUT)
            connection.sendall(data[sent:])

    threading.Thread(target=finish, name="ggrust-pane-reply", daemon=True).start()


def size_problem(width: object, height: object) -> str | None:
    """Return why ``(width, height)`` is no plot size for ``pane.size``, or None if it is one.

    Each side must be a finite number of layout pixels (not a bool) in
    ``(0, _MAX_PLOT_SIDE]``.
    """
    for label, side in (("w", width), ("h", height)):
        if isinstance(side, bool) or not isinstance(side, int | float):
            return f"pane.size {label} must be a number of pixels, got {type(side).__name__}"
        if not 0 < side <= _MAX_PLOT_SIDE:  # refuses nan, inf and huge ints
            return (
                f"pane.size {label}={side!r} is out of range; use a number of pixels "
                f"above 0 and at most {_MAX_PLOT_SIDE:,}"
            )
    return None


def _load_plot(payload: bytes) -> PlotSpec | SupPlotsSpec:
    """Unpickle a plot sent to the pane.

    The socket is the user's own (mode 0600 in a 0700 directory), so the payload
    is trusted; this only rejects junk and pickles of other things.

    Raises
    ------
    MessageError
        When the payload does not unpickle, or holds something other than a plot.
    """
    from ggrust.plot.core import PlotSpec
    from ggrust.plot.subplots import SupPlotsSpec

    try:
        plot = pickle.loads(payload)
    except Exception as error:  # pickle raises many kinds
        raise MessageError(f"not a pickled plot ({type(error).__name__}: {error})") from error
    if not isinstance(plot, PlotSpec | SupPlotsSpec):
        raise MessageError(f"expected a PlotSpec or SupPlotsSpec, got {type(plot).__name__}")
    return plot


def _as_plot(body: object) -> PlotSpec | SupPlotsSpec:
    """``body`` if it is a plot or grid of plots.

    Raises
    ------
    MessageError
        When it is something else.
    """
    from ggrust.plot.core import PlotSpec
    from ggrust.plot.subplots import SupPlotsSpec

    if not isinstance(body, PlotSpec | SupPlotsSpec):
        raise MessageError(f"expected a PlotSpec or SupPlotsSpec, got {type(body).__name__}")
    return body


def _as_table(body: object) -> tuple[str, pl.DataFrame, int]:
    """``body`` as a ``(name, frame, total_rows)`` table message.

    Raises
    ------
    MessageError
        When it is something else.
    """
    import polars as pl

    if not (
        isinstance(body, tuple)
        and len(body) == 3
        and isinstance(body[0], str)
        and isinstance(body[1], pl.DataFrame)
        and isinstance(body[2], int)
    ):
        raise MessageError("a table message holds (name, polars.DataFrame, total_rows)")
    return body[0], body[1], body[2]


def _load_message(payload: bytes) -> tuple[MessageKind, object]:
    """Unpickle a message: ``(protocol, kind, body)``, or a bare plot (sent by ``send``).

    Raises
    ------
    MessageError
        When the payload does not unpickle, or is not a message the pane knows.
    """
    try:
        message = pickle.loads(payload)
    except Exception as error:  # pickle raises many kinds
        raise MessageError(
            f"not a pickled plot or message ({type(error).__name__}: {error})"
        ) from error
    if isinstance(message, tuple) and len(message) == 3 and message[0] == _PROTOCOL:
        _, kind, body = message
        if kind not in ("plot", "table", "text", "control"):
            raise MessageError(f"unknown message kind {kind!r}")
        return kind, body
    return "plot", _as_plot(message)


def _title(plot: PlotSpec | SupPlotsSpec) -> str:
    """Return the plot's ``ggtitle`` text on one line, or "" when it has none."""
    title = (plot.as_dict().get("ggtitle") or {}).get("text")
    return _clean_label(str(title)) if title else ""


def _png_size(png: bytes) -> tuple[int, int]:
    """Width and height from a PNG's IHDR chunk."""
    width, height = struct.unpack(">II", png[16:24])
    return width, height


def _frame(rows: int, columns: int, title: str) -> str:
    """Draw a dim rounded box over ``rows`` x ``columns`` cells, ``title`` in its top edge.

    Needs ``rows`` and ``columns`` of at least 3. Every edge is exactly ``columns``
    cells wide; a title that does not fit is cut (marked with an ellipsis) or dropped.
    """
    title = _clean_label(title)
    room = columns - 4  # the corners, one rule cell before the label and one after
    label = f" {title} "
    if _cells(label) > room:  # cut long titles, marking the cut
        label = " " + _cut(title, room - 3) + "\u2026 " if room >= 3 else ""
    top = "\u256d\u2500" + label + "\u2500" * (columns - 3 - _cells(label)) + "\u256e"
    side = "\u2502" + " " * (columns - 2) + "\u2502"
    bottom = "\u2570" + "\u2500" * (columns - 2) + "\u256f"
    lines = [top] + [side] * (rows - 2) + [bottom]
    body = "".join(f"\x1b[{row};1H{line}" for row, line in enumerate(lines, start=1))
    return f"\x1b[2m{body}\x1b[0m"


def _fit_escape(png: bytes, rows: int, columns: int, cell_width: float, cell_height: float) -> str:
    """Kitty escapes drawing ``png`` as large as fits in ``rows`` x ``columns`` cells."""
    if cell_width <= 0 or cell_height <= 0:
        return kitty_escape(png, columns, move_cursor=False)
    width, height = _png_size(png)
    # Largest scale that fits both ways, in whole cells, never past the area.
    scale = min(columns * cell_width / width, rows * cell_height / height)
    fit_columns = min(max(round(width * scale / cell_width), 1), columns)
    fit_rows = min(max(round(height * scale / cell_height), 1), rows)
    return kitty_escape(png, fit_columns, fit_rows, move_cursor=False)


def _read_exact(connection: socket.socket, length: int, deadline: float) -> bytes | None:
    """``length`` bytes from ``connection``, or None if it closes first.

    Raises
    ------
    MessageError
        When the bytes have not all arrived by ``deadline`` (``time.monotonic``).
    """
    chunks: list[bytes] = []
    remaining = length
    while remaining:
        left = deadline - time.monotonic()
        if left <= 0:
            raise MessageError(f"timed out after {_RECEIVE_TIMEOUT:g} s waiting for a plot")
        connection.settimeout(left)
        try:
            chunk = connection.recv(min(remaining, 1 << 20))
        except TimeoutError as error:
            raise MessageError(
                f"timed out after {_RECEIVE_TIMEOUT:g} s waiting for a plot"
            ) from error
        if not chunk:
            return None
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def styled(plot: PlotSpec | SupPlotsSpec, style: PaneStyle) -> PlotSpec | SupPlotsSpec:
    """Dress a plot for the pane, unless it picks its own flavor.

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The plot as sent.
    style : {"dark", "light", "none"}
        ``"dark"`` adds ``flavor_high_contrast_dark() + flavor_transparent()``,
        ``"light"`` the light high-contrast flavor, ``"none"`` leaves the plot alone.

    Returns
    -------
    PlotSpec or SupPlotsSpec
        A new plot; ``plot`` itself is unchanged.
    """
    from ggrust.plot.theme import (
        flavor_high_contrast_dark,
        flavor_high_contrast_light,
        flavor_transparent,
    )

    theme = plot.as_dict().get("theme") or {}
    if style == "none" or "flavor" in theme:
        return plot
    flavor = flavor_high_contrast_dark() if style == "dark" else flavor_high_contrast_light()
    return plot + flavor + flavor_transparent()


_OSC_11_REPLY = re.compile(r"\x1b\]11;rgb:([0-9a-fA-F]+)/([0-9a-fA-F]+)/([0-9a-fA-F]+)")


def _terminal_style(keys: _KeyReader, timeout: float = 0.3) -> PaneStyle:
    """Ask the terminal for its background colour (OSC 11) and pick a style.

    Needs stdin in cbreak mode. The reply is read through ``keys``, so it is read up
    to its terminator however it is split, and a reply that comes after ``timeout``
    is dropped there rather than taken as keys. Keys typed while waiting are dropped.
    Terminals that do not answer in ``timeout`` seconds get ``"dark"``.
    """
    sys.stdout.write("\x1b]11;?\x1b\\")
    sys.stdout.flush()
    deadline = time.monotonic() + timeout
    while (left := deadline - time.monotonic()) > 0:
        ready, _, _ = select.select([sys.stdin], [], [], left)
        if not ready:
            break
        for token in keys.feed(os.read(sys.stdin.fileno(), 64)):
            match = _OSC_11_REPLY.match(token)
            if match is None:
                continue
            red, green, blue = (int(part, 16) / (16 ** len(part) - 1) for part in match.groups())
            luminance = 0.2126 * red + 0.7152 * green + 0.0722 * blue
            return "dark" if luminance < 0.5 else "light"
    return "dark"


def _token_end(text: str, start: int) -> int | None:
    """End of the key or escape sequence starting at ``text[start]``; None if unfinished."""
    if text[start] != "\x1b":
        return start + 1
    if start + 1 == len(text):
        return None  # a lone ESC: maybe the start of a sequence split across reads
    kind = text[start + 1]
    if kind == "[":  # CSI: parameters and intermediates, then one final byte
        for index in range(start + 2, len(text)):
            if "\x40" <= text[index] <= "\x7e":
                return index + 1
            if not "\x20" <= text[index] <= "\x3f":
                return index  # malformed: end it before the stray character
        return None
    if kind == "O":  # SS3, as arrow keys send in application mode
        return start + 3 if start + 2 < len(text) else None
    # OSC, as the terminal's replies (``ESC ] 11;...``) start with digits; ESC ] then
    # anything else is Alt+] and a key, so it cannot hold the input back.
    if kind == "]" and (start + 2 == len(text) or text[start + 2].isdigit()):
        for index in range(start + 2, len(text)):
            if text[index] == "\x07":
                return index + 1
            if text[index] == "\x1b":
                if index + 1 == len(text):
                    return None
                if text[index + 1] == "\\":
                    return index + 2
        return None
    return start + 1  # ESC before an ordinary key: ESC on its own


def _split_keys(text: str) -> tuple[list[str], str]:
    """Split terminal input into keys and escape sequences.

    Returns
    -------
    tuple of (list of str, str)
        The complete tokens, and an unfinished escape sequence left at the end.
    """
    tokens: list[str] = []
    position = 0
    while position < len(text):
        end = _token_end(text, position)
        if end is None:
            return tokens, text[position:]
        tokens.append(text[position:end])
        position = end
    return tokens, ""


class _KeyReader:
    """Turn raw reads from the terminal into keys, across split and batched reads."""

    def __init__(self) -> None:
        self.decoder = codecs.getincrementaldecoder("utf-8")(errors="replace")
        self.pending = ""  # an escape sequence that has not finished arriving

    def feed(self, data: bytes) -> list[str]:
        """Return the complete keys and escape sequences in ``data`` and earlier leftovers."""
        tokens, self.pending = _split_keys(self.pending + self.decoder.decode(data))
        if len(self.pending) > _MAX_PENDING:  # an unterminated sequence: give up on it
            self.pending = ""
        return tokens


def _listen(path: Path) -> tuple[socket.socket, tuple[int, int]]:
    """Bind and listen on ``path``, refusing to take over a live pane.

    A socket file nobody answers on is stale and is replaced. The socket is created
    under umask 077, so it is never open to others, even briefly.

    Returns
    -------
    tuple of (socket.socket, tuple of (int, int))
        The listening socket, and the device and inode of its file (to tell, at
        exit, whether the file is still ours).

    Raises
    ------
    PaneError
        When a pane already listens on ``path``, the path is not a socket, or the
        path is too long for a Unix socket.
    """
    if len(os.fsencode(path)) > _MAX_SOCKET_PATH:
        raise PaneError(
            f"the socket path {str(path)!r} is longer than {_MAX_SOCKET_PATH} bytes; "
            "set GGRUST_VIEWER_SOCKET to a shorter path"
        )
    if path.exists() or path.is_symlink():
        if not stat.S_ISSOCK(path.lstat().st_mode):
            raise PaneError(f"{path} exists and is not a socket; remove it or pick another path")
        probe = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        with probe:
            try:
                probe.connect(str(path))
            except (ConnectionRefusedError, FileNotFoundError):  # a pane that died
                try:
                    path.unlink(missing_ok=True)
                except OSError as error:
                    raise PaneError(f"could not remove the stale socket {path}: {error}") from error
            except OSError as error:  # busy or not ours: never remove it
                raise PaneError(f"could not check the socket {path}: {error}") from error
            else:
                raise PaneError(
                    f"a plot pane is already running on {path}; use that one, or set "
                    "GGRUST_VIEWER_SOCKET to another path for a second pane"
                )
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    previous_umask = os.umask(0o077)
    try:
        server.bind(str(path))
    except OSError as error:
        server.close()
        raise PaneError(f"could not listen on {path}: {error}") from error
    finally:
        os.umask(previous_umask)
    path.chmod(0o600)
    server.listen()
    info = path.stat()
    return server, (info.st_dev, info.st_ino)


def _unlink_if_ours(path: Path, identity: tuple[int, int]) -> None:
    """Remove the socket file at ``path`` unless another pane has replaced it."""
    try:
        info = path.lstat()
    except OSError:
        return
    if (info.st_dev, info.st_ino) == identity:
        path.unlink(missing_ok=True)


def serve(style: PaneStyle | Literal["auto"] = "auto") -> None:
    """Run the plot pane in this terminal until ``q`` (blocks).

    Parameters
    ----------
    style : {"auto", "dark", "light", "none"}, default="auto"
        How plots without a flavor of their own are dressed (see ``styled``);
        ``"auto"`` asks the terminal whether its background is dark or light.

    Raises
    ------
    PaneError
        When stdin or stdout is not a terminal, or another pane owns the socket.
    """
    if not (sys.stdin.isatty() and sys.stdout.isatty()):
        raise PaneError("the ggrust pane needs a terminal; run it in a terminal window")
    saved = termios.tcgetattr(sys.stdin.fileno())
    path = socket_path()
    if not os.environ.get("GGRUST_VIEWER_SOCKET"):  # as in ``socket_path``
        _private_directory()
    server, identity = _listen(path)
    viewer = _Viewer()
    keys = _KeyReader()
    selector = selectors.DefaultSelector()
    read_end, write_end = os.pipe()
    redraw_at: float | None = None  # re-render once resizing pauses
    try:
        selector.register(server, selectors.EVENT_READ, "socket")
        selector.register(sys.stdin, selectors.EVENT_READ, "keys")
        os.set_blocking(write_end, False)
        signal.set_wakeup_fd(write_end)
        signal.signal(signal.SIGWINCH, lambda *_: None)
        selector.register(read_end, selectors.EVENT_READ, "resize")
        # Alternate screen (no scrollback, so a resize cannot push old frames into it),
        # no line wrapping, hidden cursor: the usual setup of a full-screen terminal app.
        sys.stdout.write("\x1b[?1049h\x1b[?7l\x1b[?25l")
        tty.setcbreak(sys.stdin.fileno())
        viewer.style = _terminal_style(keys) if style == "auto" else style
        viewer.draw()
        running = True
        while running:
            timeout = None if redraw_at is None else max(redraw_at - time.monotonic(), 0.0)
            for event, _ in selector.select(timeout):
                try:
                    if event.data == "socket":
                        connection, _ = server.accept()
                        viewer.receive(connection)
                    elif event.data == "resize":
                        os.read(read_end, 64)
                        redraw_at = time.monotonic() + _RESIZE_PAUSE
                    else:
                        for key in keys.feed(os.read(sys.stdin.fileno(), 1024)):
                            running = viewer.key(key)
                            if not running:
                                break
                except MessageError as error:  # junk on the socket: say so and carry on
                    viewer.notice = f"ignored a message: {error}"
                    viewer.write_footer()
                except Exception as error:  # never let one plot take the pane down
                    viewer.notice = f"error: {type(error).__name__}: {error}"
                    viewer.write_footer()
            if redraw_at is not None and time.monotonic() >= redraw_at:
                redraw_at = None
                try:
                    viewer.draw()
                except Exception as error:
                    viewer.notice = f"error: {type(error).__name__}: {error}"
                    viewer.write_footer()
    except KeyboardInterrupt:
        pass
    finally:
        termios.tcsetattr(sys.stdin.fileno(), termios.TCSADRAIN, saved)
        sys.stdout.write("\x1b_Ga=d,q=2\x1b\\\x1b[?25h\x1b[?7h\x1b[?1049l")
        sys.stdout.flush()
        signal.set_wakeup_fd(-1)
        selector.close()
        os.close(read_end)
        os.close(write_end)
        server.close()
        _unlink_if_ours(path, identity)
        viewer.cleanup()


def request(kind: MessageKind, body: object, timeout: float = _REPLY_TIMEOUT) -> Reply | None:
    """Send one message to the running pane and wait for its reply.

    Parameters
    ----------
    kind : {"plot", "table", "text", "control"}
        What ``body`` is.
    body : object
        The plot; ``(name, frame, total_rows)``; ``(title, text)``; or ``(name, argument)``.
    timeout : float, default=120
        Seconds to wait for the pane to act (drawing a plot can take a while).

    Returns
    -------
    Reply or None
        The pane's reply; None when no pane is listening, it went away, or it did not
        answer in time.
    """
    payload = pickle.dumps((_PROTOCOL, kind, body))
    if len(payload) > _MAX_PAYLOAD:
        return Reply(error=f"the message is {len(payload):,} bytes; the limit is {_MAX_PAYLOAD:,}")
    connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    with connection:
        try:
            connection.connect(str(socket_path()))
        except OSError:  # no pane
            return None
        with contextlib.suppress(OSError):  # the pane may refuse it unread: read its reply
            connection.sendall(struct.pack(">Q", len(payload)) + payload)
        try:
            connection.settimeout(timeout)
            header = _recv_exact(connection, 8)
            if header is None:
                return None
            (length,) = struct.unpack(">Q", header)
            answer = _recv_exact(connection, length)
        except OSError:  # no pane, it went away, or it took too long
            return None
    if answer is None:
        return None
    try:
        reply = pickle.loads(answer)
    except Exception:  # a pane of another version
        return None
    return reply if isinstance(reply, Reply) else None


def _recv_exact(connection: socket.socket, length: int) -> bytes | None:
    """``length`` bytes from a socket with a timeout set, or None if it closes first."""
    chunks: list[bytes] = []
    while length:
        chunk = connection.recv(min(length, 1 << 20))
        if not chunk:
            return None
        chunks.append(chunk)
        length -= len(chunk)
    return b"".join(chunks)


def send(plot: PlotSpec | SupPlotsSpec) -> bool:
    """Send a plot to a running pane, which renders it at its pane size.

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The plot or grid of plots (with its data, so the pane can re-render it).

    Returns
    -------
    bool
        True when a pane took it, False when no pane is listening.
    """
    return request("plot", plot) is not None


_STARTUP = """\
from ggrust import *
from ggrust.data import *
from ggrust._pane_repl import interact as _interact, start as _start
pane = _start()
del _start
_interact()  # only without the new REPL: runs one whose blocks get dividers
del _interact
"""


def _write_private(path: Path, text: str) -> Path:
    """Write ``text`` to ``path`` with mode 0600, replacing (never following) what is there."""
    descriptor, temporary = tempfile.mkstemp(dir=path.parent, prefix=".tmp-", suffix=path.suffix)
    with os.fdopen(descriptor, "w", encoding="utf-8") as file:
        file.write(text)
    os.replace(temporary, path)
    return path


def _applescript_string(text: str) -> str:
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def workspace() -> None:
    """Open a Ghostty window: Python REPL on the left, plot pane on the right (macOS).

    Each window gets its own socket, so two workspaces never share a pane.

    Raises
    ------
    PaneError
        When the private runtime directory is unsafe (see ``_private_directory``).
    """
    directory_path = _private_directory()
    startup = _write_private(directory_path / "startup.py", _STARTUP)
    python = shlex.quote(sys.executable)
    repl = _applescript_string(f"{python} -q")
    pane = _applescript_string(f"{python} -m ggrust._viewer")
    directory = _applescript_string(os.getcwd())
    window_socket = directory_path / f"pane-{secrets.token_hex(4)}.sock"
    shared = [f"GGRUST_VIEWER_SOCKET={window_socket}"]
    left_environment = ", ".join(
        _applescript_string(item) for item in [f"PYTHONSTARTUP={startup}", *shared]
    )
    right_environment = ", ".join(_applescript_string(item) for item in shared)
    script = f"""
tell application "Ghostty"
    set left_config to new surface configuration
    set initial working directory of left_config to {directory}
    set command of left_config to {repl}
    set environment variables of left_config to {{{left_environment}}}
    set right_config to new surface configuration
    set initial working directory of right_config to {directory}
    set command of right_config to {pane}
    set environment variables of right_config to {{{right_environment}}}
    set new_window to new window with configuration left_config
    set left_terminal to focused terminal of selected tab of new_window
    split left_terminal direction right with configuration right_config
    focus left_terminal
end tell
"""
    subprocess.run(["osascript", "-e", script], check=True)


_USAGE = """\
usage: ggrust [pane [auto|dark|light|none]]

  ggrust         open a Ghostty window: Python REPL left, plot pane right (macOS)
  ggrust pane    run only the plot pane in this terminal; plots typed in a REPL
                 with the same GGRUST_VIEWER_SOCKET show in it. The style picks how
                 plots without a flavor of their own look (default: auto, which
                 asks the terminal whether its background is dark or light)."""

_STYLES = ("auto", "dark", "light", "none")


def main(arguments: list[str] | None = None) -> None:
    """Run the ``ggrust`` command (see ``_USAGE``).

    Parameters
    ----------
    arguments : list of str or None, default=None
        The command-line arguments; ``sys.argv[1:]`` when None.
    """
    arguments = sys.argv[1:] if arguments is None else arguments
    if not arguments:
        if sys.platform != "darwin":
            sys.exit("ggrust: the REPL window needs Ghostty on macOS; run 'ggrust pane' instead")
        try:
            workspace()
        except PaneError as error:
            sys.exit(f"ggrust: {error}")
        except (OSError, subprocess.CalledProcessError) as error:
            sys.exit(f"ggrust: could not open a Ghostty window ({error}); is Ghostty installed?")
    elif arguments[0] in ("-h", "--help"):
        print(_USAGE)
    elif arguments[0] == "pane" and len(arguments) <= 2:
        style = arguments[1] if len(arguments) == 2 else "auto"
        if style not in _STYLES:
            sys.exit(f"ggrust: unknown pane style {style!r}; use one of {', '.join(_STYLES)}")
        try:
            serve(style)
        except PaneError as error:
            sys.exit(f"ggrust: {error}")
    else:
        sys.exit(f"ggrust: unknown arguments {' '.join(arguments)!r}\n\n{_USAGE}")


if __name__ == "__main__":
    # ``python -m ggrust._viewer [style]`` is what the workspace's right pane runs.
    # Run from the imported module, not this ``__main__`` copy, so the replies it
    # pickles (``Reply``) name ``ggrust._viewer`` and the REPL can unpickle them.
    from ggrust import _viewer

    _viewer.main(["pane", *sys.argv[1:]])
