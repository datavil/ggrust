"""Arrow axes: ``element_line(arrow=, extent=)`` on axis lines (ggplot2, not lets-plot).

Heads and the truncated line are drawn by the layout in pixels, so they do not depend
on the data range and stay put in HTML zoom (the runtime stamps the axis line as is).
"""

import io
import json
import re

import ggrust as gg
import numpy as np
import polars as pl
import pytest
from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.core import PlotSpec
from PIL import Image

FRAME = pl.DataFrame({"x": [0.0, 1.0, 2.0, 10.0], "y": [0.0, 3.0, 1.0, 2.0]})
NUMBER = r"(-?[\d.]+)"


def corner(frame: pl.DataFrame = FRAME) -> PlotSpec:
    """A classic-theme plot of ``frame`` with corner arrows and titles next to them."""
    options = {"arrow": gg.arrow(length=8, type="closed"), "extent": 0.25}
    return (
        gg.ggplot(frame, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.theme_classic()
        + gg.theme(
            axis_line=gg.element_line(**options),
            axis_text=gg.element_blank(),
            axis_ticks=gg.element_blank(),
            axis_title_x=gg.element_text(hjust=0),
            axis_title_y=gg.element_text(hjust=0),
        )
        + gg.ggsize(400, 300)
    )


def axis_line_paths(svg: str, axis: str) -> list[list[tuple[float, float]]]:
    """Points of each path in the ``axis-line`` group of axis ``x`` or ``y``."""
    group = re.search(rf'<g class="axis-{axis}">\s*<g class="axis-line">(.*?)</g>', svg, re.S)
    assert group, f"no arrow axis line on axis {axis}"
    return [
        [(float(x), float(y)) for x, y in re.findall(rf"[ML]{NUMBER} {NUMBER}", d)]
        for d in re.findall(r'd="([^"]*)"', group.group(1))
    ]


def panel_of(svg: str) -> tuple[float, float, float, float]:
    match = re.search(
        rf'<clipPath[^>]*>\s*<rect x="{NUMBER}" y="{NUMBER}" width="{NUMBER}" height="{NUMBER}"',
        svg,
    )
    assert match
    return tuple(float(value) for value in match.groups())


def test_element_line_carries_arrow_and_extent_into_the_spec():
    theme = gg.theme(axis_line=gg.element_line(arrow=gg.arrow(length=8), extent=0.3))
    assert theme.as_dict()["axis_line"] == {
        "blank": False,
        "arrow": {"name": "arrow", "length": 8},
        "extent": 0.3,
    }
    # Without them the element is what lets_plot sends.
    assert gg.element_line(color="red") == {
        "color": "red",
        "size": None,
        "linetype": None,
        "blank": False,
        "arrow": None,
        "extent": None,
    }


@pytest.mark.parametrize("extent", [0, 1.5, -0.1, "half", True])
def test_bad_extents_fail_early_with_the_accepted_range(extent):
    with pytest.raises(SpecError, match=r"0 < extent <= 1") as caught:
        gg.element_line(extent=extent)
    assert caught.value.path == "element_line.extent"


def test_a_bad_arrow_fails_early():
    with pytest.raises(UnsupportedValueError, match=r"expected arrow\(...\), got dict") as caught:
        gg.element_line(arrow={"length": 8})
    assert caught.value.path == "element_line.arrow"


@pytest.mark.parametrize(
    ("line", "message"),
    [
        ({"extent": 2}, "at 'theme.axis_line_x': extent: expected a fraction"),
        ({"arrow": {"type": "filled"}}, "arrow: type: expected one of open, closed"),
        ({"arrow": {"angle": -30}}, "arrow: angle: expected a non-negative number"),
        ({"arrow": 3}, "arrow: expected arrow(), got 3"),
    ],
)
def test_the_engine_checks_raw_theme_values(line, message):
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point() + gg.theme(axis_line_x=line)
    with pytest.raises(SpecError, match=re.escape(message)):
        plot.to_svg()


def test_arrows_on_other_lines_warn_and_are_ignored():
    plot = (
        gg.ggplot(FRAME, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.theme(panel_grid=gg.element_line(arrow=gg.arrow()))
    )
    with pytest.warns(
        gg.GgrustWarning, match=r"theme.panel_grid.arrow': only axis lines take arrow"
    ):
        svg = plot.to_svg()
    assert 'class="axis-line"' not in svg


def test_arrows_are_pixels_from_the_origin_corner_whatever_the_data():
    small = corner()
    large = corner(FRAME.with_columns(pl.col("x") * 1000, pl.col("y") * 1e-3))
    for plot in [small, large]:
        svg = plot.to_svg()
        left, top, width, height = panel_of(svg)
        bottom = top + height
        x_line, x_head = axis_line_paths(svg, "x")
        y_line, y_head = axis_line_paths(svg, "y")
        assert x_line[0] == pytest.approx((left, bottom), abs=0.01)
        assert y_line[0] == pytest.approx((left, bottom), abs=0.01)
        # The tip is at a quarter of the panel; the line stops one width short.
        assert x_head[1] == pytest.approx((left + width / 4, bottom), abs=0.01)
        assert y_head[1] == pytest.approx((left, bottom - height / 4), abs=0.01)
        assert x_line[1][0] == pytest.approx(left + width / 4 - 1, abs=0.01)
        # 8 px barbs at 30 degrees.
        for head in [x_head, y_head]:
            (bx, by), (tx, ty) = head[0], head[1]
            assert np.hypot(bx - tx, by - ty) == pytest.approx(8, abs=0.01)


def test_titles_sit_next_to_the_short_arrows():
    svg = corner().to_svg()
    left, top, width, height = panel_of(svg)
    titles = re.search(r'<g class="titles">(.*?)</g>', svg, re.S).group(1)
    x_title = re.search(rf'<text x="{NUMBER}" y="{NUMBER}"[^>]*>x</text>', titles)
    # hjust=0: the title starts at the arrow's origin, just below the line.
    assert float(x_title.group(2)) - (top + height) < 25
    assert float(x_title.group(1)) < left + width / 4


def test_png_draws_the_arrow_heads():
    buffer = io.BytesIO()
    corner().to_png(buffer, scale=1)
    image = np.asarray(Image.open(io.BytesIO(buffer.getvalue())).convert("L"))
    svg = corner().to_svg()
    x_head = axis_line_paths(svg, "x")[1]
    # A closed head is filled: its centre (between barbs and tip) is dark.
    cx = (x_head[0][0] + x_head[1][0]) / 2
    cy = x_head[1][1]
    assert image[round(cy), round(cx) - 1] < 128
    # Past the extent the bottom edge of the panel is empty.
    left, top, width, height = panel_of(svg)
    assert image[round(top + height), round(left + width * 0.75)] > 200


def test_html_zoom_payload_keeps_the_arrows_as_drawn():
    plot = corner() + gg.ggtb()
    html = plot._repr_html_()
    payload = json.loads(re.search(r'class="ggrust-zoom">(.*?)</script>', html).group(1))
    (tile,) = payload["plots"][0]["tiles"]
    svg = corner().to_svg()
    for axis, key in [("x", "horizontal"), ("y", "vertical")]:
        fragment = tile[key]["line"]
        # The runtime stamps this template unchanged at every zoom level.
        assert fragment.count("<path") == 2 and 'class="axis-line"' in fragment
        drawn = re.search(rf'<g class="axis-{axis}">\s*(<g class="axis-line">.*?</g>)', svg, re.S)
        assert re.sub(r"\s", "", fragment) == re.sub(r"\s", "", drawn.group(1))
