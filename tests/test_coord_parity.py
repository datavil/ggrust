"""coord_cartesian / coord_fixed limits and ratios must match lets-plot.

Compares the axis labels, every point's position as a fraction of the panel, and for
fixed ratios the panel's aspect and horizontal centre. Absolute pixels differ by a few
px (real fonts), so positions are compared relative to the panel.
"""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.plot.core import FeatureSpec  # noqa: E402
from test_breaks_parity import ggrust_labels, lets_plot_labels  # noqa: E402

FRAME = {"x": [1.0, 2.0, 3.0, 4.5], "y": [2.0, 1.0, 4.0, 3.0], "c": ["a", "b", "c", "d"]}
NUMBER = r"([\d.\-]+)"


def point(m, x="x"):
    return m.ggplot(FRAME, m.aes(x, "y")) + m.geom_point()


CASES = {
    "cartesian_xlim": lambda m: point(m) + m.coord_cartesian(xlim=[2, 3]),
    "cartesian_ylim": lambda m: point(m) + m.coord_cartesian(ylim=[0, 10]),
    "cartesian_open_side": lambda m: point(m) + m.coord_cartesian(xlim=[2, None]),
    "cartesian_and_scale_limits": lambda m: (
        point(m) + m.scale_x_continuous(limits=[1, 4]) + m.coord_cartesian(xlim=[2, 3])
    ),
    "cartesian_log": lambda m: point(m) + m.scale_y_log10() + m.coord_cartesian(ylim=[1, 10]),
    "cartesian_reverse": lambda m: point(m) + m.scale_x_reverse() + m.coord_cartesian(xlim=[2, 4]),
    "cartesian_discrete": lambda m: point(m, x="c") + m.coord_cartesian(xlim=[0.5, 2.5]),
    "fixed": lambda m: point(m) + m.coord_fixed(),
    "fixed_ratio": lambda m: point(m) + m.coord_fixed(ratio=2),
    "fixed_tall": lambda m: point(m) + m.coord_fixed(ratio=0.2),
    "fixed_limits": lambda m: point(m) + m.coord_fixed(xlim=[0, 5], ylim=[0, 5]),
    "fixed_with_legend": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y", color="c")) + m.geom_point() + m.coord_fixed()
    ),
}


def lets_plot_panel(svg: str):
    """Absolute panel rect: the plot tile's translate plus its clip rect."""
    tile = re.search(rf'<g transform="translate\({NUMBER} {NUMBER} \) ">', svg)
    clip = re.search(
        rf'<clipPath id="[^"]+"[^>]*>\s*<rect x="{NUMBER}" y="{NUMBER}" '
        rf'width="{NUMBER}" height="{NUMBER}"',
        svg,
    )
    x, y, width, height = (float(value) for value in clip.groups())
    return float(tile.group(1)) + x, float(tile.group(2)) + y, width, height


def ggrust_panel(svg: str):
    clip = re.search(
        rf'<clipPath[^>]*><rect x="{NUMBER}" y="{NUMBER}" width="{NUMBER}" height="{NUMBER}"', svg
    )
    return tuple(float(value) for value in clip.groups())


def lets_plot_points(svg: str, panel) -> list:
    """Point centres as fractions of the panel: lets-plot draws them from its origin."""
    group = svg[: svg.find("<clipPath")]  # data points come before the clip defs
    _, _, width, height = panel
    return sorted(
        (round(float(x) / width, 3), round(float(y) / height, 3))
        for x, y in re.findall(rf'<circle[^>]*cx="{NUMBER}" cy="{NUMBER}"', group)
    )


def ggrust_points(svg: str, panel) -> list:
    group = svg[svg.find('<g class="panel"') :]
    group = group[: group.find("</g>\n</g>")]
    centres = [
        (float(x), float(y)) for x, y in re.findall(rf'<circle cx="{NUMBER}" cy="{NUMBER}"', group)
    ]
    return fractions(centres, panel)


def fractions(centres, panel) -> list:
    x, y, width, height = panel
    return sorted((round((cx - x) / width, 3), round((cy - y) / height, 3)) for cx, cy in centres)


def render(module, name):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        return CASES[name](module).to_svg()


# lets-plot's panel is 0.035 px narrower than its own x mapping, so a break exactly
# at the right edge (x = 3 with xlim=[2, 3]) falls outside and is not drawn; ggrust
# draws it. Pixel quirk, not emulated.
EDGE_BREAK = {"cartesian_xlim", "cartesian_and_scale_limits"}


@pytest.mark.parametrize(
    "name",
    [
        pytest.param(name, marks=pytest.mark.xfail(reason="edge break", strict=True))
        if name in EDGE_BREAK
        else name
        for name in CASES
    ],
)
def test_coords_match_lets_plot(name):
    theirs, ours = render(lp, name), render(gg, name)
    their_panel, our_panel = lets_plot_panel(theirs), ggrust_panel(ours)
    assert ggrust_labels(ours, "x") == lets_plot_labels(theirs, "x")
    assert ggrust_labels(ours, "y") == lets_plot_labels(theirs, "y")
    for ours_point, their_point in zip(
        ggrust_points(ours, our_panel), lets_plot_points(theirs, their_panel), strict=True
    ):
        assert ours_point == pytest.approx(their_point, abs=0.005)
    if name.startswith("fixed"):
        aspect = lambda panel: panel[2] / panel[3]  # noqa: E731
        assert aspect(our_panel) == pytest.approx(aspect(their_panel), rel=0.03)
        centre = lambda panel: panel[0] + panel[2] / 2  # noqa: E731
        assert centre(our_panel) == pytest.approx(centre(their_panel), abs=8)


def test_ratio_on_coord_cartesian_warns():
    plot = point(gg) + FeatureSpec("coord", name="cartesian", ratio=2)
    with pytest.warns(gg.GgrustWarning, match="only coord_fixed has a ratio"):
        plot.to_svg()


def test_points_and_edge_breaks_otherwise_match():
    """The edge-break cases agree on everything but the break at the edge."""
    for name in EDGE_BREAK:
        theirs, ours = render(lp, name), render(gg, name)
        assert ggrust_labels(ours, "x")[:-1] == lets_plot_labels(theirs, "x")
        assert ggrust_points(ours, ggrust_panel(ours)) == pytest.approx(
            lets_plot_points(theirs, lets_plot_panel(theirs)), abs=0.005
        )


def test_reversed_limits_fail_like_lets_plot():
    with pytest.raises(ValueError, match=r"coord.ylim': expected \[low, high\]"):
        (point(gg) + gg.coord_fixed(ylim=[3, -1])).to_svg()
