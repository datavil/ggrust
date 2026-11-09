"""ggplot2's ``geom_rug``: ticks on the panel edges for every x and/or y value.

Read from the drawn SVG: tick counts per side, tick length as a share of the
panel (ggplot2 ``unit(0.03, "npc")``), ``coord_flip`` moving the sides with
the axes (ggplot2 maps ``tblr`` to ``rlbt``), facets, discrete axes, mapped
styles and typed errors. Visual reference: the same plots in ggplot2 3.5.2.
"""

from __future__ import annotations

import re

import ggrust as gg
import pytest
from ggrust import GgrustWarning
from ggrust.errors import SpecError

DATA = {"height": [150.0, 162.0, 171.0, 180.0], "weight": [52.0, 60.0, 72.0, 85.0]}
# Rug ticks are the only 1.1 px lines (linewidth 0.5 x 2.2 px).
TICK = re.compile(r'<path d="M([\d.]+) ([\d.]+)L([\d.]+) ([\d.]+)"[^>]*stroke-width="1.1"')
PANEL = re.compile(r'<rect x="([\d.]+)" y="([\d.]+)" width="([\d.]+)" height="([\d.]+)"/>')

Tick = tuple[float, float, float, float]


def ticks(plot: gg.PlotSpec) -> list[Tick]:
    svg = plot.to_svg()
    assert isinstance(svg, str)
    return [tuple(float(value) for value in match) for match in TICK.findall(svg)]


def panel(plot: gg.PlotSpec) -> tuple[float, float, float, float]:
    svg = plot.to_svg()
    assert isinstance(svg, str)
    match = PANEL.search(svg)
    assert match is not None
    x, y, width, height = (float(value) for value in match.groups())
    return x, y, width, height


def sides_of(found: list[Tick], plot: gg.PlotSpec) -> dict[str, int]:
    x, y, _, height = panel(plot)
    counts = {"t": 0, "r": 0, "b": 0, "l": 0}
    for x0, y0, x1, _ in found:
        if x0 == x1:
            counts["b" if abs(y0 - (y + height)) < 0.02 else "t"] += 1
        else:
            counts["l" if abs(x0 - x) < 0.02 else "r"] += 1
    return counts


def plot_with(**options: object) -> gg.PlotSpec:
    return (
        gg.ggplot(DATA, gg.aes("height", "weight")) + gg.geom_rug(**options) + gg.ggsize(400, 300)
    )


def test_default_sides_bottom_and_left() -> None:
    plot = plot_with()
    assert sides_of(ticks(plot), plot) == {"t": 0, "r": 0, "b": 4, "l": 4}


def test_all_sides() -> None:
    plot = plot_with(sides="trbl")
    assert sides_of(ticks(plot), plot) == {"t": 4, "r": 4, "b": 4, "l": 4}


def test_length_is_a_share_of_the_panel() -> None:
    for length in (0.03, 0.1):
        plot = plot_with(length=length, sides="bl")
        _, _, width, height = panel(plot)
        for x0, y0, x1, y1 in ticks(plot):
            if x0 == x1:
                assert abs(y0 - y1) == pytest.approx(length * height, abs=0.02)
            else:
                assert abs(x0 - x1) == pytest.approx(length * width, abs=0.02)


def test_only_x_mapped() -> None:
    plot = gg.ggplot(DATA, gg.aes("height")) + gg.geom_rug(sides="trbl")
    assert sides_of(ticks(plot), plot) == {"t": 4, "r": 0, "b": 4, "l": 0}


def test_coord_flip_moves_the_sides() -> None:
    # ggplot2: under coord_flip, x ticks on "b" go to the left edge.
    plot = gg.ggplot(DATA, gg.aes("height")) + gg.geom_rug(sides="b") + gg.coord_flip()
    assert sides_of(ticks(plot), plot) == {"t": 0, "r": 0, "b": 0, "l": 4}


def test_discrete_axis_ticks_at_categories() -> None:
    data = {"fruit": ["apple", "pear", "apple", "fig"], "price": [1.0, 2.0, 3.0, 4.0]}
    plot = gg.ggplot(data, gg.aes("fruit", "price")) + gg.geom_rug(sides="b")
    found = ticks(plot)
    assert len(found) == 4
    assert len({tick[0] for tick in found}) == 3


def test_facets_draw_per_panel() -> None:
    data = {**DATA, "city": ["Oslo", "Rome", "Oslo", "Rome"]}
    plot = gg.ggplot(data, gg.aes("height", "weight")) + gg.geom_rug() + gg.facet_wrap("city")
    assert len(ticks(plot)) == 8


def test_mapped_styles() -> None:
    plot = gg.ggplot(DATA, gg.aes("height", "weight")) + gg.geom_rug(
        gg.aes(color="weight", alpha="height", linewidth="weight")
    )
    svg = plot.to_svg()
    assert isinstance(svg, str)
    widths = {float(width) for width in re.findall(r'stroke-width="([\d.]+)"', svg)}
    # linewidth 1..7 line units (lets-plot scale_linewidth range) x 2.2 px.
    assert {2.2, 15.4} <= {round(width, 3) for width in widths}
    assert "stroke-opacity" in svg or "rgba" in svg


def test_outside_warns_about_clipping() -> None:
    with pytest.warns(GgrustWarning, match="clip"):
        layer = gg.geom_rug(outside=True)
    plot = gg.ggplot(DATA, gg.aes("height", "weight")) + layer
    _, y, _, height = panel(plot)
    for x0, y0, x1, y1 in ticks(plot):
        if x0 == x1:
            assert y1 > y0  # bottom ticks point down, away from the panel
            assert y0 == pytest.approx(y + height, abs=0.02)


@pytest.mark.parametrize(
    ("options", "match"),
    [({"sides": "bx"}, "unknown: \\['x'\\]"), ({"sides": 3}, "sides"), ({"length": -1}, "length")],
)
def test_bad_options_fail_early(options: dict[str, object], match: str) -> None:
    with pytest.raises(SpecError, match=match):
        gg.geom_rug(**options)  # ty: ignore[invalid-argument-type]


def test_engine_rejects_bad_sides_from_a_dict_spec() -> None:
    # The Python checks run at construction; a changed spec reaches the engine.
    layer = gg.geom_rug()
    layer.props()["sides"] = "q"
    with pytest.raises(SpecError, match="unknown side 'q'"):
        (gg.ggplot(DATA, gg.aes("height", "weight")) + layer).to_svg()


def test_needs_x_or_y() -> None:
    plot = gg.ggplot(DATA) + gg.geom_rug()
    with pytest.raises(SpecError, match="x and/or y"):
        plot.to_svg()
