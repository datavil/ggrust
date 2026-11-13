"""Infinite positions are drawn at the panel edges (ggplot2), not dropped.

The panel of these plots (``theme_void``, 400 x 300) spans x 6..394 and y 6..294.
"""

import re
import warnings

import pytest
from ggrust import (
    aes,
    annotate,
    coord_fixed,
    coord_flip,
    coord_trans,
    facet_wrap,
    geom_area,
    geom_hline,
    geom_point,
    geom_text,
    geom_vline,
    ggplot,
    ggsize,
    scale_x_reverse,
    scale_y_log10,
    theme_void,
    ylim,
)
from ggrust._render import to_json
from ggrust.plot.core import PlotSpec

INF = float("inf")
DATA = {"x": [1.0, 2.0, 5.0], "y": [10.0, 100.0, 1000.0], "g": ["a", "b", "b"]}
LEFT, RIGHT, TOP, BOTTOM = 6.0, 394.0, 6.0, 294.0


def _plot() -> PlotSpec:
    return ggplot(DATA, aes("x", "y")) + geom_point() + theme_void() + ggsize(400, 300)


def _svg(plot: PlotSpec) -> str:
    """The SVG, failing on any warning (no row may be reported as removed)."""
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        return plot.to_svg()


def _text(svg: str, label: str) -> tuple[float, float]:
    match = re.search(rf'<text x="([-\d.]+)" y="([-\d.]+)"[^>]*>{re.escape(label)}</text>', svg)
    assert match, f"{label!r} not drawn"
    return float(match[1]), float(match[2])


def _rects(svg: str) -> list[tuple[float, float, float, float]]:
    panel = svg[svg.index('<g class="panel"') :]
    return [
        tuple(float(value) for value in match)
        for match in re.findall(
            r'<rect x="([-\d.]+)" y="([-\d.]+)" width="([-\d.]+)" height="([-\d.]+)"', panel
        )
    ]


def test_text_in_the_bottom_right_corner() -> None:
    plot = _plot() + annotate("text", x=INF, y=-INF, label="N = 82", hjust=1, vjust=0)
    x, y = _text(_svg(plot), "N = 82")
    # Right-aligned to the right edge (centre anchor, so left of it), on the bottom.
    assert RIGHT - 40 < x < RIGHT
    assert BOTTOM - 1 < y <= BOTTOM + 1e-9


def test_infinite_text_does_not_train_the_scales() -> None:
    plot = _plot() + annotate("text", x=INF, y=-INF, label="N")
    assert plot.scale_info("x")[0]["limits"] == (1.0, 5.0)
    assert plot.scale_info("y")[0]["limits"] == (10.0, 1000.0)


def test_text_constants_and_mapped_columns() -> None:
    constant = _plot() + geom_text(x=-INF, y=INF, label="corner")
    assert _text(_svg(constant), "corner")[0] == pytest.approx(LEFT, abs=30)
    mapped = ggplot({"x": [1.0, -INF], "y": [1.0, INF], "label": ["a", "b"]}) + geom_text(
        aes("x", "y", label="label")
    )
    _svg(mapped)


@pytest.mark.parametrize("log", [False, True])
def test_rect_spans_the_panel_height(log: bool) -> None:
    plot = _plot() + annotate("rect", xmin=2, xmax=3, ymin=-INF, ymax=INF, alpha=0.2)
    if log:
        plot += scale_y_log10()
    rects = [rect for rect in _rects(_svg(plot)) if rect[1] == TOP]
    assert len(rects) == 1
    assert rects[0][3] == pytest.approx(BOTTOM - TOP)


def test_log_scale_keeps_infinity_and_censors_finite_values_only() -> None:
    plot = (
        _plot()
        + scale_y_log10()
        + annotate("text", x=3, y=INF, label="top")
        + annotate("text", x=3, y=-INF, label="bottom")
    )
    svg = _svg(plot)
    assert _text(svg, "top")[1] == pytest.approx(TOP, abs=10)
    assert _text(svg, "bottom")[1] == pytest.approx(BOTTOM, abs=10)
    # Limits drop finite values outside them, but keep infinite ones.
    limited = _plot() + ylim(0, 500) + annotate("text", x=3, y=INF, label="kept")
    with pytest.warns(UserWarning, match="removed 1 rows"):
        svg = limited.to_svg()
    assert _text(svg, "kept")[1] == pytest.approx(TOP, abs=10)


def test_reversed_scale_flips_the_edges() -> None:
    plot = _plot() + scale_x_reverse() + annotate("text", x=INF, y=3, label="end")
    assert _text(_svg(plot), "end")[0] == pytest.approx(LEFT, abs=10)


def test_reference_lines_at_infinity() -> None:
    plot = (
        _plot()
        + geom_hline(yintercept=INF, color="red")
        + geom_vline(xintercept=-INF, color="blue")
        + annotate("segment", x=-INF, xend=INF, y=100, yend=100)
    )
    svg = _svg(plot)
    assert '<path d="M6 6L394 6" fill="none" stroke="#ff0000"' in svg
    assert '<path d="M6 294L6 6" fill="none" stroke="#0000ff"' in svg
    assert re.search(r'<path d="M6 [\d.]+L394 [\d.]+"', svg)


@pytest.mark.parametrize(
    "extra",
    [coord_flip(), coord_fixed(), facet_wrap("g")],
    ids=["flip", "fixed", "facet"],
)
def test_coords_and_facets_draw_infinite_rows(extra: object) -> None:
    plot = (
        _plot()
        + annotate("text", x=INF, y=-INF, label="corner")
        + annotate("rect", xmin=-INF, xmax=INF, ymin=50, ymax=60)
        + geom_hline(yintercept=INF)
        + extra
    )
    assert "corner" in _svg(plot)


def test_non_finite_numbers_elsewhere_are_still_dropped() -> None:
    # Only position constants carry infinity; other numbers stay null.
    spec = (_plot() + geom_point(size=INF, x=INF)).as_dict()
    sent = to_json(spec)
    assert '"x": {"$float": "inf"}' in sent
    assert '"size": null' in sent


@pytest.mark.parametrize("geom", ["pointrange", "linerange", "crossbar", "errorbar"])
def test_interval_geoms_draw_infinite_rows(geom: str) -> None:
    import ggrust

    def count(x_last: list[float]) -> int:
        data = {"x": [1.0, 2.0, *x_last], "y": [1.0, 3.0, 2.0][: 2 + len(x_last)]}
        layer = getattr(ggrust, f"geom_{geom}")(aes(ymin="y", ymax="y"))
        svg = _svg(ggplot(data, aes("x", "y")) + layer + theme_void())
        return len(re.findall(r"<(?:path|circle|rect)\b", svg[svg.index('class="panel"') :]))

    one_row = count([3.0]) - count([])
    assert count([INF]) - count([]) == one_row


def test_area_reaches_the_panel_edge() -> None:
    data = {"x": [1.0, 2.0, 5.0, INF], "y": [10.0, 100.0, 1000.0, 1000.0]}
    svg = _svg(ggplot(data, aes("x", "y")) + geom_area() + theme_void() + ggsize(400, 300))
    outline = re.search(r'<path d="([^"]*)"', svg[svg.index('class="panel"') :])
    assert outline and f"L{RIGHT:g} " in outline[1]


def test_reversed_coord_trans_flips_the_edges() -> None:
    plot = _plot() + coord_trans(x="reverse") + annotate("text", x=INF, y=100, label="end")
    assert _text(_svg(plot), "end")[0] == pytest.approx(LEFT, abs=10)
