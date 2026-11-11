"""stat_summary and stat_summary_bin against lets-plot.

With ``theme_void`` and fixed scale limits both libraries lay out the same panel,
shifted by a constant translate, so point and range coordinates are compared
relative to the first point.
"""

import re
import warnings
from collections.abc import Callable
from types import ModuleType

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

DATA = {
    "x": [1, 1, 1, 2, 2, 2, 2, 3, 3.4, 3.6],
    "y": [1.0, 2, 6, 3, 4, 8, 9, 5, 2, 7],
    "g": ["a", "b", "a", "b", "a", "b", "a", "b", "a", "b"],
}


def _numbers(tag: str, names: tuple[str, ...]) -> list[float]:
    attributes = dict(re.findall(r'([\w-]+)="([^"]*)"', tag))
    return [float(attributes[name]) for name in names]


def _marks(svg: str) -> list[tuple[float, ...]]:
    """Circle centres and vertical range ends, relative to the first circle."""
    marks: list[tuple[float, ...]] = []
    for tag in re.findall(r"<circle\b[^>]*>", svg):
        marks.append(tuple(_numbers(tag, ("cx", "cy"))))
    for tag in re.findall(r"<line\b[^>]*>", svg):
        marks.append(tuple(_numbers(tag, ("x1", "y1", "x2", "y2"))))
    for tag in re.findall(r"<path\b[^>]*>", svg):
        if "pointer-events" in tag or "fill-rule" in tag:
            continue
        d = dict(re.findall(r'([\w-]+)="([^"]*)"', tag)).get("d", "")
        numbers = [float(n) for n in re.findall(r"-?\d+(?:\.\d+)?", d)]
        if len(numbers) == 4:
            marks.append(tuple(numbers))
    origin_x, origin_y = marks[0][:2]
    relative = [
        tuple(
            value - (origin_x if index % 2 == 0 else origin_y) for index, value in enumerate(mark)
        )
        for mark in marks
    ]
    return sorted(relative)


def _svg(module: ModuleType, layer: Callable[[ModuleType], object]) -> str:
    plot = (
        module.ggplot(DATA, module.aes("x", "y"))
        + layer(module)
        + module.theme_void()
        + module.scale_y_continuous(limits=[0, 10])
        + module.scale_x_continuous(limits=[0, 4])
        + module.ggsize(400, 300)
    )
    return plot.to_svg()


CASES: dict[str, Callable[[ModuleType], object]] = {
    "default": lambda m: m.stat_summary(),
    "median_quartiles": lambda m: m.stat_summary(fun="median", fun_min="lq", fun_max="uq"),
    "quantiles": lambda m: m.stat_summary(
        fun="mq", fun_min="lq", fun_max="uq", quantiles=[0.9, 0.1, 0.5]
    ),
    "count_sum": lambda m: m.stat_summary(fun="count", fun_min="min", fun_max="sum"),
    "point_geom": lambda m: m.stat_summary(geom="point", fun="max"),
    "groups": lambda m: m.stat_summary(m.aes(group="g")),
    "bin": lambda m: m.stat_summary_bin(bins=4),
    "bin_single_x": lambda m: m.stat_summary_bin(m.aes(x=[2.0] * 10)),
    "bin_width": lambda m: m.stat_summary_bin(binwidth=1, boundary=0, fun="median"),
}


@pytest.mark.parametrize("name", list(CASES))
def test_matches_lets_plot(name: str) -> None:
    ours = _marks(_svg(gg, CASES[name]))
    theirs = _marks(_svg(lp, CASES[name]))
    assert len(ours) == len(theirs)
    for mine, other in zip(ours, theirs, strict=True):
        assert mine == pytest.approx(other, abs=0.15)


@pytest.mark.parametrize(
    ("fun_data", "fun_args", "low", "high"),
    [
        # R: x <- c(1, 2, 4, 9); mean_se(x); mean_sdl(x); mean_sdl(x, mult = 1)
        ("mean_se", None, 2.2204869579947815, 5.7795130420052185),
        ("mean_sdl", None, -3.118052168020874, 11.118052168020874),
        ("mean_sdl", {"mult": 1}, 0.4409739159895631, 7.559026084010437),
    ],
)
def test_fun_data_as_ggplot2(
    fun_data: str, fun_args: dict[str, float] | None, low: float, high: float
) -> None:
    data = {"x": [0, 0, 0, 0], "y": [1.0, 2.0, 4.0, 9.0]}
    plot = (
        gg.ggplot(data, gg.aes("x", "y"))
        + gg.stat_summary(fun_data=fun_data, fun_args=fun_args)
        # A reference point at y = 14 gives the pixel scale.
        + gg.geom_point(x=0, y=14)
    )
    svg = plot.to_svg()
    mean_y, reference_y = sorted(
        (_numbers(tag, ("cx", "cy"))[1] for tag in re.findall(r"<circle\b[^>]*>", svg)),
        reverse=True,
    )
    (segment,) = [
        [float(n) for n in re.findall(r"-?\d+(?:\.\d+)?", d)]
        for d in re.findall(
            r'<path d="([^"]+)" fill="none" stroke="#474747" stroke-width="1.65"', svg
        )
    ]
    per_unit = (mean_y - reference_y) / 10

    def to_data(pixel: float) -> float:
        return 4.0 + (mean_y - pixel) / per_unit

    assert sorted(to_data(pixel) for pixel in (segment[1], segment[3])) == pytest.approx(
        [low, high], abs=0.02
    )


@pytest.mark.parametrize(
    ("kwargs", "match"),
    [
        ({"fun": "meen"}, r"layers\[0\]\.fun.*unknown summary function 'meen'.*'mean'"),
        ({"fun": "mean_se"}, r"pass fun_data='mean_se'"),
        ({"fun_data": "mean_sd"}, r"layers\[0\]\.fun_data.*'mean_sd'.*mean_se, mean_sdl"),
        ({"fun_data": "mean_cl_normal", "fun_args": {"mult": 2}}, r"it takes conf_int"),
        ({"fun_data": "median_hilow", "fun_args": {"conf_int": 1.5}}, r"between 0 and 1"),
        ({"fun_data": "mean_cl_boot", "fun_args": {"B": 0.5}}, r"'B' expected a whole number"),
        ({"fun_data": "mean_se", "fun": "median"}, r"layers\[0\]\.fun.*cannot be combined"),
        ({"fun_data": "mean_se", "fun_args": {"mul": 2}}, r"unknown argument 'mul'"),
        ({"fun_args": {"mult": 2}}, r"fun_args applies to fun_data"),
        ({"quantiles": [0.1, 0.5]}, r"layers\[0\]\.quantiles.*three numbers"),
    ],
)
def test_errors(kwargs: dict[str, object], match: str) -> None:
    plot = gg.ggplot(DATA, gg.aes("x", "y")) + gg.stat_summary(**kwargs)
    with pytest.raises(SpecError, match=match):
        plot.to_svg()


def test_spec_matches_lets_plot() -> None:
    for build in CASES.values():
        ours = build(gg).as_dict()
        theirs = build(lp).as_dict()
        assert {key: ours[key] for key in ("geom", "stat")} == {
            key: theirs[key] for key in ("geom", "stat")
        }


@pytest.mark.parametrize("oriented", [False, True])
def test_unused_summary_bounds_do_not_train_the_axis(oriented: bool) -> None:
    """Points of `fun="mean"` train the axis on the means only, as lets-plot (bug hunt)."""
    data = {"g": list("aaabbbbbccc"), "y": [1.0, 2, 3, 2, 3, 4, 5, 9, 1, 2, 3]}

    def ticks(module: ModuleType) -> list[str]:
        mapping = module.aes("y", "g") if oriented else module.aes("g", "y")
        plot = module.ggplot(data, mapping) + module.stat_summary(fun="mean", geom="point")
        return sorted(re.findall(r">(-?[\d.]+)</t", plot.to_svg()))

    assert ticks(gg) == ticks(lp)


@pytest.mark.parametrize(
    ("fun_data", "fun_args", "centre", "low", "high"),
    [
        # R: x <- c(1, 2, 4, 9); Hmisc::smean.cl.normal(x); quantile(x, c(.5, .025, .975))
        ("mean_cl_normal", None, 4.0, -1.6632, 9.6632),
        ("mean_cl_normal", {"conf_int": 0.5}, 4.0, 2.6389, 5.3611),
        ("median_hilow", None, 3.0, 1.075, 8.625),
        ("median_hilow", {"conf_int": 0.5}, 3.0, 1.75, 5.25),
    ],
)
def test_confidence_shortcuts_match_hmisc(
    fun_data: str, fun_args: dict[str, float] | None, centre: float, low: float, high: float
) -> None:
    data = {"x": [0, 0, 0, 0], "y": [1.0, 2.0, 4.0, 9.0]}
    plot = gg.ggplot(data, gg.aes("x", "y")) + gg.stat_summary(fun_data=fun_data, fun_args=fun_args)
    row = plot.layer_data(0).row(0, named=True)
    assert [row["y"], row["ymin"], row["ymax"]] == pytest.approx([centre, low, high], abs=1e-3)


def test_mean_cl_boot_is_reproducible_and_seedable() -> None:
    data = {"x": [0] * 30, "y": [float(i) for i in range(30)]}

    def interval(fun_args: dict[str, float] | None) -> tuple[float, float]:
        plot = gg.ggplot(data, gg.aes("x", "y"))
        plot += gg.stat_summary(fun_data="mean_cl_boot", fun_args=fun_args)
        row = plot.layer_data(0).row(0, named=True)
        return row["ymin"], row["ymax"]

    assert interval(None) == interval(None)
    assert interval({"seed": 7}) != interval({"seed": 8})
    low, high = interval({"B": 2000})
    assert low < 14.5 < high
    assert high - low == pytest.approx(2 * 1.96 * 8.8 / 30**0.5, rel=0.15)
    narrow = interval({"conf_int": 0.5})
    assert narrow[0] > low
    assert narrow[1] < high
