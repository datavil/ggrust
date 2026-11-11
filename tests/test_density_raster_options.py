"""``geom_density`` bounds and plotnine options, ``geom_raster`` justification and smoothing."""

from __future__ import annotations

import io
import re

import ggrust as gg
import pytest
from ggrust.errors import SpecError
from PIL import Image

VALUES = [0.1, 0.4, 0.5, 1.2, 2.0, 3.5]


def test_bounds_reflect_at_zero_as_ggplot2():
    plot = gg.ggplot({"x": VALUES}, gg.aes("x")) + gg.geom_density(bounds=[0, float("inf")], bw=0.5)
    data = plot.layer_data(0)
    assert data["x"].min() == 0.0
    # R: ggplot2::layer_data(ggplot(data.frame(x = VALUES), aes(x)) +
    #     geom_density(bounds = c(0, Inf), bw = .5))$density[1] = 0.6301268
    assert data["density"][0] == pytest.approx(0.6301268, abs=1e-4)


def test_bounds_with_none_and_values_outside_are_dropped():
    plot = gg.ggplot({"x": [*VALUES, -3.0]}, gg.aes("x"))
    plot += gg.geom_density(bounds=[0, None], bw=0.5)
    assert plot.layer_data(0)["density"][0] == pytest.approx(0.6301268, abs=1e-4)


def test_bad_bounds_name_the_option():
    plot = gg.ggplot({"x": VALUES}, gg.aes("x")) + gg.geom_density(bounds=[2, 1])
    with pytest.raises(SpecError, match=r"layers\[0\]\.bounds.*expected \[low, high\]"):
        plot.to_svg()


GRID = {"x": [1, 2, 1, 2], "y": [1, 1, 2, 2], "v": [1.0, 2.0, 3.0, 4.0]}


def x_domain(**options: object) -> list[float]:
    plot = gg.ggplot(GRID, gg.aes("x", "y", fill="v")) + gg.geom_raster(**options)
    return plot.scale_info("x")[0]["limits"]


def raster_svg(**options: object) -> str:
    return (gg.ggplot(GRID, gg.aes("x", "y", fill="v")) + gg.geom_raster(**options)).to_svg()


def test_hjust_moves_cells_as_ggplot2():
    # ggplot2: xmin = x - w * (1 - hjust), xmax = x + w * hjust.
    assert x_domain() == pytest.approx([0.5, 2.5], abs=0.2)
    left = x_domain(hjust=0)
    right = x_domain(hjust=1)
    assert left[1] < x_domain()[1] < right[1]
    plot = gg.ggplot(GRID, gg.aes("x", "y", fill="v")) + gg.geom_raster(hjust=0, vjust=1)
    data = plot.layer_data(0)
    assert data["xmin"].to_list()[:2] == [0.0, 1.0]
    assert data["ymax"].to_list()[:2] == [2.0, 2.0]


def test_interpolation_drops_the_pixelated_style():
    assert "pixelated" in raster_svg()
    assert "pixelated" not in raster_svg(interpolate=True)
    assert "pixelated" not in raster_svg(interpolation="bilinear")
    assert "pixelated" in raster_svg(interpolation="nearest")


def test_interpolation_smooths_png_pixels():
    def distinct_colors(**options: object) -> int:
        plot = gg.ggplot(GRID, gg.aes("x", "y", fill="v")) + gg.geom_raster(**options)
        buffer = io.BytesIO()
        plot.to_png(buffer)
        image = Image.open(buffer).convert("RGB")
        return len(set(image.getdata()))

    assert distinct_colors(interpolate=True) > distinct_colors() + 20


def test_unknown_interpolation():
    with pytest.raises(SpecError, match=re.escape("did you mean 'bilinear'")):
        raster_svg(interpolation="bilinea")


def test_other_density_stats_do_not_read_bounds():
    plot = gg.ggplot({"g": ["a"] * 6, "x": VALUES}, gg.aes("g", "x"))
    with pytest.warns(UserWarning, match="does not support option 'bounds'"):
        bounded = (plot + gg.geom_violin(bounds=[0, None])).layer_data(0)
    assert bounded.equals((plot + gg.geom_violin()).layer_data(0))
