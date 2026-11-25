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
        self.index = -1
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
        else:
            try:
                png = self.render(width, height)
            except Exception as error:  # show any render failure in the pane
                out.write(f"could not draw plot {self.index + 1}: {error}\r\n")
            else:
                out.write(_fit_escape(png, rows, columns))
            footer = (
                f" plot {self.index + 1}/{len(self.history)}   style {self.style}"
                "   <- -> history   q quit"
            )
            out.write(f"\x1b[{rows + 1};1H\x1b[2m{footer[:columns]}\x1b[0m")
        out.flush()

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
        self.history.append(pickle.loads(payload))
        self.index = len(self.history) - 1
        self.draw()

    def key(self, key: str) -> bool:
        """Handle a key; False means quit."""
        if key == "q":
            return False
        if key in ("\x1b[D", "h") and self.index > 0:
            self.index -= 1
            self.draw()
        elif key in ("\x1b[C", "l") and self.index < len(self.history) - 1:
            self.index += 1
            self.draw()
        return True


def _png_size(png: bytes) -> tuple[int, int]:
    """Width and height from a PNG's IHDR chunk."""
    width, height = struct.unpack(">II", png[16:24])
    return width, height


def _fit_escape(png: bytes, rows: int, columns: int) -> str:
    """Kitty escapes drawing ``png`` as large as fits in ``rows`` x ``columns`` cells."""
    _, _, pane_width, pane_height = _plot_area()
    cell_width = pane_width / max(columns, 1)
    cell_height = pane_height / max(rows, 1)
    if cell_width <= 0 or cell_height <= 0:
        return kitty_escape(png, columns, move_cursor=False)
    width, height = _png_size(png)
    # Largest scale that fits both ways, in whole cells, never past the pane.
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
    sys.stdout.write("\x1b[?25l")  # hide the cursor
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
        sys.stdout.write("\x1b_Ga=d,q=2\x1b\\\x1b[?25h\x1b[2J\x1b[H")
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


if __name__ == "__main__":
    arguments = sys.argv[1:]
    if arguments == ["workspace"]:
        workspace()
    elif not arguments or arguments[0] in ("auto", "dark", "light", "none"):
        serve(arguments[0] if arguments else "auto")  # ty: ignore[invalid-argument-type]
    else:
        sys.exit(
            f"unknown argument {arguments[0]!r}; use 'workspace', or a pane style: "
            "'auto' (default), 'dark', 'light' or 'none'"
        )
