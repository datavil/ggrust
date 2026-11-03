"""Geom colors follow the theme as in lets-plot: pen, brush and paper.

Geom defaults, the color names ``"pen"``, ``"brush"`` and ``"paper"`` and
``theme(geom=element_geom(...))`` are compared with lets-plot's rendered colors.
"""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

FLAVORS = [
    "standard",
    "darcula",
    "solarized_light",
    "solarized_dark",
    "high_contrast_light",
    "high_contrast_dark",
]
COLORS = [{}, {"color": "pen", "fill": "brush"}, {"color": "paper", "fill": "pen"}]


def point_colors(module, *extra, **options) -> tuple[str, str]:
    """(fill, stroke) of one outlined point."""
    plot = module.ggplot() + module.geom_point(x=1, y=1, shape=21, size=8, **options)
    for feature in extra:
        plot = plot + feature
    svg = plot.to_svg()
    if module is lp:
        return re.search(r'<circle fill="([^"]+)" stroke="([^"]+)"', svg).groups()
    before = svg[: svg.find("<circle")]
    return re.findall(r'<g fill="([^"]+)" stroke="([^"]+)"', before)[-1]


@pytest.mark.parametrize("flavor", FLAVORS)
@pytest.mark.parametrize("options", COLORS)
def test_point_colors_follow_the_flavor(flavor, options):
    theirs = point_colors(lp, getattr(lp, f"flavor_{flavor}")(), **options)
    ours = point_colors(gg, getattr(gg, f"flavor_{flavor}")(), **options)
    assert ours == theirs


@pytest.mark.parametrize("options", COLORS)
def test_element_geom_sets_the_geom_colors(options):
    def geom(module):
        return module.theme(
            geom=module.element_geom(pen="#000080", brush="#ffa500", paper="#ffffe0")
        )

    assert point_colors(gg, geom(gg), **options) == point_colors(lp, geom(lp), **options)


def test_element_geom_over_a_flavor_keeps_the_other_colors():
    def features(module):
        return module.flavor_darcula(), module.theme(geom=module.element_geom(brush="#ffa500"))

    ours = point_colors(gg, *features(gg), color="pen", fill="brush")
    assert ours == point_colors(lp, *features(lp), color="pen", fill="brush")


def test_default_bars_and_boxes_follow_the_flavor():
    data = {"x": ["a", "b", "b"], "y": [1.0, 2.0, 3.0]}
    plot = gg.ggplot(data, gg.aes("x", "y")) + gg.geom_boxplot() + gg.flavor_darcula()
    svg = plot.to_svg()
    assert 'fill="#303030" stroke="#bbbbbb"' in svg  # paper fill, pen outline


def test_element_geom_rejects_a_bad_color():
    plot = gg.ggplot() + gg.geom_point(x=1, y=1) + gg.theme(geom=gg.element_geom(pen="reddish"))
    with pytest.raises(SpecError, match="invalid color 'reddish'") as caught:
        plot.to_svg()
    assert caught.value.path == "theme.geom.pen"


def test_element_geom_does_not_warn():
    plot = gg.ggplot() + gg.geom_point(x=1, y=1) + gg.theme(geom=gg.element_geom(pen="navy"))
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        plot.to_svg()
