"""coord_flip must match lets-plot: shapes, axis labels, and which theme elements
style which axis (lets-plot keeps them with the aesthetic, so the x title and
`axis_text_x` move to the left axis).
"""

import re
import warnings

import pytest
from test_bar_parity import _ggrust_bars, _lets_plot_bars
from test_boxplot_parity import _ggrust_shapes, _lets_plot_shapes

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

DATA = {
    "g": list("aaabbbbbccc"),
    "h": list("uvuvuvuvuvu"),
    "y": [1.0, 2, 3, 2, 3, 4, 5, 9, 1, 2, 3],
}


def _both(build):
    return [(build(m) + m.theme(legend_position="none")).to_svg() for m in (lp, gg)]


@pytest.mark.parametrize(
    "build",
    [
        lambda m: m.ggplot(DATA, m.aes("g")) + m.geom_bar() + m.coord_flip(),
        lambda m: (
            m.ggplot(DATA, m.aes("g", fill="h")) + m.geom_bar(position="dodge") + m.coord_flip()
        ),
        lambda m: m.ggplot(DATA, m.aes("y")) + m.geom_histogram(bins=4) + m.coord_flip(),
        lambda m: m.ggplot(DATA, m.aes("g")) + m.geom_bar() + m.coord_cartesian(flip=True),
    ],
)
def test_flipped_bars_match_lets_plot(build):
    expected_svg, actual_svg = _both(build)
    expected, _ = _lets_plot_bars(expected_svg)
    actual = _ggrust_bars(actual_svg)
    assert len(actual) == len(expected)
    for got, want in zip(actual, expected, strict=True):
        assert got[:4] == pytest.approx(want[:4], abs=1e-3), (got, want)
        assert got[4:] == want[4:]


@pytest.mark.parametrize(
    "build",
    [
        lambda m: m.ggplot(DATA, m.aes("g", "y")) + m.geom_boxplot() + m.coord_flip(),
        lambda m: (
            m.ggplot(DATA, m.aes("g", "y"))
            + m.geom_point()
            + m.geom_hline(yintercept=2.5)
            + m.coord_flip()
        ),
        lambda m: (
            m.ggplot(DATA, m.aes("y", "y"))
            + m.geom_point()
            + m.geom_vline(xintercept=2)
            + m.coord_flip(xlim=[0, 6])
        ),
    ],
)
def test_flipped_shapes_match_lets_plot(build):
    expected_svg, actual_svg = _both(build)
    expected, actual = _lets_plot_shapes(expected_svg), _ggrust_shapes(actual_svg)
    expected = [shape for shape in expected if shape[0] != "line" or shape[2][1] != 1.0]
    assert [shape[0] for shape in actual] == [shape[0] for shape in expected]
    for got, want in zip(actual, expected, strict=True):
        assert got[1] == pytest.approx(want[1], abs=1e-3), (got, want)


def test_titles_and_theme_follow_the_aesthetic():
    plot = (
        gg.ggplot(DATA, gg.aes("g", "y"))
        + gg.geom_point()
        + gg.coord_flip()
        + gg.labs(x="group", y="value")
        + gg.theme(axis_text_x=gg.element_text(color="red"))
    )
    svg = plot.to_svg()
    left = svg[svg.index('<g class="axis-y"') :]
    left = left[: left.index("</g>")]
    bottom = svg[svg.index('<g class="axis-x"') :]
    bottom = bottom[: bottom.index("</g>")]
    # The groups (data x) are on the left, styled by axis_text_x.
    assert re.findall(r">([^<>]+)</text>", left) == ["a", "b", "c"]
    assert 'fill="#ff0000"' in left and 'fill="#ff0000"' not in bottom
    titles = svg[svg.index('<g class="titles"') :]
    assert re.search(r"rotate\(-90[^>]*>group</text>", titles), titles
    assert ">value</text>" in titles
