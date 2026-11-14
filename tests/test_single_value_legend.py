"""A continuous legend over a single value shows one key at that value (#1267).

lets-plot widens the zero-width range and lists about 5 near-identical breaks
(4.6 ... 5.4, or 9.9999999996e9 ... 1.00000000004e10); ggplot2 shows one key, and
ggrust follows ggplot2 (deviations.md). Given breaks, limits and real ranges are
unchanged.
"""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402

TEXT = re.compile(r">([^<>]+)</t(?:ext|span)>")


def legend_texts(plot: gg.PlotSpec) -> list[str]:
    texts = TEXT.findall(plot.to_svg())
    return texts[texts.index("s") + 1 :]


def single(value: float, aesthetic: str) -> gg.PlotSpec:
    data = {"x": [1, 2], "y": [1, 2], "s": [value, value]}
    return gg.ggplot(data, gg.aes("x", "y", **{aesthetic: "s"})) + gg.geom_point()


@pytest.mark.parametrize("aesthetic", ["size", "color", "alpha"])
@pytest.mark.parametrize(("value", "label"), [(5.0, ["5"]), (-3.0, ["-3"]), (1e10, ["10"])])
def test_one_key(aesthetic: str, value: float, label: list[str]) -> None:
    assert legend_texts(single(value, aesthetic)) == label


def test_transformed_scale() -> None:
    plot = single(5.0, "color") + gg.scale_color_continuous(trans="log10")
    assert legend_texts(plot) == ["5"]


def test_given_breaks_and_limits_are_kept() -> None:
    assert legend_texts(single(5.0, "size") + gg.scale_size(breaks=[4, 5, 6])) == ["4", "5", "6"]
    assert legend_texts(single(5.0, "size") + gg.scale_size(limits=[0, 10])) == [
        "0",
        "2",
        "4",
        "6",
        "8",
        "10",
    ]


def test_real_range_is_unchanged() -> None:
    data = {"x": [1, 2], "y": [1, 2], "s": [1.0, 2.0]}
    plot = gg.ggplot(data, gg.aes("x", "y", size="s")) + gg.geom_point()
    assert len(legend_texts(plot)) == 6
