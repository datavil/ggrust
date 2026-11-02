"""`orientation="y"` (and its auto-detection on a discrete y) must match lets-plot:
horizontal bars, histograms, boxplots and violins, axis labels included.
"""

import re
import warnings

import pytest
from test_bar_parity import _ggrust_bars, _lets_plot_bars, _texts
from test_boxplot_parity import _ggrust_shapes, _lets_plot_shapes
from test_violin_parity import _ggrust_shapes as _ggrust_violins
from test_violin_parity import _lets_plot_shapes as _lets_plot_violins

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


BARS = {
    "count_auto": lambda m: m.ggplot(DATA, m.aes(y="g")) + m.geom_bar(),
    "count_explicit": lambda m: m.ggplot(DATA, m.aes(y="g")) + m.geom_bar(orientation="y"),
    "dodge": lambda m: m.ggplot(DATA, m.aes(y="g", fill="h")) + m.geom_bar(position="dodge"),
    "stack": lambda m: m.ggplot(DATA, m.aes(y="g", fill="h")) + m.geom_bar(),
    "identity": lambda m: (
        m.ggplot({"v": [3.0, -1.0, 2.0], "g": list("abc")}, m.aes("v", "g"))
        + m.geom_bar(stat="identity", orientation="y")
    ),
    "histogram": lambda m: m.ggplot(DATA, m.aes(y="y")) + m.geom_histogram(bins=4, orientation="y"),
    "histogram_fill": lambda m: (
        m.ggplot(DATA, m.aes(y="y", fill="h")) + m.geom_histogram(bins=3, orientation="y")
    ),
}


@pytest.mark.parametrize("build", BARS.values(), ids=BARS.keys())
def test_bars_match_lets_plot(build):
    expected_svg, actual_svg = _both(build)
    expected, _ = _lets_plot_bars(expected_svg)
    actual = _ggrust_bars(actual_svg)
    assert len(actual) == len(expected)
    for got, want in sorted(zip(sorted(actual), sorted(expected), strict=True)):
        assert got[:4] == pytest.approx(want[:4], abs=1e-3), (got, want)
        assert got[4:] == want[4:]
    assert sorted(_texts(actual_svg)) == sorted(_texts(expected_svg))


SHAPES = {
    "boxplot_auto": lambda m: m.ggplot(DATA, m.aes("y", "g")) + m.geom_boxplot(),
    "boxplot_explicit": lambda m: m.ggplot(DATA, m.aes("y", "g")) + m.geom_boxplot(orientation="y"),
}


@pytest.mark.parametrize("build", SHAPES.values(), ids=SHAPES.keys())
def test_shapes_match_lets_plot(build):
    expected_svg, actual_svg = _both(build)
    expected, actual = _lets_plot_shapes(expected_svg), _ggrust_shapes(actual_svg)
    expected = [shape for shape in expected if shape[0] != "line" or shape[2][1] != 1.0]
    assert sorted(shape[0] for shape in actual) == sorted(shape[0] for shape in expected)
    for got, want in zip(actual, expected, strict=True):
        assert got[1] == pytest.approx(want[1], abs=1e-3), (got, want)
    assert sorted(_texts(actual_svg)) == sorted(_texts(expected_svg))


VIOLINS = {
    "auto": lambda m: m.ggplot(DATA, m.aes("y", "g")) + m.geom_violin(bw=0.8),
    "explicit": lambda m: (
        m.ggplot(DATA, m.aes("y", "g")) + m.geom_violin(bw=0.8, orientation="y", n=32)
    ),
    "quantile_lines": lambda m: (
        m.ggplot(DATA, m.aes("y", "g"))
        + m.geom_violin(bw=0.8, quantile_lines=True, quantiles=[0.25, 0.5, 0.75])
    ),
}


@pytest.mark.parametrize("build", VIOLINS.values(), ids=VIOLINS.keys())
def test_violins_match_lets_plot(build):
    expected_svg, actual_svg = _both(build)
    expected, actual = _lets_plot_violins(expected_svg), _ggrust_violins(actual_svg)
    assert len(actual) == len(expected), (len(actual), len(expected))
    for got, want in zip(actual, expected, strict=True):
        assert got[0] == want[0] and got[2] == want[2], (got[2], want[2])
        assert [value for point in got[1] for value in point] == pytest.approx(
            [value for point in want[1] for value in point], abs=1e-3
        )
    assert sorted(_texts(actual_svg)) == sorted(_texts(expected_svg))


FLIPPED = {
    # Two-point boxes: lets-plot's quartiles differ from R type 7 (a recorded
    # deviation), so the y orientation is checked against coord_flip instead.
    "boxplot_dodge": (
        lambda m: m.ggplot(DATA, m.aes("y", "g", fill="h")) + m.geom_boxplot(),
        lambda m: m.ggplot(DATA, m.aes("g", "y", fill="h")) + m.geom_boxplot() + m.coord_flip(),
    ),
    "violin_dodge": (
        lambda m: m.ggplot(DATA, m.aes("y", "g", fill="h")) + m.geom_violin(bw=0.8, n=16),
        lambda m: (
            m.ggplot(DATA, m.aes("g", "y", fill="h")) + m.geom_violin(bw=0.8, n=16) + m.coord_flip()
        ),
    ),
    "bar_stack_flip": (
        lambda m: m.ggplot(DATA, m.aes(y="g", fill="h")) + m.geom_bar(),
        lambda m: m.ggplot(DATA, m.aes("g", fill="h")) + m.geom_bar() + m.coord_flip(),
    ),
}


@pytest.mark.parametrize("builds", FLIPPED.values(), ids=FLIPPED.keys())
def test_y_orientation_draws_as_coord_flip(builds):
    oriented, flipped = (
        (build(gg) + gg.theme(legend_position="none")).to_svg() for build in builds
    )
    for shapes in (_ggrust_bars, _ggrust_shapes, _ggrust_violins):
        got, want = shapes(oriented), shapes(flipped)
        assert len(got) == len(want)
        assert _flat(got) == pytest.approx(_flat(want), abs=1e-3)


def _flat(value: object) -> list[object]:
    """Numbers and strings of nested tuples and lists, in order."""
    if isinstance(value, list | tuple):
        return [item for part in value for item in _flat(part)]
    return [float(value)] if isinstance(value, int | float) else [value]


def test_stat_variable_on_the_count_axis():
    # Bug hunt: the default `..count..` overwrote a mapped `x="..prop.."`.
    def bars(module):
        plot = module.ggplot(DATA, module.aes(y="g", x="..prop..")) + module.geom_bar()
        return _ggrust_bars(plot.to_svg()) if module is gg else _lets_plot_bars(plot.to_svg())[0]

    assert _flat(sorted(bars(gg))) == pytest.approx(_flat(sorted(bars(lp))), abs=1e-3)


def test_missing_aesthetic_is_named_as_mapped():
    plot = gg.ggplot(DATA, gg.aes(x="g")) + gg.geom_bar(orientation="y")
    with pytest.raises(ValueError, match=re.escape("requires aesthetics y with orientation='y'")):
        plot.to_svg()


def test_bad_orientation_is_an_error():
    plot = gg.ggplot(DATA, gg.aes(y="g")) + gg.geom_bar(orientation="z")
    with pytest.raises(ValueError, match=re.escape("orientation must be 'x' or 'y'")):
        plot.to_svg()
