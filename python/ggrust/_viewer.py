"""Prototype: an RStudio-style plot pane for the terminal.

A viewer process (``python -m ggrust._viewer``) owns a terminal pane and draws
every plot sent to it, rendered to fill the pane (and re-rendered when the pane
is resized), with the Kitty graphics protocol
(Ghostty, Kitty, WezTerm). A REPL in another pane sends plots over a Unix socket:
``install_displayhook()`` makes a bare plot expression go to the pane. Left and
right arrows (or ``h`` / ``l``) page through the plot history; ``q`` quits.

``python -m ggrust._viewer workspace`` opens a Ghostty window (AppleScript, macOS)
with a Python REPL on the left and the viewer on the right.

Plots that set no flavor of their own get ``flavor_high_contrast_dark()`` (or the
light one, after asking the terminal for its background) and ``flavor_transparent()``,
so they sit on the terminal's background. Plots travel pickled over a socket only the
user can open (in a 0700 directory of the user's own). Does not: work without
Kitty graphics, or handle tmux. Not part of the public API: a prototype on its own branch.
"""

from __future__ import annotations

import builtins
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
import time
import tty
import unicodedata
import warnings
import webbrowser
from pathlib import Path
from typing import TYPE_CHECKING, Literal

from ggrust._terminal import kitty_escape

if TYPE_CHECKING:
    from collections.abc import Callable

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


class PaneError(RuntimeError):
    """The plot pane cannot start: no terminal, a live pane on the socket, or a bad path."""


class MessageError(ValueError):
    """A message on the pane's socket is not a plot: too big, too slow, cut short or junk."""


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
    packed = fcntl.ioctl(sys.stdout.fileno(), termios.TIOCGWINSZ, b"\0" * 8)
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
    """The pane: plot history, rendering at the pane's size, and drawing."""

    def __init__(self, style: PaneStyle = "none") -> None:
        self.history: list[PlotSpec | SupPlotsSpec] = []
        self.titles: list[str] = []  # each plot's ggtitle, "" when it has none
        self.index = -1
        self.notice = ""  # one-off message shown in the footer
        self.cache: tuple[int, int, int, bytes] | None = None  # index, width, height, png
        self.style = style
        self.html_directory: Path | None = None  # made on the first ``o``, removed at exit

    def render(self, width: int, height: int) -> bytes:
        """Render the current plot as PNG bytes of ``width`` x ``height`` pixels (cached)."""
        if self.cache is not None and self.cache[:3] == (self.index, width, height):
            return self.cache[3]
        buffer = io.BytesIO()
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            styled(self.history[self.index], self.style).to_png(
                buffer, w=width / _SCALE, h=height / _SCALE, unit="px", scale=_SCALE
            )
        png = buffer.getvalue()
        self.cache = (self.index, width, height, png)
        return png

    def draw(self) -> None:
        """Clear the pane and draw the current plot in its frame, then the footer."""
        out = sys.stdout
        out.write("\x1b_Ga=d,q=2\x1b\\\x1b[2J\x1b[H")  # delete images, clear, home
        rows, columns, width, height = _plot_area()
        if self.index < 0:
            out.write("ggrust plots: waiting for a plot...\r\n")
            out.flush()
            return
        cell_width = width / max(columns, 1)
        cell_height = height / max(rows, 1)
        framed = rows > 2 * _FRAME and columns > 2 * _FRAME
        inner_rows, inner_columns = rows, columns
        if framed:
            label = f"plot {self.index + 1}/{len(self.history)}"
            if self.titles[self.index]:
                label += f" \u00b7 {self.titles[self.index]}"
            out.write(_frame(rows, columns, label))
            inner_rows, inner_columns = rows - 2 * _FRAME, columns - 2 * _FRAME
            out.write(f"\x1b[{1 + _FRAME};{1 + _FRAME}H")
        try:
            png = self.render(round(inner_columns * cell_width), round(inner_rows * cell_height))
        except Exception as error:  # show any render failure in the pane
            out.write(
                _cut(_clean_label(f"could not draw plot {self.index + 1}: {error}"), inner_columns)
            )
        else:
            out.write(_fit_escape(png, inner_rows, inner_columns, cell_width, cell_height))
        self.write_footer()

    def write_footer(self) -> None:
        """Write the footer row: a pending notice, else the style and the keys."""
        rows, columns, _, _ = _plot_area()
        footer = self.notice or f"style {self.style}   <- -> history   o browser   q quit"
        text = _cut(" " + _clean_label(footer), columns)
        sys.stdout.write(f"\x1b[{rows + 1};1H\x1b[2K\x1b[2m{text}\x1b[0m")
        sys.stdout.flush()
        self.notice = ""

    def open_in_browser(self) -> None:
        """Save the current plot as interactive HTML and open it in the default browser."""
        if self.index < 0:
            return
        try:
            if self.html_directory is None:
                self.html_directory = Path(tempfile.mkdtemp(prefix="ggrust-pane-"))
            path = self.html_directory / f"plot-{self.index + 1}.html"
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                self.history[self.index].to_html(path)  # as sent: browsers have a light page
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
        """Read one plot from ``connection``, add it to the history and draw it.

        A connection that closes before sending anything (a probe) is ignored.

        Raises
        ------
        MessageError
            When the message is too big, too slow, cut short, or not a plot.
        """
        deadline = time.monotonic() + _RECEIVE_TIMEOUT
        with connection:
            header = _read_exact(connection, 8, deadline)
            if header is None:
                return
            (length,) = struct.unpack(">Q", header)
            if length > _MAX_PAYLOAD:
                raise MessageError(f"refused a {length}-byte message; the limit is {_MAX_PAYLOAD}")
            payload = _read_exact(connection, length, deadline)
            if payload is None:
                raise MessageError(f"the sender hung up before sending all {length} bytes")
        plot = _load_plot(payload)
        try:
            title = _title(plot)
        except Exception as error:  # a plot whose spec cannot be read
            raise MessageError(f"could not read the plot's spec: {error}") from error
        self.history.append(plot)
        self.titles.append(title)
        self.index = len(self.history) - 1
        self.draw()

    def key(self, key: str) -> bool:
        """Handle a key; False means quit."""
        if key == "q":
            return False
        if key == "o":
            self.open_in_browser()
        if key in ("\x1b[D", "\x1bOD", "h") and self.index > 0:
            self.index -= 1
            self.draw()
        elif key in ("\x1b[C", "\x1bOC", "l") and self.index < len(self.history) - 1:
            self.index += 1
            self.draw()
        return True


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


def _title(plot: PlotSpec | SupPlotsSpec) -> str:
    """Return the plot's ``ggtitle`` text on one line, or "" when it has none."""
    title = (plot.as_dict().get("ggtitle") or {}).get("text")
    return _clean_label(str(title)) if title else ""


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


def _cell_width(character: str) -> int:
    """Terminal cells ``character`` takes: 2 for wide and full-width, 0 for combining marks."""
    if unicodedata.combining(character) or unicodedata.category(character) in ("Mn", "Me", "Cf"):
        return 0
    return 2 if unicodedata.east_asian_width(character) in ("W", "F") else 1


def _cells(text: str) -> int:
    """Terminal cells ``text`` takes."""
    return sum(_cell_width(character) for character in text)


def _cut(text: str, cells: int) -> str:
    """Return the longest start of ``text`` that fits in ``cells`` terminal cells."""
    used = 0
    for index, character in enumerate(text):
        used += _cell_width(character)
        if used > cells:
            return text[:index]
    return text


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


def send(plot: PlotSpec | SupPlotsSpec) -> bool:
    """Send a plot to a running viewer, which renders it at its pane size.

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The plot or grid of plots (with its data, so the pane can re-render it).

    Returns
    -------
    bool
        True when a viewer took it, False when no viewer is listening.
    """
    payload = pickle.dumps(plot)
    connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    with connection:
        try:
            connection.connect(str(socket_path()))
            connection.sendall(struct.pack(">Q", len(payload)) + payload)
        except OSError:  # no pane, or it went away while we were sending
            return False
    return True


def install_displayhook() -> None:
    """Send plots typed at the REPL to the viewer; everything else prints as usual."""
    from ggrust.plot.core import PlotSpec
    from ggrust.plot.subplots import SupPlotsSpec

    previous: Callable[[object], None] = sys.displayhook

    def hook(value: object) -> None:
        if not isinstance(value, PlotSpec | SupPlotsSpec):
            previous(value)
            return
        setattr(builtins, "_", value)  # noqa: B010  # like the default hook: ``_`` is the last plot
        if send(value):
            print(_dim("  \u21b3 sent to the plot pane"))
        else:
            print(_dim("  no plot pane is running; start one with 'ggrust pane' in another"))
            print(_dim("  terminal, or save the plot with ggsave(_, 'plot.png')"))

    sys.displayhook = hook


_STARTUP = """\
from ggrust import *
from ggrust.data import *
from ggrust._viewer import install_displayhook as _install, repl_banner as _banner
_install()
print(_banner())
del _install, _banner
"""


def _color(code: str, text: str) -> str:
    """Wrap ``text`` in an SGR color ``code`` unless NO_COLOR is set or stdout is no tty."""
    if os.environ.get("NO_COLOR") or not sys.stdout.isatty():
        return text
    return f"\x1b[{code}m{text}\x1b[0m"


def _dim(text: str) -> str:
    return _color("2", text)


def repl_banner() -> str:
    """Return the REPL's welcome text: what was imported, the data, and how to start.

    Starts by clearing the screen, so the shell's login line does not show.

    Returns
    -------
    str
        The banner, with colors unless ``NO_COLOR`` is set or stdout is no terminal.
    """
    from ggrust import __version__

    def row(label: str, text: str) -> str:
        return f"  {_dim(f'{label:<9}')} {text}"

    clear = "\x1b[H\x1b[2J" if sys.stdout.isatty() else ""
    code = lambda text: _color("36", text)  # noqa: E731
    lines = [
        clear
        + _color("1", "ggrust")
        + _dim(f" {__version__} \u00b7 plots show in the pane \u2192"),
        "",
        row("imported", code("from ggrust import *")),
        row("", code("from ggrust.data import *")),
        row("data", "mpg, penguins, iris, economics"),
        row("style", "high contrast unless a flavor is set"),
        row("try", ""),
        "    " + code("ggplot(mpg, aes('displ', 'hwy')) + geom_point()"),
        "",
    ]
    return "\n".join(lines)


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
    main(["pane", *sys.argv[1:]])
