"""``plot.svg``, ``plot.png`` and ``plot.html``: renderable str/bytes for notebooks."""

from __future__ import annotations

import pickle
import re
from collections.abc import Iterator

import pytest
from ggrust import aes, display, geom_point, gggrid, ggplot
from ggrust._display import current_display
from ggrust._rendered import HTML, PNG, SVG
from ggrust.plot.core import PlotSpec
from ggrust.plot.subplots import SupPlotsSpec
from IPython.core.formatters import DisplayFormatter
from IPython.lib.pretty import pretty

DISPLAY_ID = re.compile(r"ggrust-[0-9a-f]{12}")


def _plot() -> PlotSpec:
    data = {"x": [1, 2, 3], "y": [3, 1, 2], "group": ["a", "b", "a"]}
    return ggplot(data, aes("x", "y", color="group")) + geom_point()


def _plots() -> list[PlotSpec | SupPlotsSpec]:
    return [_plot(), gggrid([_plot(), _plot()])]


def _mimetypes(value: object) -> list[str]:
    data, _ = DisplayFormatter().format(value)
    return sorted(data)


@pytest.fixture
def restore_display() -> Iterator[None]:
    previous = current_display()
    yield
    display(previous)


@pytest.mark.parametrize("plot", _plots(), ids=["plot", "grid"])
def test_svg_is_the_to_svg_text_and_shows_as_svg(plot: PlotSpec | SupPlotsSpec) -> None:
    svg = plot.svg
    assert isinstance(svg, SVG)
    assert isinstance(svg, str)
    assert svg == plot.to_svg()
    assert _mimetypes(svg) == ["image/svg+xml", "text/plain"]
    assert svg._repr_svg_() == svg


@pytest.mark.parametrize("plot", _plots(), ids=["plot", "grid"])
def test_png_is_a_png_and_shows_as_png(plot: PlotSpec | SupPlotsSpec) -> None:
    png = plot.png
    assert isinstance(png, PNG)
    assert isinstance(png, bytes)
    assert png.startswith(b"\x89PNG\r\n\x1a\n")
    assert _mimetypes(png) == ["image/png", "text/plain"]
    assert png._repr_png_() == png


@pytest.mark.parametrize("plot", _plots(), ids=["plot", "grid"])
def test_png_is_the_png_display_mode_image(
    plot: PlotSpec | SupPlotsSpec, restore_display: None
) -> None:
    import base64

    display("png")
    shown = plot._repr_html_()
    assert base64.b64encode(plot.png).decode() in shown


@pytest.mark.parametrize("plot", _plots(), ids=["plot", "grid"])
def test_html_is_the_to_html_page_and_shows_interactive(plot: PlotSpec | SupPlotsSpec) -> None:
    page = plot.html
    assert isinstance(page, HTML)
    assert isinstance(page, str)
    # Every render gets a fresh display id; otherwise it is the to_html() page.
    expected = DISPLAY_ID.sub("ID", plot.to_html() or "")
    assert DISPLAY_ID.sub("ID", page) == expected
    assert page.startswith("<!DOCTYPE html>")
    assert _mimetypes(page) == ["text/html", "text/plain"]
    fragment = page._repr_html_()
    assert not fragment.startswith("<!DOCTYPE")
    assert 'class="ggrust-plot"' in fragment


@pytest.mark.parametrize("mode", ["svg", "png"])
def test_html_stays_interactive_in_static_display_modes(mode: str, restore_display: None) -> None:
    plot = _plot()
    display(mode)  # ty: ignore[invalid-argument-type]
    assert not plot._repr_html_().startswith('<div id="ggrust-')
    assert 'class="ggrust-plot"' in plot.html._repr_html_()


def _shown_kind(plot: PlotSpec | SupPlotsSpec) -> str:
    """Which display mode ``plot._repr_html_()`` used."""
    shown = plot._repr_html_()
    if shown.startswith("<svg"):
        return "svg"
    if shown.startswith('<img src="data:image/png;base64,'):
        return "png"
    assert 'class="ggrust-plot"' in shown
    return "html"


@pytest.mark.parametrize("mode", ["html", "svg", "png"])
@pytest.mark.parametrize("plot", _plots(), ids=["plot", "grid"])
def test_properties_leave_the_display_of_later_plots_alone(
    mode: str, plot: PlotSpec | SupPlotsSpec, restore_display: None
) -> None:
    display(mode)  # ty: ignore[invalid-argument-type]
    later = _plot()
    assert _shown_kind(later) == mode
    # Read and display every property, as notebook cells ending in them would.
    for value in (plot.svg, plot.png, plot.html):
        DisplayFormatter().format(value)
    assert current_display() == mode
    assert _shown_kind(plot) == mode
    assert _shown_kind(later) == mode
    assert _shown_kind(gggrid([later])) == mode


def test_properties_follow_the_plot_they_were_read_from() -> None:
    plot = _plot()
    before = plot.html
    after = (plot + geom_point(size=9)).html
    assert DISPLAY_ID.sub("ID", before._repr_html_()) != DISPLAY_ID.sub("ID", after._repr_html_())
    assert plot.svg != (plot + geom_point(size=9)).svg


def test_pretty_printing_is_a_short_summary() -> None:
    plot = _plot()
    assert re.fullmatch(r"<SVG: [\d,]+ characters>", pretty(plot.svg))
    assert re.fullmatch(r"<PNG: [\d,]+ bytes>", pretty(plot.png))
    assert re.fullmatch(r"<HTML: [\d,]+ characters>", pretty(plot.html))


def test_html_without_a_plot_shows_its_own_text() -> None:
    assert HTML("<b>x</b>")._repr_html_() == "<b>x</b>"


def test_rendered_values_pickle() -> None:
    plot = _plot()
    for value in (plot.svg, plot.png):
        assert pickle.loads(pickle.dumps(value)) == value
    page = pickle.loads(pickle.dumps(plot.html))
    assert isinstance(page, HTML)
    assert 'class="ggrust-plot"' in page._repr_html_()
