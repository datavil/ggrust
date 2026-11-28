"""Prototype: the REPL side of the terminal workspace (the left pane of ``ggrust``).

``start()`` runs from the REPL's startup file. It sends plots you type to the pane
and says where they went (or why they could not be drawn), completes keyword
arguments on Tab, prints the banner, and returns the ``pane`` object. ``names(pane)``
gives the REPL the workspace controls as bare names: ``show``, ``help`` (in the pane,
else Python's), ``last``, ``save``, ``style``, ``size`` and ``clear``. These exist only
in the workspace REPL; nothing is added to ``from ggrust import *``. Not part of the
public API: a prototype on its own branch.
"""

from __future__ import annotations

import builtins
import code
import contextlib
import inspect
import keyword
import os
import re
import rlcompleter
import shutil
import sys
import time
import types
from typing import TYPE_CHECKING, Literal

from ggrust._pane_pages import help_text
from ggrust._viewer import _MAX_TABLE_ROWS, PaneError, Reply, request, size_problem

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
        f"show takes a plot or a data frame (polars or pandas), got {type(data).__name__}"
    )


class Pane:
    """Controls for the plot pane, available as ``pane`` in the ggrust REPL.

    The REPL also has each method as a bare name (see ``names``).

    Examples
    --------
    >>> show(mpg)                 # the data as a table in the pane
    >>> help(geom_point)          # the docstring, formatted, in the pane
    >>> last() + theme_bw()       # the plot on screen, to build on
    >>> save("plot.png")          # save it at the pane's size
    >>> style("light")            # restyle plots without a flavor
    >>> size(800, 500)            # draw plots at a fixed size; size() undoes it
    >>> clear()                   # empty the history
    """

    def __repr__(self) -> str:
        lines = [
            "pane: the plot pane on the right",
            "  show(mpg)            data as a table (also plots)",
            "  help(geom_point)     a docstring, formatted; help() is Python's",
            "  last()               the plot on screen",
            "  save('plot.png')     save it at the pane's size",
            "  style('light')       'dark', 'light' or 'none'",
            "  size(800, 500)       fixed size; size() fills the pane",
            "  clear()              empty the history",
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

        if isinstance(data, pl.LazyFrame):  # one run of the query gives the rows and the count
            names = set(data.collect_schema().names())
            count = "__ggrust_rows"
            while count in names:
                count += "_"
            head = data.with_columns(pl.len().alias(count)).head(_MAX_TABLE_ROWS).collect()
            total = head.get_column(count)[0] if head.height else 0
            frame = head.drop(count)
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
            The plot, to build on: ``last() + theme_bw()``.

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
            Size in pixels; the pane's plot area (or ``size``) when None.

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
        TypeError
            When a side is not a number (a bool is not one).
        ValueError
            When a side is not finite, not positive, or above 20,000 pixels.
        """
        if (w is None) != (h is None):
            raise PaneError("give both w and h, or neither to fill the pane")
        if w is not None:
            problem = size_problem(w, h)
            if problem is not None:
                number = not isinstance(w, bool) and not isinstance(h, bool)
                number = number and all(isinstance(side, int | float) for side in (w, h))
                raise (ValueError if number else TypeError)(problem)
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
_TYPED_KEYWORD = re.compile(r"(?:^|,)\s*([A-Za-z_]\w*)\s*=(?!=)")


def _code_only(line: str) -> str | None:
    """Return ``line`` with string contents blanked and any comment cut off.

    Brackets and ``=`` inside strings and comments then no longer count. Returns None
    when the line ends inside a string (nothing to complete there).
    """
    out: list[str] = []
    index, quote = 0, ""
    while index < len(line):
        character = line[index]
        if quote:
            if character == "\\":
                out.append("  ")
                index += 2
                continue
            if line.startswith(quote, index):
                out.append(quote)
                index += len(quote)
                quote = ""
                continue
            out.append(" ")
        elif character == "#":
            break
        elif character in "'\"":
            quote = character * 3 if line.startswith(character * 3, index) else character
            out.append(quote)
            index += len(quote)
            continue
        else:
            out.append(character)
        index += 1
    return None if quote else "".join(out)


def _call_at_end(line: str) -> tuple[str, str] | None:
    """Return the innermost call still open at the end of ``line``, and its arguments so far.

    The arguments are the text typed at the call's own level (nested brackets left out).
    """
    code = _code_only(line)
    if code is None:
        return None
    depth = 0
    for index in range(len(code) - 1, -1, -1):
        character = code[index]
        if character in ")]}":
            depth += 1
        elif character in "([{":
            if depth:
                depth -= 1
            elif character == "(":
                match = _CALL.search(code[: index + 1])
                if match is None:
                    return None
                arguments: list[str] = []
                level = 0
                for character in code[index + 1 :]:
                    if character in "([{":
                        level += 1
                    elif character in ")]}":
                        level -= 1
                    elif level == 0:
                        arguments.append(character)
                return match.group(1), "".join(arguments)
            else:
                return None
    return None


def _open_call(line: str) -> str | None:
    """Return the dotted name of the innermost call still open at the end of ``line``."""
    call = _call_at_end(line)
    return None if call is None else call[0]


def _typed_keywords(line: str) -> set[str]:
    """Return the keywords already given in the call still open at the end of ``line``."""
    call = _call_at_end(line)
    return set() if call is None else set(_TYPED_KEYWORD.findall(call[1]))


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
            owner = target
            target = inspect.getattr_static(owner, part, None)
            if target is None:
                return None
            # bind methods as attribute access would (without running any code), so the
            # signature drops ``self`` / ``cls``
            if isinstance(target, staticmethod):
                target = target.__func__
            elif isinstance(target, classmethod):
                cls = owner if isinstance(owner, type) else type(owner)
                target = types.MethodType(target.__func__, cls)
            elif isinstance(target, types.FunctionType) and not isinstance(
                owner, type | types.ModuleType
            ):
                target = types.MethodType(target, owner)
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
                        typed = _typed_keywords(before)
                        self.matches_ = [
                            f"{name}="
                            for name in _keywords(target)
                            if name.startswith(text) and name not in typed
                        ]
            if text and not self.matches_:  # keywords that match win; else names
                base: list[str] = []
                index = 0
                while (match := super().complete(text, index)) is not None:
                    base.append(match)
                    index += 1
                self.matches_ += [match for match in base if match not in self.matches_]
        return self.matches_[state] if state < len(self.matches_) else None


def _new_repl() -> bool:
    """Whether the REPL that starts after the startup file is Python 3.13+'s ``_pyrepl``."""
    if os.environ.get("PYTHON_BASIC_REPL"):
        return False
    try:
        from _pyrepl.main import CAN_USE_PYREPL  # ty: ignore[unresolved-import]
    except ImportError:
        return False
    return bool(CAN_USE_PYREPL) and sys.stdin.isatty() and sys.stdout.isatty()


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
    if _new_repl():
        with contextlib.suppress(Exception):  # private API: fall back quietly if it moved
            from _pyrepl import readline as pyrepl  # ty: ignore[unresolved-import]

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


# --- block dividers ------------------------------------------------------------


def _duration(seconds: float) -> str:
    """Return a short run time: ``0.24s``, ``12.3s``, ``2m 05s``."""
    if seconds < 10:
        return f"{seconds:.2f}s"
    if seconds < 60:
        return f"{seconds:.1f}s"
    minutes, rest = divmod(round(seconds), 60)
    return f"{minutes}m {rest:02d}s"


def divider(number: int, seconds: float, failed: bool, columns: int) -> str:
    """Return the dim rule drawn under a block: its number, a red cross if it raised, its run time.

    Parameters
    ----------
    number : int
        The block's number, counting from 1.
    seconds : float
        How long it ran.
    failed : bool
        Whether it raised an exception.
    columns : int
        The terminal's width; the rule fills it.

    Returns
    -------
    str
        The line, with colors unless ``NO_COLOR`` is set or stdout is no terminal.
    """
    left = f"\u2570\u2500 {number} "
    mark = "\u2717 " if failed else ""
    right = f" {_duration(seconds)} \u2500"
    fill = max(columns - len(left) - len(mark) - len(right), 2)
    return _dim(left) + _color("31", mark) + _dim("\u2500" * fill + right)


class _Blocks:
    """Counts the blocks the REPL runs, times them, and draws a divider under each.

    A block starts when the REPL runs code it compiled from your input (the
    ``exec`` audit event for ``<stdin>``, ``<console>`` or ``<python-input-N>`` code) and ends
    when the REPL asks for the next input. Empty lines and syntax errors run no
    code, so they get no divider.
    """

    def __init__(self) -> None:
        self.count = 0
        self.started: float | None = None
        self.last_exception: object = getattr(sys, "last_exc", None)

    def audit(self, event: str, arguments: tuple[object, ...]) -> None:
        """Note when the REPL starts running a block (audit hooks must be fast)."""
        if event != "exec" or self.started is not None or not arguments:
            return
        filename = getattr(arguments[0], "co_filename", "")
        names = getattr(arguments[0], "co_names", ())
        constants = getattr(arguments[0], "co_consts", ())
        if not names and all(constant is None for constant in constants):
            return  # an empty line, which code.InteractiveConsole runs as ``pass``
        if filename in ("<stdin>", "<console>") or filename.startswith("<python-input-"):
            self.started = time.perf_counter()

    def finish(self) -> None:
        """Draw the divider for the block that just ran, if one did."""
        if self.started is None:
            return
        seconds = time.perf_counter() - self.started
        self.started = None
        self.count += 1
        exception = getattr(sys, "last_exc", None)
        failed = exception is not None and exception is not self.last_exception
        self.last_exception = exception
        columns = shutil.get_terminal_size((80, 24)).columns
        sys.stdout.write(divider(self.count, seconds, failed, columns) + "\n")
        sys.stdout.flush()


class _Prompt:
    """``sys.ps1`` for the basic REPL: draws the last block's divider, then reads as ``>>> ``."""

    def __init__(self, blocks: _Blocks, prompt: str) -> None:
        self.blocks = blocks
        self.prompt = prompt

    def __str__(self) -> str:
        with contextlib.suppress(Exception):  # a prompt must never fail
            self.blocks.finish()
        return self.prompt


def install_dividers() -> bool:
    """Draw a numbered, timed divider under every block the REPL runs.

    The new REPL (``_pyrepl``) gets it by wrapping the function it calls to read the
    next input. Otherwise ``sys.ps1`` draws it when the prompt is shown; that needs
    the REPL to run blocks with ``exec`` (an audit event), which Python's C REPL does
    not, so ``interact()`` runs ``code.InteractiveConsole`` in its place.

    Returns
    -------
    bool
        False when ``GGRUST_NO_DIVIDERS`` is set, so no dividers are drawn.
    """
    if os.environ.get("GGRUST_NO_DIVIDERS"):
        return False
    blocks = _Blocks()
    sys.addaudithook(blocks.audit)
    sys.ps1 = _Prompt(blocks, str(getattr(sys, "ps1", ">>> ")))
    with contextlib.suppress(Exception):  # private API: the basic REPL's way still works
        from _pyrepl import simple_interact  # ty: ignore[unresolved-import]

        read = simple_interact.multiline_input

        def multiline_input(more_lines: object, ps1: object, ps2: object) -> str:
            with contextlib.suppress(Exception):
                blocks.finish()
            return read(more_lines, str(ps1), ps2)

        simple_interact.multiline_input = multiline_input
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
        row("pane", code("show(mpg)") + "  " + code("help(aes)")),
        row("", _dim("type ") + code("pane") + _dim(" for the rest")),
        row("try", ""),
        "    " + code("ggplot(mpg, aes('displ', 'hwy')) + geom_point()"),
        "",
    ]
    return "\n".join(lines)


def interact() -> None:
    """Run the REPL here when the new REPL is not available (Python 3.11/3.12, or basic).

    ``code.InteractiveConsole`` runs each block with ``exec``, so the dividers work,
    and uses ``readline`` for editing, history and completion as the C REPL does.
    Returns at once under the new REPL; otherwise exits Python when you leave it.
    """
    if _new_repl() or not sys.stdin.isatty():
        return
    with contextlib.suppress(ImportError):
        import readline  # noqa: F401  # line editing for input()
    console = code.InteractiveConsole(vars(sys.modules["__main__"]))
    console.interact(banner="", exitmsg="")
    raise SystemExit(0)


def start() -> Pane:
    """Set up the workspace REPL: plot hook, Tab completion, banner; return ``pane``.

    Returns
    -------
    Pane
        The pane controls, bound to ``pane`` in the REPL.
    """
    install_displayhook()
    install_completer()
    install_dividers()
    print(repl_banner())
    return Pane()


class _Help:
    """``help`` in the workspace REPL: help pages in the pane, else Python's own ``help``.

    ``help(geom_point)`` shows the page in the pane; with no pane running it prints
    Python's help instead. ``help()`` with no argument is Python's interactive help.
    """

    def __init__(self, pane: Pane) -> None:
        self.pane = pane

    def __repr__(self) -> str:
        return "Type help(object) for its help in the pane, or help() for Python's help."

    def __call__(self, target: object = None, /) -> None:
        """Show ``target``'s help in the pane (Python's help when no pane is running).

        Parameters
        ----------
        target : object, default=None
            What to explain; None opens Python's interactive help.

        Raises
        ------
        PaneError
            When the pane refused the page.
        """
        if target is None:
            builtins.help()
            return
        try:
            self.pane.help(target)
        except PaneError as error:
            if str(error) != _NO_PANE:
                raise
            builtins.help(target)


def names(pane: Pane) -> dict[str, object]:
    """Return the workspace controls as the bare names the REPL gets.

    Parameters
    ----------
    pane : Pane
        The pane controls, which stay available as ``pane`` too.

    Returns
    -------
    dict of str to object
        ``show``, ``help``, ``last``, ``save``, ``style``, ``size`` and ``clear``.
    """
    return {
        "show": pane.show,
        "help": _Help(pane),
        "last": pane.last,
        "save": pane.save,
        "style": pane.style,
        "size": pane.size,
        "clear": pane.clear,
    }
