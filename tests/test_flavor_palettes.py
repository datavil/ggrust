"""Theme flavors must leave scale palettes alone, as in lets-plot.

A flavor changes the theme's symbolic colors and the geom defaults (pen, paper). The
colors a scale maps data to, including the default ``na_value``, stay the same.
"""

import re
import warnings
from collections.abc import Callable
from types import ModuleType

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

FRAME = {
    "x": [1.0, 2.0, 3.0, 4.0],
    "y": [1.0, 2.0, 3.0, 4.0],
    "value": [1.0, 2.0, 3.0, None],
    "name": ["a", "b", "c", None],
}
FLAVORS = [
    "flavor_darcula",
    "flavor_solarized_light",
    "flavor_solarized_dark",
    "flavor_high_contrast_light",
    "flavor_high_contrast_dark",
]
SCALES: dict[str, tuple[str, Callable[[ModuleType], object] | None]] = {
    "discrete_default": ("name", None),
    "continuous_default": ("value", None),
    "hue": ("name", lambda m: m.scale_color_hue()),
    "grey": ("name", lambda m: m.scale_color_grey()),
    "brewer": ("name", lambda m: m.scale_color_brewer(palette="Dark2")),
    "viridis": ("value", lambda m: m.scale_color_viridis()),
    "gradient": ("value", lambda m: m.scale_color_gradient(low="red", high="blue")),
    "gradient2": ("value", lambda m: m.scale_color_gradient2()),
    "manual": ("name", lambda m: m.scale_color_manual(values=["red", "green", "blue"])),
}


def _hex(color: str) -> str:
    match = re.fullmatch(r"rgb\((\d+),\s*(\d+),\s*(\d+)\)", color)
    if match is None:
        return color.lower()
    red, green, blue = (int(channel) for channel in match.groups())
    return f"#{red:02x}{green:02x}{blue:02x}"


def _point_colors(module: ModuleType, scale: str, flavor: str | None) -> set[str]:
    column, make_scale = SCALES[scale]
    plot = module.ggplot(FRAME, module.aes("x", "y", color=column)) + module.geom_point(size=5)
    if make_scale is not None:
        plot += make_scale(module)
    if flavor is not None:
        plot += getattr(module, flavor)()
    colors = re.findall(r'<circle\b[^>]*?(?:fill|stroke)="([^"]+)"', plot.to_svg())
    assert colors
    return {_hex(color) for color in colors}


@pytest.mark.filterwarnings("ignore::UserWarning")
@pytest.mark.parametrize("flavor", FLAVORS)
@pytest.mark.parametrize("scale", SCALES)
def test_flavor_keeps_scale_colors(scale: str, flavor: str) -> None:
    standard = _point_colors(gg, scale, None)
    assert _point_colors(gg, scale, flavor) == standard
    assert _point_colors(lp, scale, flavor) == standard


def test_flavor_still_changes_the_background() -> None:
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point()
    assert (plot + gg.flavor_darcula()).to_svg() != plot.to_svg()
