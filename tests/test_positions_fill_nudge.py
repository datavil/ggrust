"""``position_fill`` and ``position_nudge`` match lets_plot."""

import re
import warnings
from typing import Any

import polars as pl
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

FRAME = pl.DataFrame(
    {
        "g": ["a", "a", "b", "b", "c", "c", "a"],
        "k": ["u", "v", "u", "v", "u", "v", "u"],
        "v": [1.0, 3, 2, 2, -1, 4, 1],
    }
)


def axis_texts(module: Any, layer: Any) -> list[str]:
    svg = (module.ggplot(FRAME) + layer).to_svg()
    return sorted(text.strip() for text in re.findall(r">([^<>]{1,8})<", svg) if text.strip())


@pytest.mark.parametrize(
    "build",
    [
        lambda m: m.geom_bar(m.aes("g", fill="k"), position="fill"),
        lambda m: m.geom_bar(
            m.aes("g", "v", fill="k"), stat="identity", position=m.position_fill(vjust=0.5)
        ),
        lambda m: m.geom_point(m.aes("g", "v", color="k"), position=m.position_fill()),
    ],
    ids=["count", "identity-negative", "points"],
)
def test_position_fill_axes_match_lets_plot(build: Any) -> None:
    assert axis_texts(gg, build(gg)) == axis_texts(lp, build(lp))


def test_position_nudge_moves_text_in_data_units() -> None:
    frame = pl.DataFrame({"x": [1.0], "y": [2.0], "t": ["a"]})
    plain = gg.ggplot(frame) + gg.geom_text(gg.aes("x", "y", label="t"))
    nudged = gg.ggplot(frame) + gg.geom_text(
        gg.aes("x", "y", label="t"), position=gg.position_nudge(x=1, y=-1)
    )
    assert plain.to_svg() != nudged.to_svg()
    assert "2" in axis_texts(gg, gg.geom_point(gg.aes("g", "v"), position=gg.position_nudge(y=1)))


def test_position_nudge_unit_px_is_an_error() -> None:
    layer = gg.geom_point(gg.aes("g", "v"), position=gg.position_nudge(x=1, unit="px"))
    with pytest.raises(SpecError, match="unsupported nudge unit 'px'"):
        (gg.ggplot(FRAME) + layer).to_svg()


def point_xs(module: Any, nudge: float) -> list[float]:
    frame = {"x": ["a", "b", "c"], "y": [1.0, 2, 3]}
    plot = module.ggplot(frame, module.aes("x", "y")) + module.geom_point(
        position=module.position_nudge(x=nudge)
    )
    return [float(x) for x in re.findall(r'<circle[^>]*cx="([-\d.]+)"', plot.to_svg())]


def test_position_nudge_keeps_a_discrete_axis_like_lets_plot() -> None:
    def shifts(module: Any) -> list[float]:
        before, after = point_xs(module, 0.0), point_xs(module, 0.3)
        spacing = before[1] - before[0]
        return [
            round((moved - start) / spacing, 3) for start, moved in zip(before, after, strict=True)
        ]

    assert shifts(gg) == shifts(lp) == [0.3, 0.3, 0.3]
