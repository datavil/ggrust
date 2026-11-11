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
        if build_examples.binds_plot(code):
            assert test.globs["plot"].to_svg().startswith("<svg")
