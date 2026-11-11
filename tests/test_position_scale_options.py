"""Positional scale options: ``break_width`` on number axes and ``na_value``.

``break_width`` is compared with lets-plot 4.11 (``to_svg()``); ``na_value`` follows
ggplot2, because lets-plot documents it but drops missing positions anyway.
"""

from __future__ import annotations

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402
from test_breaks_parity import ggrust_labels, lets_plot_labels  # noqa: E402


def x_labels(module: object, low: float, high: float, **scale: object) -> list[str]:
    """The x tick labels of a two-point scatter with `scale_x_continuous(**scale)`."""
    frame = {"x": [low, high], "y": [0, 1]}
    plot = module.ggplot(frame, module.aes("x", "y")) + module.geom_point()
    plot += module.scale_x_continuous(**scale)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        svg = plot.to_svg()
    reader = lets_plot_labels if module is lp else ggrust_labels
    return reader(svg, "x")


BREAK_WIDTH_CASES = [
    (0, 10, 3, {}),
    (0.5, 10, 3, {}),
    (-7, 13, 5, {}),
    (0.13, 0.91, 0.25, {}),
    (3, 1003, 250, {}),
    (17, 23, 4, {}),
    (0, 10, 10.25, {}),
    (0, 10, 0.4, {}),
    (0, 10, 3, {"limits": [2, 9]}),
    (1, 10000, 2, {"trans": "log10"}),
    (1, 400, 4, {"trans": "sqrt"}),
    (0, 10, 3, {"trans": "reverse"}),
    (5, 5, 1, {}),
]


@pytest.mark.parametrize(("low", "high", "width", "scale"), BREAK_WIDTH_CASES)
def test_break_width_matches_lets_plot(
    low: float, high: float, width: float, scale: dict[str, object]
) -> None:
    expected = x_labels(lp, low, high, break_width=width, **scale)
    assert x_labels(gg, low, high, break_width=width, **scale) == expected


def test_break_width_is_laid_out_like_fixed_breaks() -> None:
    # lets-plot draws break_width=1 over -5..105 exactly as breaks=[-5, ..., 105]
    # (crowded fixed labels: lets-plot thins them, ggrust turns them vertical).
    fixed = list(range(-5, 106))
    assert x_labels(lp, 0, 100, break_width=1) == x_labels(lp, 0, 100, breaks=fixed)
    assert x_labels(gg, 0, 100, break_width=1) == x_labels(gg, 0, 100, breaks=fixed)


def test_breaks_win_over_break_width() -> None:
    assert x_labels(gg, 0, 10, breaks=[1, 2], break_width=3) == ["1", "2"]


def test_tiny_break_width_is_widened_instead_of_hanging() -> None:
    labels = x_labels(gg, 0, 1e6, break_width=1e-6)
    assert 0 < len(labels) <= 10_000


@pytest.mark.parametrize("width", [0, -3, "wide"])
def test_invalid_break_width_fails_early(width: object) -> None:
    plot = gg.ggplot({"x": [0, 10], "y": [0, 1]}, gg.aes("x", "y")) + gg.geom_point()
    plot += gg.scale_x_continuous(break_width=width)
    if isinstance(width, str):
        # A time interval on a number axis is reported and ignored, as before.
        with pytest.warns(UserWarning, match="break_width"):
            plot.to_svg()
        return
    with pytest.raises(SpecError, match=r"break_width.*positive number"):
        plot.to_svg()


def circle_centres(svg: str) -> list[tuple[float, float]]:
    """Centres of the data circles (ggrust draws points as `<circle>`)."""
    return [
        (float(x), float(y))
        for x, y in re.findall(r'<circle[^>]*cx="([-\d.]+)"[^>]*cy="([-\d.]+)"', svg)
    ]


def point_svg(frame: dict[str, list[object]], *layers: object) -> str:
    plot = gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_point() + gg.theme_void()
    for layer in layers:
        plot += layer
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        return plot.to_svg()


def test_na_value_draws_missing_positions_there() -> None:
    # ggplot2: the missing x is drawn at 0 and the axis trains on it.
    frame = {"x": [1, 2, None, 4], "y": [1, 2, 3, 4]}
    dropped = circle_centres(point_svg(frame))
    placed = circle_centres(point_svg(frame, gg.scale_x_continuous(na_value=0)))
    assert len(dropped) == 3
    assert len(placed) == 4
    xs = sorted(x for x, _ in placed)
    # 0, 1, 2, 4 evenly mapped: the gap 2 -> 4 is twice the gap 0 -> 1.
    assert xs[3] - xs[2] == pytest.approx(2 * (xs[1] - xs[0]), abs=0.05)


def test_na_value_also_takes_values_outside_the_limits() -> None:
    frame = {"x": [1, 2, 4], "y": [1, 2, 3]}
    placed = circle_centres(point_svg(frame, gg.scale_x_continuous(limits=[1, 3], na_value=3)))
    assert len(placed) == 3
    by_y = sorted(placed, key=lambda centre: -centre[1])
    # ggplot2 censors x = 4, then draws it at 3: the right end of the limits.
    assert by_y[2][0] > by_y[1][0]


def test_na_value_on_a_log_axis_must_be_in_its_domain() -> None:
    frame = {"x": [1, None, 100], "y": [1, 2, 3]}
    with pytest.raises(SpecError, match="na_value"):
        point_svg(frame, gg.scale_x_log10(na_value=0))
    assert len(circle_centres(point_svg(frame, gg.scale_x_log10(na_value=10)))) == 3


def test_a_log_axis_keeps_values_up_to_the_largest_double() -> None:
    # lets-plot drops values above f64::MAX / 1e5 (about 1.8e303); ggplot2 keeps them.
    frame = {"x": [1e300, 1e304, 1e308], "y": [1, 2, 3]}
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        assert len(circle_centres(point_svg(frame, gg.scale_x_log10()))) == 3
    plot = gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_point() + gg.scale_x_log10()
    # ggplot2 4.0.1 x.range: 299.6, 308.4.
    assert plot.scale_info("x")[0]["domain"] == pytest.approx((299.6, 308.4))


def test_na_value_on_a_discrete_axis_is_reported() -> None:
    frame = {"x": ["a", None, "b"], "y": [1, 2, 3]}
    plot = gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_point()
    plot += gg.scale_x_discrete(na_value="a")
    with pytest.warns(UserWarning, match="na_value applies to continuous axes"):
        plot.to_svg()


LABLIM_FRAME = {"x": ["alphabet", "be", "gamma", "ε-épsilon"], "y": [1, 2, 3, 4]}

LABLIM_CASES = {
    "discrete": lambda m: m.scale_x_discrete(lablim=3),
    "labels": lambda m: m.scale_x_discrete(lablim=3, labels=["first label", "b", "c", "d"]),
    "format_first": lambda m: m.scale_x_discrete(lablim=3, format="{} !"),
    "fraction_truncates": lambda m: m.scale_x_discrete(lablim=2.7),
    "zero_is_no_limit": lambda m: m.scale_x_discrete(lablim=0),
}


@pytest.mark.parametrize("case", LABLIM_CASES)
def test_lablim_cuts_discrete_axis_labels_like_lets_plot(case: str) -> None:
    def labels(module: object) -> list[str]:
        plot = module.ggplot(LABLIM_FRAME, module.aes("x", "y")) + module.geom_point()
        plot += LABLIM_CASES[case](module)
        reader = lets_plot_labels if module is lp else ggrust_labels
        return [label.replace(" ", "") for label in reader(plot.to_svg(), "x")]

    assert labels(gg) == labels(lp)


def test_lablim_on_a_flipped_y_axis() -> None:
    frame = {"x": ["a", "b"], "y": ["long name here", "other"]}
    plot = gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_point() + gg.scale_y_discrete(lablim=4)
    assert ggrust_labels(plot.to_svg(), "y") == ["long...", "othe..."]


def test_lablim_is_ignored_on_continuous_axes_like_lets_plot() -> None:
    assert x_labels(gg, 1000, 20000, lablim=2) == x_labels(lp, 1000, 20000, lablim=2)


@pytest.mark.parametrize("key", ["break_width", "na_value"])
@pytest.mark.parametrize("value", [float("nan"), float("inf")])
def test_non_finite_numbers_fail_at_spec_time(key: str, value: float) -> None:
    with pytest.raises(SpecError, match=rf"{key}.*finite number"):
        gg.scale_x_continuous(**{key: value})
