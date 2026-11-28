"""Prototype: the REPL side of the terminal workspace (the left pane of ``ggrust``).

``start()`` runs from the REPL's startup file. It sends plots you type to the pane
and says where they went (or why they could not be drawn), completes keyword
arguments on Tab, prints the banner, and returns the ``pane`` object: workspace
controls (``pane.show``, ``pane.help``, ``pane.last``, ``pane.save``, ``pane.style``,
``pane.size``, ``pane.clear``). These exist only in the workspace REPL; nothing is
added to ``from ggrust import *``. Not part of the public API: a prototype on its
own branch.
"""

from __future__ import annotations

import builtins
import contextlib
import inspect
import keyword
import os
import re
import rlcompleter
import sys
from typing import TYPE_CHECKING, Literal

from ggrust._pane_pages import help_text
from ggrust._viewer import _MAX_TABLE_ROWS, PaneError, Reply, request

if TYPE_CHECKING:
    from collections.abc import Callable

    import polars as pl

    from ggrust.plot.core import PlotSpec
    from ggrust.plot.subplots import SupPlotsSpec

_NO_PANE = (
    "no plot pane is running; start one with 'ggrust pane' in another terminal, "
    "or save the plot with ggsave(_, 'plot.png')"
)


def _color(code: str, text: str) -> str:
    """Wrap ``text`` in an SGR color ``code`` unless NO_COLOR is set or stdout is no tty."""
    if os.environ.get("NO_COLOR") or not sys.stdout.isatty():
        return text
    return f"\x1b[{code}m{text}\x1b[0m"


def _dim(text: str) -> str:
    return _color("2", text)


def _ask(kind: Literal["plot", "table", "text", "control"], body: object) -> Reply:
    """Send a message to the pane and return its reply.

    Raises
    ------
    PaneError
        When no pane is running, or the pane refused the message.
    """
    reply = request(kind, body)
    if reply is None:
        raise PaneError(_NO_PANE)
    if reply.error and kind != "plot":
        raise PaneError(f"the pane refused it: {reply.error}")
    return reply


def _variable_name(value: object) -> str | None:
    """Return the name ``value`` has in the REPL's namespace, if any (for page titles)."""
    namespace = vars(sys.modules["__main__"])
    for name, candidate in namespace.items():
        if candidate is value and not name.startswith("_"):
            return name
    return None


def _as_polars(data: object) -> pl.DataFrame:
    """Return ``data`` as a polars DataFrame (polars, a LazyFrame's first rows, or pandas).

    Raises
    ------
    TypeError
        When ``data`` is no data frame.
    """
    import polars as pl

    if isinstance(data, pl.DataFrame):
        return data
    if isinstance(data, pl.LazyFrame):
        return data.collect()
    if type(data).__module__.startswith("pandas"):
        frame = pl.from_pandas(data)
        if isinstance(frame, pl.DataFrame):
            return frame
    raise TypeError(
        f"pane.show takes a plot or a data frame (polars or pandas), got {type(data).__name__}"
    )


class Pane:
    """Controls for the plot pane, available as ``pane`` in the ggrust REPL.

    Examples
    --------
    >>> pane.show(mpg)                 # the data as a table in the pane
    >>> pane.help(geom_point)          # the docstring, formatted, in the pane
    >>> pane.last() + theme_bw()       # the plot on screen, to build on
    >>> pane.save("plot.png")          # save it at the pane's size
    >>> pane.style("light")            # restyle plots without a flavor
    >>> pane.size(800, 500)            # draw plots at a fixed size; pane.size() undoes it
    >>> pane.clear()                   # empty the history
    """

    def __repr__(self) -> str:
        lines = [
            "pane: the plot pane on the right",
            "  pane.show(mpg)            data as a table (also plots)",
            "  pane.help(geom_point)     a docstring, formatted",
            "  pane.last()               the plot on screen",
            "  pane.save('plot.png')     save it at the pane's size",
            "  pane.style('light')       'dark', 'light' or 'none'",
            "  pane.size(800, 500)       fixed size; pane.size() fills the pane",
            "  pane.clear()              empty the history",
        ]
        return "\n".join(lines)

    def show(self, data: object, name: str | None = None) -> None:
        """Show a plot, or a data frame as a scrollable table, in the pane.

        Parameters
        ----------
        data : PlotSpec, SupPlotsSpec, polars.DataFrame, polars.LazyFrame or pandas.DataFrame
            What to show. Frames send their first 10,000 rows (a LazyFrame collects
            only those); the table says how many rows there are in all.
        name : str or None, default=None
            The table's title; the variable's name when None.

        Raises
        ------
        TypeError
            When ``data`` is neither a plot nor a data frame.
        PaneError
            When no pane is running.
        """
        from ggrust.plot.core import PlotSpec
        from ggrust.plot.subplots import SupPlotsSpec

        if isinstance(data, PlotSpec | SupPlotsSpec):
            _report_plot(data)
            return
        import polars as pl

        if isinstance(data, pl.LazyFrame):
            frame = data.head(_MAX_TABLE_ROWS).collect()
            total = data.select(pl.len()).collect().item()
        else:
            whole = _as_polars(data)
            frame, total = whole.head(_MAX_TABLE_ROWS), whole.height
        title = name or _variable_name(data) or "data"
        reply = _ask("table", (title, frame, int(total)))
        print(_dim(f"  ↳ page {reply.index} in the pane: {title}, {total:,} rows"))

    def help(self, target: object) -> None:
        """Show ``target``'s signature and docstring in the pane, formatted and scrollable.

        Parameters
        ----------
        target : object
            A function, class or other object, such as ``geom_point``.

        Raises
        ------
        PaneError
            When no pane is running.
        """
        title, text = help_text(target)
        reply = _ask("text", (title, text))
        print(_dim(f"  ↳ page {reply.index} in the pane: help for {title}"))

    def last(self) -> PlotSpec | SupPlotsSpec:
        """Return the plot on screen (or, under a table or help page, the plot before it).

        Returns
        -------
        PlotSpec or SupPlotsSpec
            The plot, to build on: ``pane.last() + theme_bw()``.

        Raises
        ------
        PaneError
            When no pane is running, or it has no plot yet.
        """
        plot = _ask("control", ("last", None)).value
        if plot is None:
            raise PaneError("the pane has no plot yet")
        return plot  # ty: ignore[invalid-return-type]

    def save(
        self, filename: str | os.PathLike[str], w: float | None = None, h: float | None = None
    ) -> str:
        """Save the plot on screen to ``filename`` at the pane's size (or ``w`` x ``h``).

        The plot is saved as you made it, without the pane's high-contrast look.

        Parameters
        ----------
        filename : str or PathLike
            Where to save; the extension picks the format, as in ``ggsave``.
        w, h : float or None, default=None
            Size in pixels; the pane's plot area (or ``pane.size``) when None.

        Returns
        -------
        str
            The path written.

        Raises
        ------
        PaneError
            When no pane is running, or it has no plot yet.
        """
        from ggrust import ggsave

        plot = self.last()
        state = _ask("control", ("state", None)).value
        assert isinstance(state, dict)
        width, height = state["size"]
        path = ggsave(plot, filename, w=w or round(width), h=h or round(height), unit="px")
        print(_dim(f"  ↳ saved {path}"))
        return path

    def style(self, name: Literal["dark", "light", "none"]) -> None:
        """Set how the pane dresses plots that have no flavor of their own.

        Parameters
        ----------
        name : {"dark", "light", "none"}
            High contrast for a dark or light terminal, or plots as they are.

        Raises
        ------
        PaneError
            When no pane is running, or the name is unknown.
        """
        _ask("control", ("style", name))

    def size(self, w: float | None = None, h: float | None = None) -> None:
        """Draw plots at ``w`` x ``h`` pixels (fit into the frame), or fill the pane again.

        Parameters
        ----------
        w, h : float or None, default=None
            The size; both None to fill the pane.

        Raises
        ------
        PaneError
            When no pane is running, or only one side is given.
        """
        if (w is None) != (h is None):
            raise PaneError("give both w and h, or neither to fill the pane")
        _ask("control", ("size", None if w is None else (w, h)))

    def clear(self) -> None:
        """Empty the pane's history.

        Raises
        ------
        PaneError
            When no pane is running.
        """
        _ask("control", ("clear", None))


def _report_plot(plot: PlotSpec | SupPlotsSpec) -> None:
    """Send a plot to the pane and say in the REPL where it went, or what went wrong."""
    reply = request("plot", plot)
    if reply is None:
        print(_dim(f"  {_NO_PANE}"))
    elif reply.error:
        print(_color("31", f"  ✗ plot {reply.index} could not be drawn: {reply.error}"))
    else:
        print(_dim(f"  ↳ plot {reply.index} in the pane"))


def install_displayhook() -> None:
    """Send plots typed at the REPL to the pane; everything else prints as usual."""
    from ggrust.plot.core import PlotSpec
    from ggrust.plot.subplots import SupPlotsSpec

    previous: Callable[[object], None] = sys.displayhook

    def hook(value: object) -> None:
        if not isinstance(value, PlotSpec | SupPlotsSpec):
            previous(value)
            return
        setattr(builtins, "_", value)  # noqa: B010  # like the default hook: ``_`` is the last plot
        _report_plot(value)

    sys.displayhook = hook


# --- keyword completion ---------------------------------------------------------

_CALL = re.compile(r"([A-Za-z_][\w.]*)\($")


def _open_call(line: str) -> str | None:
    """Return the dotted name of the innermost call still open at the end of ``line``."""
    depth = 0
    for index in range(len(line) - 1, -1, -1):
        character = line[index]
        if character in ")]}":
            depth += 1
        elif character in "([{":
            if depth:
                depth -= 1
            elif character == "(":
                match = _CALL.search(line[: index + 1])
                return match.group(1) if match else None
            else:
                return None
    return None


def _keywords(target: object) -> list[str]:
    """Return the keyword arguments ``target`` accepts, in signature order."""
    try:
        parameters = inspect.signature(target).parameters.values()  # ty: ignore[invalid-argument-type]
    except (TypeError, ValueError):
        return []
    usable = (inspect.Parameter.POSITIONAL_OR_KEYWORD, inspect.Parameter.KEYWORD_ONLY)
    return [parameter.name for parameter in parameters if parameter.kind in usable]


class _Completer(rlcompleter.Completer):
    """Python's completer, plus ``name=`` for the keywords of the call being typed.

    Inside ``geom_point(``, typing ``s`` and Tab lists ``stat=``, ``show_legend=``,
    ``sampling=`` and ``size_unit=``; when no keyword matches, names complete as usual.
    After a bare ``(`` the new REPL's Tab indents instead: type a letter first.
    The call's name is looked up as a plain dotted name in the namespace (attribute
    access only, nothing is called).
    """

    def __init__(self, namespace: dict[str, object], line: Callable[[], str]) -> None:
        super().__init__(namespace)
        self.namespace_ = namespace
        self.line = line
        self.matches_: list[str] = []

    def lookup(self, dotted: str) -> object | None:
        """Return the object a dotted name refers to in the namespace, without calling anything."""
        parts = dotted.split(".")
        if parts[0] not in self.namespace_ and not hasattr(builtins, parts[0]):
            return None
        target = self.namespace_.get(parts[0], getattr(builtins, parts[0], None))
        for part in parts[1:]:
            target = inspect.getattr_static(target, part, None)
            if target is None:
                return None
        return target

    def complete(self, text: str, state: int) -> str | None:
        if state == 0:
            self.matches_ = []
            with contextlib.suppress(Exception):  # completion must never raise
                line = self.line()
                before = line[: len(line) - len(text)] if line.endswith(text) else line
                call = _open_call(before)
                if call is not None and not keyword.iskeyword(call):
                    target = self.lookup(call)
                    if target is not None and (text == "" or text.isidentifier()):
                        self.matches_ = [
                            f"{name}=" for name in _keywords(target) if name.startswith(text)
                        ]
            if text and not self.matches_:  # keywords that match win; else names
                base: list[str] = []
                index = 0
                while (match := super().complete(text, index)) is not None:
                    base.append(match)
                    index += 1
                self.matches_ += [match for match in base if match not in self.matches_]
        return self.matches_[state] if state < len(self.matches_) else None


def install_completer() -> bool:
    """Make Tab complete keyword arguments too, in the new REPL and the basic one.

    Python 3.13+'s REPL (``_pyrepl``) sets up its completer once, from ``__main__``'s
    namespace, after the startup file runs; running that setup here (the REPL then
    skips it) lets us replace the completer it made. The basic REPL uses ``readline``.

    Returns
    -------
    bool
        False when neither REPL's completion could be set up.
    """
    namespace = vars(sys.modules["__main__"])
    installed = False
    if not os.environ.get("PYTHON_BASIC_REPL"):
        with contextlib.suppress(Exception):  # private API: fall back quietly if it moved
            from _pyrepl import readline as pyrepl  # ty: ignore[unresolved-import]
            from _pyrepl.main import CAN_USE_PYREPL  # ty: ignore[unresolved-import]

            if CAN_USE_PYREPL:
                pyrepl._setup(namespace)
                pyrepl.set_completer(_Completer(namespace, pyrepl.get_line_buffer).complete)
                installed = True
    try:
        import readline
    except ImportError:
        return installed
    readline.set_completer(_Completer(namespace, readline.get_line_buffer).complete)
    readline.parse_and_bind("tab: complete")
    return True


# --- banner ---------------------------------------------------------------------


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
        clear + _color("1", "ggrust") + _dim(f" {__version__} · plots show in the pane →"),
        "",
        row("imported", code("from ggrust import *")),
        row("", code("from ggrust.data import *")),
        row("data", "mpg, penguins, iris, economics"),
        row("pane", code("pane.show(mpg)") + "  " + code("pane.help(aes)")),
        row("", _dim("type ") + code("pane") + _dim(" for the rest")),
        row("try", ""),
        "    " + code("ggplot(mpg, aes('displ', 'hwy')) + geom_point()"),
        "",
    ]
    return "\n".join(lines)


def start() -> Pane:
    """Set up the workspace REPL: plot hook, Tab completion, banner; return ``pane``.

    Returns
    -------
    Pane
        The pane controls, bound to ``pane`` in the REPL.
    """
    install_displayhook()
    install_completer()
    print(repl_banner())
    return Pane()
