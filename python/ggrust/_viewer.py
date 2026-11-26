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
user can open. Does not: work without
Kitty graphics, or handle tmux. Not part of the public API: a prototype on its own branch.
"""

from __future__ import annotations

import fcntl
import io
import os
import pickle
import re
import select
import selectors
import signal
import socket
import struct
import subprocess
import sys
import tempfile
import termios
import time
import tty
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


def socket_path() -> Path:
    """Where the viewer listens: ``$GGRUST_VIEWER_SOCKET``, else a per-user temp file.

    Returns
    -------
    Path
        The Unix socket path.
    """
    override = os.environ.get("GGRUST_VIEWER_SOCKET")
    if override:
        return Path(override)
    return Path(tempfile.gettempdir()) / f"ggrust-viewer-{os.getuid()}.sock"


def _pane_size() -> tuple[int, int, int, int]:
    """Rows, columns, width and height in pixels of the terminal on stdout."""
    packed = fcntl.ioctl(sys.stdout.fileno(), termios.TIOCGWINSZ, b"\0" * 8)
    rows, columns, width, height = struct.unpack("HHHH", packed)
    return rows, columns, width, height


def _plot_area() -> tuple[int, int, int, int]:
    """Rows, columns and pixel size of the pane above the footer."""
    rows, columns, width, height = _pane_size()
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
            out.write(f"could not draw plot {self.index + 1}: {error}")
        else:
            out.write(_fit_escape(png, inner_rows, inner_columns, cell_width, cell_height))
        self.write_footer()

    def write_footer(self) -> None:
        """Write the footer row: a pending notice, else the style and the keys."""
        rows, columns, _, _ = _plot_area()
        footer = self.notice or f"style {self.style}   <- -> history   o browser   q quit"
        text = f" {footer}"[:columns]
        sys.stdout.write(f"\x1b[{rows + 1};1H\x1b[2K\x1b[2m{text}\x1b[0m")
        sys.stdout.flush()
        self.notice = ""

    def open_in_browser(self) -> None:
        """Save the current plot as interactive HTML and open it in the default browser."""
        if self.index < 0:
            return
        path = Path(tempfile.gettempdir()) / f"ggrust-plot-{os.getpid()}-{self.index + 1}.html"
        try:
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

    def receive(self, connection: socket.socket) -> None:
        with connection:
            header = _read_exact(connection, 8)
            if header is None:
                return
            (length,) = struct.unpack(">Q", header)
            payload = _read_exact(connection, length)
            if payload is None:
                return
        # The socket is the user's own (mode 0600), so the payload is trusted.
        plot = pickle.loads(payload)
        self.history.append(plot)
        self.titles.append(_title(plot))
        self.index = len(self.history) - 1
        self.draw()

    def key(self, key: str) -> bool:
        """Handle a key; False means quit."""
        if key == "q":
            return False
        if key == "o":
            self.open_in_browser()
        if key in ("\x1b[D", "h") and self.index > 0:
            self.index -= 1
            self.draw()
        elif key in ("\x1b[C", "l") and self.index < len(self.history) - 1:
            self.index += 1
            self.draw()
        return True


def _title(plot: PlotSpec | SupPlotsSpec) -> str:
    """Return the plot's ``ggtitle`` text on one line, or "" when it has none."""
    title = (plot.as_dict().get("ggtitle") or {}).get("text")
    return " ".join(str(title).split()) if title else ""


def _png_size(png: bytes) -> tuple[int, int]:
    """Width and height from a PNG's IHDR chunk."""
    width, height = struct.unpack(">II", png[16:24])
    return width, height


def _frame(rows: int, columns: int, title: str) -> str:
    """Draw a dim rounded box over ``rows`` x ``columns`` cells, ``title`` in its top edge."""
    room = max(columns - 4, 0)
    label = f" {title} "
    if len(label) > room:  # cut long titles, marking the cut
        label = label[: max(room - 2, 0)] + "\u2026 "
    top = "\u256d\u2500" + label + "\u2500" * (columns - 3 - len(label)) + "\u256e"
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


def _read_exact(connection: socket.socket, length: int) -> bytes | None:
    """``length`` bytes from ``connection``, or None if it closes first."""
    chunks: list[bytes] = []
    remaining = length
    while remaining:
        chunk = connection.recv(min(remaining, 1 << 20))
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


def _terminal_style(timeout: float = 0.3) -> PaneStyle:
    """Ask the terminal for its background colour (OSC 11) and pick a style.

    Needs stdin in cbreak mode. Terminals that do not answer in ``timeout``
    seconds get ``"dark"``.
    """
    sys.stdout.write("\x1b]11;?\x1b\\")
    sys.stdout.flush()
    reply = b""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline and not reply.endswith((b"\x07", b"\x1b\\")):
        ready, _, _ = select.select([sys.stdin], [], [], max(deadline - time.monotonic(), 0))
        if not ready:
            break
        reply += os.read(sys.stdin.fileno(), 64)
    match = re.search(rb"rgb:([0-9a-fA-F]+)/([0-9a-fA-F]+)/([0-9a-fA-F]+)", reply)
    if match is None:
        return "dark"
    red, green, blue = (int(part, 16) / (16 ** len(part) - 1) for part in match.groups())
    luminance = 0.2126 * red + 0.7152 * green + 0.0722 * blue
    return "dark" if luminance < 0.5 else "light"


def serve(style: PaneStyle | Literal["auto"] = "auto") -> None:
    """Run the plot pane in this terminal until ``q`` (blocks).

    Parameters
    ----------
    style : {"auto", "dark", "light", "none"}, default="auto"
        How plots without a flavor of their own are dressed (see ``styled``);
        ``"auto"`` asks the terminal whether its background is dark or light.
    """
    path = socket_path()
    path.unlink(missing_ok=True)
    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    server.bind(str(path))
    path.chmod(0o600)
    server.listen()
    viewer = _Viewer()
    selector = selectors.DefaultSelector()
    selector.register(server, selectors.EVENT_READ, "socket")
    selector.register(sys.stdin, selectors.EVENT_READ, "keys")
    read_end, write_end = os.pipe()
    os.set_blocking(write_end, False)
    signal.set_wakeup_fd(write_end)
    signal.signal(signal.SIGWINCH, lambda *_: None)
    selector.register(read_end, selectors.EVENT_READ, "resize")
    saved = termios.tcgetattr(sys.stdin.fileno())
    # Alternate screen (no scrollback, so a resize cannot push old frames into it),
    # no line wrapping, hidden cursor: the usual setup of a full-screen terminal app.
    sys.stdout.write("\x1b[?1049h\x1b[?7l\x1b[?25l")
    redraw_at: float | None = None  # re-render once resizing pauses
    try:
        tty.setcbreak(sys.stdin.fileno())
        viewer.style = _terminal_style() if style == "auto" else style
        viewer.draw()
        running = True
        while running:
            timeout = None if redraw_at is None else max(redraw_at - time.monotonic(), 0.0)
            for event, _ in selector.select(timeout):
                if event.data == "socket":
                    connection, _ = server.accept()
                    viewer.receive(connection)
                elif event.data == "resize":
                    os.read(read_end, 64)
                    redraw_at = time.monotonic() + _RESIZE_PAUSE
                else:
                    running = viewer.key(os.read(sys.stdin.fileno(), 16).decode(errors="ignore"))
            if redraw_at is not None and time.monotonic() >= redraw_at:
                redraw_at = None
                viewer.draw()
    except KeyboardInterrupt:
        pass
    finally:
        termios.tcsetattr(sys.stdin.fileno(), termios.TCSADRAIN, saved)
        sys.stdout.write("\x1b_Ga=d,q=2\x1b\\\x1b[?25h\x1b[?7h\x1b[?1049l")
        sys.stdout.flush()
        server.close()
        path.unlink(missing_ok=True)


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
    try:
        connection = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        connection.connect(str(socket_path()))
    except OSError:
        return False
    with connection:
        connection.sendall(struct.pack(">Q", len(payload)) + payload)
    return True


def install_displayhook() -> None:
    """Send plots typed at the REPL to the viewer; everything else prints as usual."""
    from ggrust.plot.core import PlotSpec
    from ggrust.plot.subplots import SupPlotsSpec

    previous: Callable[[object], None] = sys.displayhook

    def hook(value: object) -> None:
        if isinstance(value, PlotSpec | SupPlotsSpec) and send(value):
            return
        previous(value)

    sys.displayhook = hook


_STARTUP = '''\
from ggrust import *
from ggrust.data import *
from ggrust._viewer import install_displayhook as _install
_install()
del _install
print("""ggrust REPL: plain Python. These two imports already ran for you:
    from ggrust import *        # ggplot, aes, geom_*, scale_*, theme_*, flavor_*, ...
    from ggrust.data import *   # sample data: mpg, penguins, iris, economics
A plot you type shows in the pane on the right, e.g.
    ggplot(mpg, aes('displ', 'hwy', color='class')) + geom_point()
Plots without a flavor of their own get the pane's high-contrast, transparent look.""")
'''


def _applescript_string(text: str) -> str:
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def workspace() -> None:
    """Open a Ghostty window: Python REPL on the left, plot pane on the right (macOS)."""
    startup = Path(tempfile.gettempdir()) / f"ggrust-startup-{os.getuid()}.py"
    startup.write_text(_STARTUP, encoding="utf-8")
    python = sys.executable
    repl = _applescript_string(f"{python} -q")
    pane = _applescript_string(f"{python} -m ggrust._viewer")
    directory = _applescript_string(os.getcwd())
    shared = [f"GGRUST_VIEWER_SOCKET={socket_path()}"]
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
        except (OSError, subprocess.CalledProcessError) as error:
            sys.exit(f"ggrust: could not open a Ghostty window ({error}); is Ghostty installed?")
    elif arguments[0] in ("-h", "--help"):
        print(_USAGE)
    elif arguments[0] == "pane" and len(arguments) <= 2:
        style = arguments[1] if len(arguments) == 2 else "auto"
        if style not in _STYLES:
            sys.exit(f"ggrust: unknown pane style {style!r}; use one of {', '.join(_STYLES)}")
        serve(style)
    else:
        sys.exit(f"ggrust: unknown arguments {' '.join(arguments)!r}\n\n{_USAGE}")


if __name__ == "__main__":
    # ``python -m ggrust._viewer [style]`` is what the workspace's right pane runs.
    main(["pane", *sys.argv[1:]])
