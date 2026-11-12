"""Accessors tier 2: ``panels()``, ``LayerSpec.defaults``, ``theme_defaults()``, ``repr``."""

from __future__ import annotations

import re
from typing import TYPE_CHECKING

import pytest
from ggrust import (
    aes,
    coord_flip,
    coord_polar,
    coord_trans,
    facet_grid,
    facet_wrap,
    geom_histogram,
    geom_point,
    geom_rect,
    geom_smooth,
    gggrid,
    ggplot,
    ggsize,
    layer,
    scale_color_brewer,
    scale_x_reverse,
    scale_y_log10,
    theme_defaults,
)
from ggrust.errors import SpecError

if TYPE_CHECKING:
    from ggrust._typing import PanelBox
    from ggrust.plot.core import PlotSpec

DATA = {
    "x": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
    "y": [10.0, 20.0, 30.0, 40.0, 50.0, 60.0],
    "kind": ["a", "a", "b", "b", "c", "c"],
}


def _pixel(panel: PanelBox, x: float, y: float) -> tuple[float, float]:
    """Map a data point (transformed units) to plot pixels, as ``panels()`` documents."""
    left, top, width, height = panel["box"]

    def fraction(axis: str, value: float) -> float:
        low, high = panel[axis]["domain"]  # type: ignore[literal-required]
        share = (value - low) / (high - low)
        return 1.0 - share if panel[axis]["reversed"] else share  # type: ignore[literal-required]

    horizontal, vertical = fraction("x", x), fraction("y", y)
    if panel["flipped"]:
        horizontal, vertical = vertical, horizontal
    return left + horizontal * width, top + (1.0 - vertical) * height


def _circles(plot: PlotSpec) -> list[tuple[float, float]]:
    svg = plot.to_svg()
    assert isinstance(svg, str)
    pattern = r'<circle[^>]*?cx="([-0-9.]+)"[^>]*?cy="([-0-9.]+)"'
    return [(float(cx), float(cy)) for cx, cy in re.findall(pattern, svg)]


@pytest.mark.parametrize(
    "extra",
    [
        [],
        [facet_wrap("kind")],
        [facet_grid(x="kind")],
        [coord_flip()],
        [scale_x_reverse()],
        [scale_y_log10()],
        [coord_trans(x="reverse")],
    ],
    ids=["plain", "facet_wrap", "facet_grid", "flip", "reverse", "log", "trans_reverse"],
)
def test_panels_map_layer_data_onto_the_drawn_points(extra: list[object]) -> None:
    plot = ggplot(DATA, aes("x", "y")) + geom_point() + ggsize(500, 300)
    for feature in extra:
        plot = plot + feature
    panels = plot.panels()
    frame = plot.layer_data(0)
    predicted = sorted(
        _pixel(panels[row["panel"]], row["x"], row["y"]) for row in frame.iter_rows(named=True)
    )
    drawn = sorted(_circles(plot))
    assert len(drawn) == len(predicted) == 6
    for (px, py), (cx, cy) in zip(predicted, drawn, strict=True):
        assert px == pytest.approx(cx, abs=0.1)
        assert py == pytest.approx(cy, abs=0.1)


def test_panels_shape_index_flip_and_reverse() -> None:
    plot = ggplot(DATA, aes("x", "y")) + geom_point() + facet_wrap("kind") + ggsize(600, 300)
    panels = plot.panels()
    assert [panel["panel"] for panel in panels] == [0, 1, 2]
    assert set(panels[0]) == {"panel", "box", "flipped", "x", "y"}
    left, top, width, height = panels[0]["box"]
    assert left > 0 and top > 0 and left + width < 600 and top + height < 300
    assert not panels[0]["flipped"]
    flipped = (ggplot(DATA, aes("x", "y")) + geom_point() + coord_flip()).panels()[0]
    assert flipped["flipped"]
    reversed_axis = (ggplot(DATA, aes("x", "y")) + geom_point() + coord_trans(x="reverse")).panels()
    assert reversed_axis[0]["x"]["reversed"] and not reversed_axis[0]["y"]["reversed"]


def test_build_panels_carry_box_flip_and_reversed() -> None:
    built = (ggplot(DATA, aes("x", "y")) + geom_point()).build()["panels"][0]
    assert len(built["box"]) == 4
    assert built["flipped"] is False
    assert built["x"]["reversed"] is False


@pytest.mark.parametrize(
    ("coord", "named"),
    [(coord_polar(), "coord_polar()"), (coord_trans(y="log10"), "coord_trans(y='log10')")],
)
def test_panels_reject_nonlinear_coords(coord: object, named: str) -> None:
    plot = ggplot(DATA, aes("x", "y")) + geom_point() + coord
    with pytest.raises(SpecError, match=re.escape(named)) as error:
        plot.panels()
    assert error.value.path == "coord"
    assert "coord_cartesian()" in str(error.value)


def test_layer_defaults_name_theme_colors_and_defaults() -> None:
    point = geom_point(size=7).defaults
    assert point["stat"] == "identity"
    assert point["position"] == "identity"
    assert point["aesthetics"]["color"] == "pen"
    assert point["aesthetics"]["size"] == 3.0  # the geom's default, not the layer's size
    assert point["aesthetics"]["alpha"] is None
    assert point["aesthetics"]["shape"] == 16
    rect = geom_rect().defaults["aesthetics"]
    assert rect["fill"] == "alpha(pen, 0.1)"
    assert rect["linetype"] == "solid"
    assert geom_histogram().defaults["stat"] == "bin"
    assert geom_histogram().defaults["position"] == "stack"
    assert (ggplot() + geom_smooth()).layers[0].defaults["stat"] == "smooth"


def test_layer_defaults_errors() -> None:
    with pytest.raises(SpecError, match="has no geom") as error:
        _ = layer().defaults
    assert error.value.path == "layer.geom"
    with pytest.raises(SpecError, match="unknown geom 'piont'; did you mean 'point'"):
        _ = layer(geom="piont").defaults


def test_theme_defaults_values_and_flavor() -> None:
    bw = theme_defaults("bw")
    assert bw["panel_background"]["color"] == "#333333"
    assert bw["legend_position"] == "right"
    assert bw["plot_margin"] == [3.0, 3.0, 3.0, 3.0]
    assert "angle" not in bw["axis_text"]  # unset (automatic) properties are left out
    assert theme_defaults("bw", flavor="darcula")["rect"]["fill"] == "#303030"
    assert theme_defaults() == theme_defaults("minimal2")
    assert theme_defaults("gray") == theme_defaults("grey")
    # A fresh dict each call.
    bw["legend_position"] = "none"
    assert theme_defaults("bw")["legend_position"] == "right"


@pytest.mark.parametrize(
    ("arguments", "path", "hint"),
    [
        (("bww",), "theme_defaults.name", "did you mean 'bw'"),
        (("bw", "dracula"), "theme_defaults.flavor", "did you mean 'darcula'"),
    ],
)
def test_theme_defaults_errors(arguments: tuple[str, ...], path: str, hint: str) -> None:
    with pytest.raises(SpecError, match=hint) as error:
        theme_defaults(*arguments)  # type: ignore[arg-type]
    assert error.value.path == path
    assert "expected one of" in str(error.value)


def test_plot_repr_is_one_readable_line() -> None:
    plot = (
        ggplot(DATA, aes("x", "y", color="kind"))
        + geom_point()
        + geom_smooth()
        + scale_color_brewer()
        + facet_wrap("kind")
    )
    assert repr(plot) == "<PlotSpec: 2 layers (point, smooth), 1 scale, facet_wrap(~kind)>"
    empty = ggplot() + facet_grid(x="a", y=["b", "c"]) + coord_flip()
    assert repr(empty) == "<PlotSpec: 0 layers, facet_grid(b + c ~ a), coord_flip()>"
    assert repr(ggplot() + geom_point()) == "<PlotSpec: 1 layer (point)>"
    assert "\n" not in repr(plot)
    assert str(plot).startswith("plot")  # str keeps the lets_plot-style dump


def test_layer_and_grid_repr() -> None:
    assert repr(geom_point(aes(color="kind"), stat="identity", size=2)) == (
        "<LayerSpec: point, stat=identity, mapping={'color': 'kind'}>"
    )
    assert repr(geom_point()) == "<LayerSpec: point>"
    assert repr(layer()) == "<LayerSpec: no geom>"
    plot = ggplot(DATA, aes("x", "y")) + geom_point()
    grid = gggrid([plot, None, plot], ncol=2)
    assert repr(grid) == "<SupPlotsSpec: 2 figures, nrow=2, ncol=2>"
