"""Every public name documents a runnable example, and every example plot renders.

The examples are the ``Examples`` sections of the docstrings listed in the API
reference (``docs/build_reference.py``). They run as doctests, so printed output
must match. The docs build renders the ones that bind ``plot``; this test keeps them
working and complete.
"""

import doctest
import inspect
import sys
import warnings
from pathlib import Path

import pytest
from ggrust import animate
from ggrust._render import is_animated

sys.path.insert(0, str(Path(__file__).parent.parent / "docs"))

import build_examples
import build_reference

PATHS: list[str] = [
    path
    for _, paths in build_reference.SECTIONS.values()
    for path in paths
    if path not in build_reference.ALIASES  # an alias runs its target's example
]


@pytest.mark.parametrize("path", PATHS)
def test_docstring_example_runs(path: str) -> None:
    """The docstring has an example; it passes as a doctest and its plot renders."""
    item = build_reference.resolve(path)
    code = build_examples.docstring_example(item)
    assert code, f"{path} has no runnable Examples section"
    test = doctest.DocTestParser().get_doctest(inspect.getdoc(item), {}, path, None, 0)
    runner = doctest.DocTestRunner(optionflags=doctest.ELLIPSIS)
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        report: list[str] = []
        result = runner.run(test, out=report.append, clear_globs=False)
        assert result.failed == 0, "".join(report)
        # Every example that binds `plot` gets a figure in the docs, not only the last.
        plots = list(build_examples.example_plots(item))
        assert len(plots) == len(build_examples.reference_figures("name", item))
        for plot in plots:
            # An animated plot renders as its first frame, as in the docs gallery.
            still = animate(plot).frame(0) if is_animated(plot) else plot
            assert still.to_svg().startswith("<svg")


def _documented_methods() -> list[tuple[str, str]]:
    """Return (class path, method name) for each public method with a docstring example."""
    found: list[tuple[str, str]] = []
    for path in PATHS:
        item = build_reference.resolve(path)
        if not inspect.isclass(item):
            continue
        for name, member in vars(item).items():
            target = member.fget if isinstance(member, property) else member
            if not name.startswith("_") and ">>>" in (inspect.getdoc(target) or ""):
                found.append((path, name))
    return found


@pytest.mark.parametrize(("path", "name"), _documented_methods())
def test_method_docstring_example_runs(path: str, name: str) -> None:
    """A method's docstring example passes as a doctest, without warnings."""
    member = vars(build_reference.resolve(path))[name]
    target = member.fget if isinstance(member, property) else member
    test = doctest.DocTestParser().get_doctest(
        inspect.getdoc(target), {}, f"{path}.{name}", None, 0
    )
    runner = doctest.DocTestRunner(optionflags=doctest.ELLIPSIS)
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        report: list[str] = []
        result = runner.run(test, out=report.append, clear_globs=False)
    assert result.failed == 0, "".join(report)


def test_each_example_block_gets_a_figure() -> None:
    """A docstring with several examples gives one labelled figure per `plot` block."""

    def documented() -> None:
        """Summary.

        Examples
        --------
        >>> from ggrust import *
        >>> data = {"x": [1, 2], "y": [3, 4]}
        >>> plot = ggplot(data, aes("x", "y")) + geom_point()

        The same data as
        ``geom_line``:

        >>> plot = ggplot(data, aes("x", "y")) + geom_line()

        No plot here:

        >>> len(data)
        2
        >>> hidden = 1  # doctest: +SKIP
        """

    blocks = build_examples.example_blocks(documented)
    assert [block.label for block in blocks] == ["", "The same data as `geom_line`", "No plot here"]
    assert "hidden" not in blocks[2].code
    figures = build_examples.reference_figures("documented", documented)
    assert figures == [("documented", ""), ("documented_2", "The same data as `geom_line`")]
    assert len(list(build_examples.example_plots(documented))) == 2
