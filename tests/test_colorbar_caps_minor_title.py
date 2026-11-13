"""Colorbar triangle caps, minor ticks (ggrust options) and ``legend_title_position``."""

import re
import warnings

import ggrust as gg
import numpy as np
import pytest
from ggrust.errors import SpecError
from ggrust.plot.core import FeatureSpec, PlotSpec

FRAME = {"x": np.arange(6) % 3, "y": np.arange(6) // 3, "v": np.geomspace(5.0, 500.0, 6)}
TRIANGLE = re.compile(r'<path d="M[^"]*L[^"]*L[^"]*Z" fill="([^"]+)"')
LINE = re.compile(r'<path d="M([\d.]+) ([\d.]+)L([\d.]+) ([\d.]+)"')


def _plot(*, log: bool = True, position: str = "right", **guide: object) -> PlotSpec:
    scale = (
        gg.scale_fill_gradient(name="Life", trans="log10", breaks=[10, 50, 100])
        if log
        else gg.scale_fill_gradient(name="Life", breaks=[100, 200, 300, 400])
    )
    return (
        gg.ggplot(FRAME)
        + gg.geom_tile(gg.aes("x", "y", fill="v"))
        + scale
        + gg.theme(legend_position=position)
        + gg.guides(fill=gg.guide_colorbar(**guide))
    )


def _svg(plot: PlotSpec) -> str:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        return plot.to_svg()


def _ticks(svg: str) -> list[tuple[float, float, float, float]]:
    return [tuple(float(value) for value in match) for match in LINE.findall(svg)]


def _text(svg: str, word: str) -> dict[str, str]:
    match = re.search(r"<text([^>]*)>" + word + "<", svg)
    assert match, word
    return dict(re.findall(r'([\w-]+)="([^"]*)"', match.group(1)))


def test_default_colorbar_has_no_caps() -> None:
    assert not TRIANGLE.findall(_svg(_plot()))


@pytest.mark.parametrize("position", ["right", "bottom"])
def test_triangle_caps_take_the_end_colors(position: str) -> None:
    svg = _svg(_plot(position=position, shape="triangle"))
    fills = TRIANGLE.findall(svg)
    assert len(fills) == 2
    assert len(set(fills)) == 2


def test_show_picks_the_lower_or_upper_cap() -> None:
    both = TRIANGLE.findall(_svg(_plot(shape="triangle")))
    upper = TRIANGLE.findall(_svg(_plot(shape="triangle", show=(False, True))))
    lower = TRIANGLE.findall(_svg(_plot(shape="triangle", show=[True, False])))
    assert len(upper) == len(lower) == 1
    assert upper[0] != lower[0]
    assert set(both) == {upper[0], lower[0]}
    assert not TRIANGLE.findall(_svg(_plot(shape="triangle", show=False)))


def test_log10_minor_ticks_fill_the_decades() -> None:
    plain = _ticks(_svg(_plot()))
    minor = _ticks(_svg(_plot(minor_ticks=True)))
    # 5..500 on log10 with breaks 10, 50, 100: minor at 5-9, 20-40, 60-90, 200-500
    # (16 values), each a mark on both edges of the bar.
    assert len(minor) - len(plain) == 2 * 16


def test_linear_minor_ticks_sit_between_breaks() -> None:
    plain = _ticks(_svg(_plot(log=False, position="bottom")))
    minor = _ticks(_svg(_plot(log=False, position="bottom", minor_ticks=True)))
    # Breaks 100..400 over 5..500: minor at 50, 150, 250, 350, 450.
    assert len(minor) - len(plain) == 2 * 5
    # Tick marks only (grid lines and the axis are longer).
    lengths = sorted({round(abs(y2 - y1), 3) for _, y1, _, y2 in minor if 0 < abs(y2 - y1) < 10})
    assert len(lengths) == 2 and lengths[0] == pytest.approx(lengths[1] / 2)


def test_title_below_a_horizontal_bar_centred() -> None:
    plot = _plot(position="bottom") + gg.theme(
        legend_title_position="bottom", legend_title=gg.element_text(hjust=0.5)
    )
    svg = _svg(plot)
    title, label = _text(svg, "Life"), _text(svg, "50")
    assert float(title["y"]) > float(label["y"])
    # Centred on the bar: the 50 break sits mid-bar on a log scale from 5 to 500.
    width = 4 * 6.5  # "Life" is about 26 px wide at the default size
    assert abs(float(title["x"]) + width / 2 - float(label["x"])) < 10


def test_guide_theme_sets_the_title_position() -> None:
    plot = _plot(theme=gg.theme(legend_title_position="right"))
    svg = _svg(plot)
    title, label = _text(svg, "Life"), _text(svg, "100")
    assert float(title["x"]) > float(label["x"])


def test_title_position_is_checked_by_the_engine() -> None:
    plot = _plot() + FeatureSpec("theme", name=None, legend_title_position="botom")
    with pytest.raises(ValueError, match=r"legend_title_position.*did you mean 'bottom'"):
        plot.to_svg()


@pytest.mark.parametrize(
    ("options", "message"),
    [
        ({"shape": "arrow"}, r"shape='arrow'.*expected 'triangle'"),
        ({"show": True}, r"show=True.*add shape='triangle'"),
        ({"shape": "triangle", "show": (True,)}, r"show=\(True,\).*\(lower, upper\)"),
        ({"shape": "triangle", "show": "both"}, r"show='both'"),
        ({"minor_ticks": 1}, r"minor_ticks=1.*True or False"),
    ],
)
def test_bad_options_fail_early(options: dict[str, object], message: str) -> None:
    with pytest.raises(SpecError, match=message):
        gg.guide_colorbar(**options)  # ty: ignore[invalid-argument-type]


def test_engine_rejects_hand_written_caps() -> None:
    from ggrust.plot.guide import _guide

    plot = _plot() + gg.guides(fill=_guide("colorbar", shape="triangel"))
    with pytest.raises(ValueError, match=r"shape.*did you mean 'triangle'"):
        plot.to_svg()
    plot = _plot() + gg.guides(fill=_guide("colorbar", shape="triangle", show=[1, 0]))
    with pytest.raises(ValueError, match=r"show.*\[lower, upper\]"):
        plot.to_svg()


def _bar_paths(svg: str) -> list[str]:
    return re.findall(r'<path d="(M[^"]*Z)" fill="none"', svg)


def test_legend_frame_outlines_bar_and_caps() -> None:
    assert not _bar_paths(_svg(_plot(shape="triangle")))
    framed = _plot(shape="triangle") + gg.theme(legend_frame=gg.element_rect(color="black"))
    outlines = _bar_paths(_svg(framed))
    assert len(outlines) == 1
    # A vertical bar with both caps: 4 corners plus 2 tips.
    assert outlines[0].count("L") == 5


def test_negative_tick_length_points_ticks_out_of_the_bar() -> None:
    # Axis ticks off: only the colorbar draws short marks.
    ticks = gg.theme(legend_ticks=gg.element_line(color="black"), axis_ticks=gg.element_blank())
    inside = _ticks(_svg(_plot(position="bottom") + ticks))
    outside = _ticks(_svg(_plot(position="bottom") + ticks + gg.theme(legend_ticks_length=[-5, 0])))
    short = [tick for tick in outside if 0 < abs(tick[3] - tick[1]) < 10]
    # Bottom edge only, pointing down (y grows), 5 px long.
    assert short and all(y2 - y1 == pytest.approx(5) for _, y1, _, y2 in short)
    assert len(short) * 2 == len([t for t in inside if 0 < abs(t[3] - t[1]) < 10])


def test_cap_size_sets_the_cap_length() -> None:
    svg = _svg(_plot(position="bottom", shape="triangle", cap_size=30))
    match = re.search(r'<path d="M([\d.]+) [\d.]+L([\d.]+) [\d.]+L[^"]*Z" fill="#', svg)
    assert match
    base, tip = float(match.group(1)), float(match.group(2))
    assert abs(base - tip) == pytest.approx(31)  # 30 plus the unit tucked under the bar


@pytest.mark.parametrize(
    ("options", "message"),
    [
        ({"cap_size": 5}, r"cap_size=5.*add shape='triangle'"),
        ({"shape": "triangle", "cap_size": -1}, r"cap_size=-1.*positive"),
        ({"shape": "triangle", "cap_size": True}, r"cap_size=True.*positive"),
    ],
)
def test_bad_cap_size_fails_early(options: dict[str, object], message: str) -> None:
    with pytest.raises(SpecError, match=message):
        gg.guide_colorbar(**options)  # ty: ignore[invalid-argument-type]


def test_tick_length_takes_one_or_two_numbers() -> None:
    plot = _plot() + gg.theme(legend_ticks_length=[1, 2, 3])
    with pytest.raises(ValueError, match=r"legend_ticks_length.*two numbers"):
        plot.to_svg()


def _themed(position: str) -> PlotSpec:
    return (
        gg.ggplot({"x": [1, 2, 3], "y": [1, 2, 3], "v": [1.0, 2.0, 3.0]})
        + gg.geom_point(gg.aes("x", "y", color="v"))
        + gg.labs(color="Life")
        + gg.guides(color=gg.guide_colorbar(theme=gg.theme(legend_title_position=position)))
    )


@pytest.mark.parametrize(("second", "count"), [("bottom", 1), ("top", 2)])
def test_collect_merges_colorbars_with_equal_guide_themes(second: str, count: int) -> None:
    grid = gg.gggrid([_themed("bottom"), _themed(second)], ncol=1, guides="collect")
    assert _svg(grid).count(">Life<") == count
