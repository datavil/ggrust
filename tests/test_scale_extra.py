"""Distiller, desaturate, ordinal and radius scales against ggplot2 and plotnine values."""

from __future__ import annotations

import re

import ggrust as gg
import pytest
from ggrust.errors import SpecError

DATA = {"x": [1, 2, 3], "y": [1, 2, 3], "v": [0, 0.5, 1], "s": ["a", "b", "c"]}


def _circles(scale: gg.FeatureSpec, **mapping: str) -> list[str]:
    # Shape 21 draws `fill` inside a stroke; shape 16 draws `color` as its fill.
    shape = 21 if "fill" in mapping else 16
    plot = gg.ggplot(DATA, gg.aes("x", "y", **mapping)) + gg.geom_point(size=5, shape=shape)
    plot = plot + scale
    return re.findall(r"<circle[^>]*>", plot.to_svg())[:3]


def _fills(scale: gg.FeatureSpec, **mapping: str) -> list[str]:
    return [re.search(r'fill="([^"]+)"', circle)[1] for circle in _circles(scale, **mapping)]  # ty: ignore[non-subscriptable]


def test_distiller_matches_ggplot2_default_direction() -> None:
    # R: scale_colour_distiller() mapped at 0, 0.5, 1.
    assert _fills(gg.scale_color_distiller(), color="v") == ["#084594", "#6baed6", "#eff3ff"]
    assert _fills(gg.scale_color_distiller(direction=1), color="v") == [
        "#eff3ff",
        "#6baed6",
        "#084594",
    ]


def test_fill_distiller_named_palette() -> None:
    # R: scale_fill_distiller(palette = "RdBu"): 7-color RdBu, reversed.
    fills = _fills(gg.scale_fill_distiller(palette="RdBu"), fill="v")
    assert fills[0] == "#2166ac" and fills[-1] == "#b2182b"


def test_desaturate_ends_match_plotnine() -> None:
    # mizani desaturate_pal('red', .1): '#ff0000' ... '#8c7373'.
    fills = _fills(gg.scale_color_desaturate(prop=0.1), color="v")
    assert (fills[0], fills[-1]) == ("#ff0000", "#8c7373")
    fills = _fills(gg.scale_fill_desaturate(prop=0.1, reverse=True), fill="v")
    assert (fills[0], fills[-1]) == ("#8c7373", "#ff0000")


def test_desaturate_rejects_prop_outside_unit_range() -> None:
    with pytest.raises(SpecError, match=r"'prop' should be in range \[0..1\], got 1.5"):
        _circles(gg.scale_color_desaturate(prop=1.5), color="v")


def test_ordinal_colors_are_viridis_d() -> None:
    # R: scales::viridis_pal()(3) = #440154 #21908C #FDE725.
    assert _fills(gg.scale_color_ordinal(), color="s") == ["#440154", "#21918c", "#fde725"]
    assert _fills(gg.scale_fill_ordinal(), fill="s")[0] == "#440154"


def test_alpha_ordinal_spreads_the_default_range() -> None:
    circles = _circles(gg.scale_alpha_ordinal(), alpha="s")
    opacities = [re.search(r'fill-opacity="([^"]+)"', circle) for circle in circles]
    assert [float(match[1]) if match else 1.0 for match in opacities] == pytest.approx(
        [0.1, 0.55, 1.0], abs=0.003
    )


def _radii(scale: gg.FeatureSpec, column: str) -> list[float]:
    plot = gg.ggplot(DATA, gg.aes("x", "y", size=column)) + gg.geom_point() + scale
    return [float(radius) for radius in re.findall(r'<circle[^>]* r="([^"]+)"', plot.to_svg())[:3]]


def test_size_radius_is_linear_over_the_ggplot2_range() -> None:
    radii = _radii(gg.scale_size_radius(), "v")
    # Sizes 1, 3.5, 6: the radius grows linearly with the value.
    assert radii[1] / radii[0] == pytest.approx(3.5)
    assert radii[2] / radii[0] == pytest.approx(6.0)


def test_size_ordinal_steps_from_two_to_six() -> None:
    radii = _radii(gg.scale_size_ordinal(), "s")
    assert radii[1] / radii[0] == pytest.approx(2.0)
    assert radii[2] / radii[0] == pytest.approx(3.0)
