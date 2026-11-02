"""``PlotSpec.show()``: inline in IPython, the browser elsewhere."""

import webbrowser
from pathlib import Path

import ggrust as gg
import pytest
from ggrust.plot.core import PlotSpec


def plots() -> list[PlotSpec | gg.SupPlotsSpec]:
    plot = gg.ggplot() + gg.geom_point(x=0, y=0)
    return [plot, gg.gggrid([plot, plot])]


@pytest.mark.parametrize("plot", plots(), ids=["plot", "grid"])
def test_show_outside_ipython_opens_the_html_page(
    plot: PlotSpec | gg.SupPlotsSpec, monkeypatch: pytest.MonkeyPatch
) -> None:
    opened: list[str] = []
    monkeypatch.setattr(webbrowser, "open", lambda url: opened.append(url) or True)
    assert plot.show() is None
    assert len(opened) == 1 and opened[0].startswith("file://")
    page = Path(opened[0].removeprefix("file://"))
    try:
        assert "<svg" in page.read_text(encoding="utf-8")
    finally:
        page.unlink()


def test_show_in_ipython_displays_inline(monkeypatch: pytest.MonkeyPatch) -> None:
    ipython = pytest.importorskip("IPython")
    ipython_display = pytest.importorskip("IPython.display")

    shown: list[object] = []
    monkeypatch.setattr(ipython, "get_ipython", lambda: object())
    monkeypatch.setattr(ipython_display, "display", lambda value: shown.append(value))
    monkeypatch.setattr(webbrowser, "open", lambda url: pytest.fail("browser opened"))
    plot = plots()[0]
    plot.show()
    assert shown == [plot]
