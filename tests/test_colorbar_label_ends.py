"""``guide_colorbar(label_position="ends")``: end labels beside the bar (a ggrust option)."""

import re

import ggrust as gg
import numpy as np
import pytest
from ggrust.errors import SpecError
from ggrust.plot.core import PlotSpec

FRAME = {"x": np.arange(6) % 3, "y": np.arange(6) // 3, "v": np.linspace(-1.0, 1.0, 6)}


def _plot(position: str, **guide: object) -> PlotSpec:
    return (
        gg.ggplot(FRAME)
        + gg.geom_tile(gg.aes("x", "y", fill="v"))
        + gg.scale_fill_gradient(name="RNA", breaks=[-1.0, 0.0, 1.0], labels=["Min", "mid", "Max"])
        + gg.theme(legend_position=position)
        + gg.guides(fill=gg.guide_colorbar(**guide))
    )


def _text(svg: str, word: str) -> dict[str, str]:
    match = re.search(r"<text([^>]*)>" + word + "<", svg)
    assert match, word
    return dict(re.findall(r'([\w-]+)="([^"]*)"', match.group(1)))


def test_horizontal_end_labels_sit_left_and_right_of_the_bar() -> None:
    svg = _plot("bottom", label_position="ends", barwidth=80).to_svg()
    low, high = _text(svg, "Min"), _text(svg, "Max")
    # Bar-side edges: Min ends where the bar starts (minus a gap), Max starts after it.
    assert low["text-anchor"] == "end"
    assert "text-anchor" not in high or high["text-anchor"] == "start"
    assert float(high["x"]) - float(low["x"]) >= 80
    assert low["y"] == high["y"]
    assert ">mid<" not in svg
    title = _text(svg, "RNA")
    assert float(title["x"]) < float(low["x"])


def test_default_keeps_every_label_below_the_bar() -> None:
    svg = _plot("bottom").to_svg()
    assert ">mid<" in svg
    assert _text(svg, "Min")["y"] == _text(svg, "Max")["y"]


def test_vertical_end_labels_sit_below_and_above_the_bar() -> None:
    svg = _plot("right", label_position="ends").to_svg()
    assert float(_text(svg, "Max")["y"]) < float(_text(svg, "Min")["y"])
    assert ">mid<" not in svg


def test_unknown_label_position_suggests_ends() -> None:
    with pytest.raises(
        SpecError, match=r"label_position='end'.*expected 'ends'.*did you mean 'ends'"
    ):
        gg.guide_colorbar(label_position="end")  # ty: ignore[invalid-argument-type]


def test_engine_rejects_unknown_label_position() -> None:
    # Past the Python check (as a hand-written spec would be): the engine names the path.
    from ggrust.plot.guide import _guide

    plot = _plot("bottom") + gg.guides(fill=_guide("colorbar", label_position="sides"))
    with pytest.raises(
        ValueError, match=r"label_position.*unknown colorbar label_position 'sides'"
    ):
        plot.to_svg()
