"""The terminal plot pane prototype (``ggrust._viewer``): unit tests and pty-driven runs.

The pty tests start a real pane as a child process on a pseudo-terminal, send it
plots over its socket and read what it writes. A reader thread drains the pty
the whole time, or the pane would block writing large Kitty escapes.
"""

from __future__ import annotations

import fcntl
import itertools
import os
import pickle
import re
import socket
import stat
import struct
import subprocess
import sys
import termios
import threading
import time
from pathlib import Path
from typing import TYPE_CHECKING

import pytest

if sys.platform == "win32":  # pragma: no cover
    pytest.skip("the plot pane needs POSIX ttys and Unix sockets", allow_module_level=True)

import pty

from ggrust import _viewer
from ggrust._pane_pages import Page

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from ggrust.plot.core import PlotSpec

# The reader threads make pytest multi-threaded; each forked child only calls execve.
pytestmark = pytest.mark.filterwarnings("ignore:This process .* is multi-threaded")

_COUNTER = itertools.count()
_ROOT = Path(__file__).resolve().parent.parent


def _short_socket() -> str:
    """A short, unused socket path (AF_UNIX paths are capped at about 104 bytes)."""
    path = f"/tmp/ggr-t-{os.getpid()}-{next(_COUNTER)}.sock"
    Path(path).unlink(missing_ok=True)
    return path


def _plot(title: str | None = None) -> PlotSpec:
    from ggrust import aes, geom_point, ggplot, ggtitle

    plot = ggplot({"x": [1.0, 2.0, 3.0], "y": [3.0, 1.0, 2.0]}, aes("x", "y")) + geom_point()
    return plot + ggtitle(title) if title is not None else plot


def _wait(condition: Callable[[], bool], timeout: float = 20.0) -> bool:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if condition():
            return True
        time.sleep(0.05)
    return condition()


class _Pane:
    """A plot pane running on a pty, with a thread that keeps reading its output."""

    def __init__(
        self,
        socket_file: str | None,
        size: tuple[int, int, int, int] = (30, 100, 1000, 600),
        environment: dict[str, str] | None = None,
    ) -> None:
        env = dict(os.environ, BROWSER="true", TERM="xterm-kitty", **(environment or {}))
        env.pop("GGRUST_VIEWER_SOCKET", None)
        if socket_file is not None:
            env["GGRUST_VIEWER_SOCKET"] = socket_file
        self.socket = socket_file
        pid, fd = pty.fork()
        if pid == 0:  # child: become the pane
            try:
                os.chdir(_ROOT)
                os.execve(sys.executable, [sys.executable, "-m", "ggrust._viewer", "dark"], env)
            finally:
                os._exit(127)
        self.pid, self.fd = pid, fd
        self.resize(*size)
        self.output = b""
        self.status: int | None = None
        self.lock = threading.Lock()
        threading.Thread(target=self._drain, daemon=True).start()

    def _drain(self) -> None:
        while True:
            try:
                data = os.read(self.fd, 65536)
            except OSError:
                return
            if not data:
                return
            with self.lock:
                self.output += data

    def text(self) -> bytes:
        with self.lock:
            return self.output

    def resize(self, rows: int, columns: int, width: int, height: int) -> None:
        fcntl.ioctl(self.fd, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, width, height))

    def type(self, data: bytes) -> None:
        os.write(self.fd, data)

    def alive(self) -> bool:
        if self.status is not None:
            return False
        pid, status = os.waitpid(self.pid, os.WNOHANG)
        if pid == 0:
            return True
        self.status = status
        return False

    def ready(self) -> bool:
        return b"waiting for a plot" in self.text()

    def images(self) -> int:
        return self.text().count(b"a=T")

    def stop(self) -> None:
        if self.alive():
            os.kill(self.pid, 9)
            os.waitpid(self.pid, 0)
            self.status = -9
        os.close(self.fd)


@pytest.fixture
def pane_factory() -> Iterator[Callable[..., _Pane]]:
    panes: list[_Pane] = []

    def start(socket_file: str | None = "new", **options: object) -> _Pane:
        path = _short_socket() if socket_file == "new" else socket_file
        pane = _Pane(path, **options)  # type: ignore[arg-type]
        panes.append(pane)
        assert _wait(pane.ready), pane.text()[-500:]
        return pane

    yield start
    for pane in panes:
        pane.stop()
        if pane.socket:
            Path(pane.socket).unlink(missing_ok=True)


def _send_to(path: str, plot: object) -> bool:
    previous = os.environ.get("GGRUST_VIEWER_SOCKET")
    os.environ["GGRUST_VIEWER_SOCKET"] = path
    try:
        return _viewer.send(plot)  # type: ignore[arg-type]
    finally:
        if previous is None:
            os.environ.pop("GGRUST_VIEWER_SOCKET")
        else:
            os.environ["GGRUST_VIEWER_SOCKET"] = previous


def _raw_send(path: str, data: bytes) -> None:
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
        connection.connect(path)
        connection.sendall(data)


# --- unit tests -----------------------------------------------------------------------------


def test_clean_label_strips_control_characters() -> None:
    assert _viewer._clean_label("A\x1b]0;PWNED\x07B") == "A]0;PWNEDB"
    assert _viewer._clean_label("a\x9bb\x7fc") == "abc"  # C1 CSI and DEL
    assert _viewer._clean_label("two\n  lines") == "two lines"


def test_cells_counts_wide_characters_twice() -> None:
    assert _viewer._cells("abc") == 3
    assert _viewer._cells("東京") == 4  # Tokyo in kanji
    assert _viewer._cells("é") == 1  # combining accent
    assert _viewer._cut("東京", 3) == "東"


def _visible_lines(frame: str) -> list[str]:
    import re

    return [re.sub(r"\x1b\[[0-9;]*m", "", line) for line in re.split(r"\x1b\[\d+;1H", frame)[1:]]


@pytest.mark.parametrize("columns", [3, 4, 5, 6, 8, 12, 40])
@pytest.mark.parametrize("title", ["plot 1/1", "plot 1/1 · 東京の天気"])
def test_frame_edges_are_exactly_the_pane_width(columns: int, title: str) -> None:
    lines = _visible_lines(_viewer._frame(4, columns, title))
    assert len(lines) == 4
    for line in lines:
        assert _viewer._cells(line) == columns, (columns, line)


def test_frame_drops_escapes_from_titles() -> None:
    frame = _viewer._frame(5, 60, "plot 1/1 · A\x1b]0;PWNED\x07B")
    assert "\x1b]" not in frame
    assert "\x07" not in frame
    assert "A]0;PWNEDB" in frame


def test_split_keys_handles_batched_and_split_input() -> None:
    assert _viewer._split_keys("hq") == (["h", "q"], "")
    assert _viewer._split_keys("\x1b[D\x1b[D") == (["\x1b[D", "\x1b[D"], "")
    assert _viewer._split_keys("h\x1b") == (["h"], "\x1b")
    assert _viewer._split_keys("\x1b[") == ([], "\x1b[")
    assert _viewer._split_keys("\x1bq") == (["\x1b", "q"], "")
    reader = _viewer._KeyReader()
    assert reader.feed(b"\x1b") == []
    assert reader.feed(b"[Dl") == ["\x1b[D", "l"]


def test_alt_keys_do_not_hold_back_later_keys() -> None:
    for alt in ("\x1bX", "\x1bP", "\x1b_", "\x1b]"):
        tokens, pending = _viewer._split_keys(alt + "q")
        assert tokens[-1] == "q", alt
        assert pending == ""


def test_clean_label_strips_format_characters() -> None:
    assert _viewer._clean_label("\u202eabc") == "abc"
    assert "\u202e" not in _viewer._frame(3, 12, "\u202eabc")


def test_empty_socket_variable_uses_the_private_directory(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.setenv("GGRUST_VIEWER_SOCKET", "")
    assert _viewer.socket_path() == _viewer._runtime_directory() / "viewer.sock"


def test_listen_keeps_a_socket_it_cannot_check(monkeypatch: pytest.MonkeyPatch) -> None:
    path = Path(_short_socket())
    holder = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    holder.bind(str(path))
    holder.close()

    def refuse(self: socket.socket, address: str) -> None:
        raise PermissionError(13, "Permission denied")

    monkeypatch.setattr(socket.socket, "connect", refuse)
    try:
        with pytest.raises(_viewer.PaneError, match="could not check the socket"):
            _viewer._listen(path)
        assert path.exists()
    finally:
        path.unlink(missing_ok=True)


def test_key_reader_swallows_split_osc_replies() -> None:
    reader = _viewer._KeyReader()
    reply = b"\x1b]11;rgb:ffff/ffff/ffff\x1b\\"
    assert reader.feed(reply[:7]) == []
    tokens = reader.feed(reply[7:] + b"h")
    assert tokens == [reply.decode(), "h"]
    viewer = _viewer._Viewer()
    assert viewer.key(tokens[0]) is True  # an OSC reply is no key


def test_load_plot_rejects_junk_and_non_plots() -> None:
    with pytest.raises(_viewer.MessageError, match="not a pickled plot"):
        _viewer._load_plot(b"junk!")
    with pytest.raises(_viewer.MessageError, match="expected a PlotSpec or SupPlotsSpec, got dict"):
        _viewer._load_plot(pickle.dumps({"a": 1}))
    assert _viewer._title(_viewer._load_plot(pickle.dumps(_plot("hi")))) == "hi"


def test_open_in_browser_files_are_removed_at_cleanup(monkeypatch: pytest.MonkeyPatch) -> None:
    opened: list[str] = []
    monkeypatch.setattr(_viewer.webbrowser, "open", lambda url: opened.append(url) or True)
    monkeypatch.setattr(_viewer._Viewer, "write_footer", lambda self: None)
    viewer = _viewer._Viewer()
    viewer.pages, viewer.index = [Page("plot", "", plot=_plot())], 0
    viewer.open_in_browser()
    directory = viewer.html_directory
    assert directory is not None
    assert (directory / "plot-1.html").exists()
    assert opened == [(directory / "plot-1.html").as_uri()]
    viewer.cleanup()
    assert not directory.exists()


def test_workspace_quotes_python_and_gives_each_window_a_socket(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    scripts: list[str] = []
    runtime = Path(f"/tmp/ggr-t-{os.getpid()}-ws")
    monkeypatch.setattr(_viewer, "_runtime_directory", lambda: runtime)
    monkeypatch.setattr(_viewer.sys, "executable", "/opt/my python/bin/python3")
    monkeypatch.setattr(
        _viewer.subprocess, "run", lambda command, check: scripts.append(command[2])
    )
    try:
        _viewer.workspace()
        _viewer.workspace()
        assert "'/opt/my python/bin/python3' -q" in scripts[0]
        assert "'/opt/my python/bin/python3' -m ggrust._viewer" in scripts[0]
        sockets = [script.split("GGRUST_VIEWER_SOCKET=")[1].split('"')[0] for script in scripts]
        assert sockets[0] != sockets[1]
        assert all(len(path) < 104 for path in sockets)
        assert stat.S_IMODE(runtime.stat().st_mode) == 0o700
        startup = runtime / "startup.py"
        assert stat.S_IMODE(startup.stat().st_mode) == 0o600
        assert "from ggrust._pane_repl import" in startup.read_text()
    finally:
        for file in runtime.glob("*"):
            file.unlink()
        runtime.rmdir()


def test_private_directory_refuses_a_symlink(monkeypatch: pytest.MonkeyPatch) -> None:
    target = Path(f"/tmp/ggr-t-{os.getpid()}-target")
    link = Path(f"/tmp/ggr-t-{os.getpid()}-link")
    target.mkdir(exist_ok=True)
    link.unlink(missing_ok=True)
    link.symlink_to(target)
    monkeypatch.setattr(_viewer, "_runtime_directory", lambda: link)
    try:
        with pytest.raises(_viewer.PaneError, match="not a directory owned by you"):
            _viewer._private_directory()
    finally:
        link.unlink()
        target.rmdir()


# --- pty tests ------------------------------------------------------------------------------


def test_pane_draws_a_framed_plot(pane_factory: Callable[..., _Pane]) -> None:
    pane = pane_factory()
    assert pane.socket is not None
    assert _send_to(pane.socket, _plot("Fuel use"))
    assert _wait(lambda: pane.images() >= 1)
    assert b"plot 1/1 \xc2\xb7 Fuel use" in pane.text()
    assert b"\x1b[?1049h\x1b[?7l" in pane.text()  # alternate screen, no autowrap
    assert stat.S_IMODE(os.stat(pane.socket).st_mode) == 0o600


def test_idle_client_does_not_freeze_the_pane(pane_factory: Callable[..., _Pane]) -> None:
    pane = pane_factory()
    assert pane.socket is not None
    idle = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    idle.connect(pane.socket)
    try:
        started = time.monotonic()
        result: list[bool] = []
        sender = threading.Thread(target=lambda: result.append(_send_to(pane.socket, _plot())))
        sender.start()
        assert _wait(lambda: pane.images() >= 1, timeout=_viewer._RECEIVE_TIMEOUT + 15)
        assert time.monotonic() - started < _viewer._RECEIVE_TIMEOUT + 10
        sender.join(10)
        assert result == [True]
        assert b"timed out" in pane.text()
    finally:
        idle.close()


def test_huge_length_header_is_refused(pane_factory: Callable[..., _Pane]) -> None:
    pane = pane_factory()
    assert pane.socket is not None
    _raw_send(pane.socket, struct.pack(">Q", 2**63))
    assert _wait(lambda: b"refused a" in pane.text())
    assert _send_to(pane.socket, _plot())
    assert _wait(lambda: pane.images() >= 1)


def test_bad_payloads_do_not_kill_the_pane(pane_factory: Callable[..., _Pane]) -> None:
    pane = pane_factory()
    assert pane.socket is not None
    _raw_send(pane.socket, struct.pack(">Q", 5) + b"junk!")
    assert _wait(lambda: b"not a pickled plot" in pane.text())
    payload = pickle.dumps({"not": "a plot"})
    _raw_send(pane.socket, struct.pack(">Q", len(payload)) + payload)
    assert _wait(lambda: b"got dict" in pane.text())
    _raw_send(pane.socket, struct.pack(">Q", 100) + b"short")  # hangs up mid-message
    assert _wait(lambda: b"hung up" in pane.text())
    assert pane.alive()
    assert _send_to(pane.socket, _plot())
    assert _wait(lambda: pane.images() >= 1)
    assert pane.alive()


def test_title_escapes_do_not_reach_the_terminal(pane_factory: Callable[..., _Pane]) -> None:
    pane = pane_factory()
    assert pane.socket is not None
    assert _send_to(pane.socket, _plot("A\x1b]0;PWNED\x07B"))
    assert _wait(lambda: pane.images() >= 1)
    assert b"]0;PWNEDB" in pane.text()
    assert b"\x1b]0;PWNED" not in pane.text()


def test_batched_and_split_keys_page_the_history(pane_factory: Callable[..., _Pane]) -> None:
    pane = pane_factory()
    assert pane.socket is not None
    for number in range(3):
        assert _send_to(pane.socket, _plot(f"p{number}"))
    assert _wait(lambda: b"plot 3/3" in pane.text())

    def after(action: bytes, marker: bytes) -> None:
        start = len(pane.text())
        pane.type(action)
        assert _wait(lambda: marker in pane.text()[start:]), (action, pane.text()[start:][-300:])

    after(b"hh", b"plot 1/3")
    after(b"\x1b[C\x1b[C", b"plot 3/3")
    pane.type(b"\x1b")  # an arrow split across two reads
    time.sleep(0.3)
    after(b"[D", b"plot 2/3")
    pane.type(b"\x1b]11;rgb:0000/0000/0000\x07")  # a late OSC 11 reply is no key
    after(b"lq", b"plot 3/3")
    assert _wait(lambda: not pane.alive())


def test_pane_renders_without_a_pixel_size(pane_factory: Callable[..., _Pane]) -> None:
    pane = pane_factory(size=(30, 100, 0, 0))
    assert pane.socket is not None
    assert _send_to(pane.socket, _plot())
    assert _wait(lambda: pane.images() >= 1)
    assert b"could not draw" not in pane.text()


def test_second_pane_refuses_a_live_socket(pane_factory: Callable[..., _Pane]) -> None:
    first = pane_factory()
    assert first.socket is not None
    second = _Pane(first.socket)
    try:
        assert _wait(lambda: not second.alive())
        assert b"already running" in second.text()
    finally:
        second.stop()
    assert _send_to(first.socket, _plot())  # the first pane still has its socket
    assert _wait(lambda: first.images() >= 1)


def test_stale_socket_is_replaced(pane_factory: Callable[..., _Pane]) -> None:
    path = _short_socket()
    stale = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    stale.bind(path)
    stale.close()  # leaves the file, with nobody listening
    pane = pane_factory(path)
    assert _send_to(path, _plot())
    assert _wait(lambda: pane.images() >= 1)


def test_quitting_pane_leaves_a_newer_panes_socket(pane_factory: Callable[..., _Pane]) -> None:
    old = pane_factory()
    assert old.socket is not None
    Path(old.socket).unlink()  # as if someone else took the path over
    new = pane_factory(old.socket)
    old.type(b"q")
    assert _wait(lambda: not old.alive())
    assert Path(old.socket).exists()
    assert _send_to(old.socket, _plot())
    assert _wait(lambda: new.images() >= 1)


def test_default_socket_lives_in_a_private_directory(
    pane_factory: Callable[..., _Pane],
) -> None:
    temporary = Path(f"/tmp/ggr-t-{os.getpid()}-tmpdir")
    temporary.mkdir(exist_ok=True)
    try:
        pane = pane_factory(None, environment={"TMPDIR": str(temporary)})
        directory = temporary / f"ggrust-{os.getuid()}"
        assert stat.S_IMODE(directory.stat().st_mode) == 0o700
        assert stat.S_IMODE((directory / "viewer.sock").stat().st_mode) == 0o600
        pane.type(b"q")
        assert _wait(lambda: not pane.alive())
        assert not (directory / "viewer.sock").exists()
    finally:
        for file in temporary.rglob("*"):
            if not file.is_dir():
                file.unlink()
        for folder in sorted(temporary.rglob("*"), reverse=True):
            folder.rmdir()
        temporary.rmdir()


def test_pane_without_a_terminal_fails_cleanly() -> None:
    path = _short_socket()
    result = subprocess.run(
        [sys.executable, "-m", "ggrust._viewer", "dark"],
        stdin=subprocess.DEVNULL,
        capture_output=True,
        text=True,
        env=dict(os.environ, GGRUST_VIEWER_SOCKET=path),
        cwd=_ROOT,
        timeout=60,
        check=False,
    )
    assert result.returncode != 0
    assert "needs a terminal" in result.stderr
    assert not Path(path).exists()


# --- pages, messages and the REPL side ------------------------------------------

_SGR = re.compile(r"\x1b\[[0-9;]*m")


def _plain(line: str) -> str:
    return _SGR.sub("", line)


def _request(path: str, kind: str, body: object) -> _viewer.Reply | None:
    previous = os.environ.get("GGRUST_VIEWER_SOCKET")
    os.environ["GGRUST_VIEWER_SOCKET"] = path
    try:
        return _viewer.request(kind, body, timeout=20)  # type: ignore[arg-type]
    finally:
        if previous is None:
            os.environ.pop("GGRUST_VIEWER_SOCKET")
        else:
            os.environ["GGRUST_VIEWER_SOCKET"] = previous


def test_table_lines_show_names_types_and_rows_within_the_width() -> None:
    import polars as pl
    from ggrust._pane_pages import table_lines

    frame = pl.DataFrame({"name": ["a", "bb", None], "value": [2.0, 0.5, 3.25], "n": [1, 2, 3]})
    page = Page("table", "frame", frame=frame, total_rows=3)
    lines = [_plain(line) for line in table_lines(page, 10, 40)]
    assert lines[0].split() == ["name", "value", "n"]
    assert lines[1].split() == ["str", "f64", "i64"]
    assert lines[3].split() == ["1", "a", "2.0", "1"]  # floats keep their decimal point
    assert "null" in lines[5]
    assert all(_viewer._cells(line) <= 40 for line in lines)
    narrow = [_plain(line) for line in table_lines(page, 10, 12)]
    assert all(_viewer._cells(line) <= 12 for line in narrow)


def test_table_columns_scroll_from_left() -> None:
    import polars as pl
    from ggrust._pane_pages import table_lines, visible_columns

    frame = pl.DataFrame({f"column_{index}": [index] for index in range(10)})
    page = Page("table", "wide", frame=frame, total_rows=1)
    shown = visible_columns(page, 40)
    assert shown.start == 0 and 0 < len(shown) < 10
    page.left = 5
    assert _plain(table_lines(page, 4, 40)[0]).split()[0] == "column_5"


def test_help_page_reflows_and_styles_a_numpy_docstring() -> None:
    from ggrust import geom_point
    from ggrust._pane_pages import help_text, text_lines

    title, text = help_text(geom_point)
    assert title == "geom_point"
    assert text.startswith("geom_point(mapping=None, *, data=None")  # no type hints
    lines = text_lines(text, 60)
    plain = [_plain(line) for line in lines]
    assert "Parameters" in plain and "----------" not in plain
    assert "\x1b[1mParameters" in lines[plain.index("Parameters")]
    assert any(line.startswith("mapping : ") for line in plain)
    assert all(_viewer._cells(line) <= 60 for line in plain)
    assert "\x1b" not in "".join(_plain(line) for line in text_lines("a\x1b]0;x\x07b", 20))


def test_open_call_finds_the_innermost_open_call() -> None:
    from ggrust._pane_repl import _open_call

    assert _open_call("geom_point(") == "geom_point"
    assert _open_call("ggplot(mpg, aes('x', 'y')) + geom_point(size=2, ") == "geom_point"
    assert _open_call("ggplot(mpg, aes('x', ") == "aes"
    assert _open_call("x = [1, ") is None
    assert _open_call("print(1)") is None


def test_completer_offers_keywords_of_the_open_call() -> None:
    from ggrust import geom_point
    from ggrust._pane_repl import _Completer

    namespace: dict[str, object] = {"geom_point": geom_point, "sample": 1}
    line = ""
    completer = _Completer(namespace, lambda: line)

    def matches(text: str) -> list[str]:
        found: list[str] = []
        while (match := completer.complete(text, len(found))) is not None:
            found.append(match)
        return found

    line = "geom_point(s"
    assert set(matches("s")) == {"stat=", "show_legend=", "sampling=", "size_unit="}
    line = "geom_point(sa"
    assert matches("sa") == ["sampling="]
    line = "geom_point(sam"
    assert "sample" not in matches("sam")  # a matching keyword wins
    line = "geom_point(x, sampl"
    assert matches("sampl") == ["sampling="]
    line = "samp"
    assert matches("samp") == ["sample"]  # outside a call: names as usual


def test_controls_change_the_pane_and_last_skips_failed_plots() -> None:
    viewer = _viewer._Viewer("dark")
    good = _plot("good")
    viewer.handle("plot", good)
    bad = ggplot_bad = _plot() + __import__("ggrust").aes(y="missing")
    reply = viewer.handle("plot", ggplot_bad)
    assert reply.error is not None and "missing" in reply.error
    assert reply.index == 2 and reply.total == 2
    assert viewer.handle("control", ("last", None)).value is good  # plot 2 failed
    del bad
    viewer.handle("control", ("style", "light"))
    assert viewer.style == "light"
    viewer.handle("control", ("size", (400, 300)))
    assert viewer.fixed_size == (400.0, 300.0)
    state = viewer.handle("control", ("state", None)).value
    assert isinstance(state, dict) and state["size"] == (400.0, 300.0)
    viewer.handle("control", ("size", None))
    assert viewer.fixed_size is None
    viewer.handle("control", ("clear", None))
    assert viewer.pages == [] and viewer.index == -1
    for body in [("style", "neon"), ("size", (0, 1)), ("nope", None), "junk"]:
        with pytest.raises(_viewer.MessageError):
            viewer.handle("control", body)


def test_pane_replies_with_page_numbers_and_shows_tables_and_help(
    pane_factory: Callable[..., _Pane],
) -> None:
    import polars as pl

    pane = pane_factory()
    assert pane.socket is not None
    first = _request(pane.socket, "plot", _plot("first"))
    assert first is not None and (first.index, first.total, first.error) == (1, 1, None)
    frame = pl.DataFrame({"species": ["Adelie", "Gentoo"], "mass": [3700.0, 5000.0]})
    table = _request(pane.socket, "table", ("penguins", frame, 344))
    assert table is not None and table.index == 2
    assert _wait(lambda: b"data 2/2" in pane.text() and b"Adelie" in pane.text())
    assert _wait(lambda: b"344" in pane.text())
    help_page = _request(pane.socket, "text", ("geom_point", "geom_point()\n\nScatter points."))
    assert help_page is not None and help_page.index == 3
    assert _wait(lambda: b"help 3/3" in pane.text())
    pane.type(b"h")  # back to the table, then scroll it
    pane.type(b"j")
    assert _wait(lambda: b"rows 2-" in pane.text())
    refused = _request(pane.socket, "control", ("bogus", None))
    assert refused is not None and refused.error is not None and "unknown control" in refused.error
    assert pane.alive()


def test_divider_shows_number_failure_and_time_at_full_width(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from ggrust._pane_repl import divider

    monkeypatch.setenv("NO_COLOR", "1")
    line = divider(3, 0.24, failed=False, columns=40)
    assert line.startswith("╰─ 3 ") and line.endswith(" 0.24s ─")
    assert len(line) == 40
    assert "✗" in divider(4, 75.0, failed=True, columns=40)
    assert "1m 15s" in divider(4, 75.0, failed=True, columns=40)


def test_blocks_time_repl_code_only_and_skip_empty_lines(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    from ggrust._pane_repl import _Blocks

    monkeypatch.setenv("NO_COLOR", "1")
    blocks = _Blocks()
    blocks.audit("exec", (compile("x = 1", "startup.py", "exec"),))  # not typed: no block
    blocks.finish()
    blocks.audit("exec", (compile("pass", "<console>", "single"),))  # an empty line
    blocks.finish()
    assert capsys.readouterr().out == ""
    blocks.audit("exec", (compile("x = 1", "<python-input-0>", "single"),))
    blocks.audit("open", ("file", "r", 0))
    blocks.finish()
    assert "╰─ 1 " in capsys.readouterr().out
    blocks.finish()  # nothing ran since: no second divider
    assert capsys.readouterr().out == ""
