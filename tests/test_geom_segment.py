"""geom_segment end padding: `size_start`/`size_end`, `stroke_start`/`stroke_end`
and `spacer` stop the line short of its end points by the same pixels as lets-plot.

Panels of the two libraries sit a few pixels apart, so each library's padded ends are
compared with its own unpadded ends: the cut lengths must agree.
"""

import math
import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

Point = tuple[float, float]
Ends = tuple[Point, Point]
NUMBER = r"-?\d+(?:\.\d+)?(?:[eE]-?\d+)?"


def _ggrust_ends(svg: str) -> list[Ends]:
    """Ends of every ggrust segment (a two-point `M x y L x y` path, size 0.75)."""
    ends = []
    for d in re.findall(r'<path\b[^>]*\bd="([^"]*)"[^>]*stroke-width="1\.65', svg):
        values = [float(value) for value in re.findall(NUMBER, d)]
        ends.append(((values[0], values[1]), (values[-2], values[-1])))
    return ends


def _letsplot_ends(svg: str) -> list[Ends]:
    """Ends of every lets-plot segment (a `<line>` with the 1.65 px stroke)."""
    ends = []
    for tag in re.findall(r"<line\b[^>]*>", svg):
        if 'stroke-width="1.65' not in tag:
            continue

        def attribute(name: str, tag: str = tag) -> float:
            match = re.search(rf'\b{name}="({NUMBER})"', tag)
            assert match is not None
            return float(match.group(1))

        ends.append(((attribute("x1"), attribute("y1")), (attribute("x2"), attribute("y2"))))
    return ends


def _plot(module, data: dict | None = None, **options: object):
    data = data or {"x": [0.0, 0.0], "y": [0.0, 2.0], "xend": [10.0, 6.0], "yend": [0.0, 5.0]}
    return (
        module.ggplot(data)
        + module.geom_segment(module.aes("x", "y", xend="xend", yend="yend"), **options)
        + module.scale_x_continuous(limits=[-1, 11])
        + module.scale_y_continuous(limits=[-1, 6])
        + module.theme_void()
    )


def _cuts(module, **options: object) -> list[tuple[float, float]]:
    """Pixels cut off the start and the end of each segment."""
    ends = _letsplot_ends if module is lp else _ggrust_ends
    base = ends(_plot(module).to_svg())
    padded = ends(_plot(module, **options).to_svg())
    assert len(base) == len(padded) == 2
    return [
        (round(math.dist(b[0], p[0]), 2), round(math.dist(b[1], p[1]), 2))
        for b, p in zip(base, padded, strict=True)
    ]


@pytest.mark.parametrize(
    "options",
    [
        {"size_start": 3},
        {"size_end": 3},
        {"stroke_start": 2},
        {"stroke_end": 0.5},
        {"spacer": 5},
        {"size_start": 3, "stroke_start": 2, "size_end": 5, "stroke_end": 1, "spacer": 2},
        # Longer than the rest of the line after the start cut: the end stays.
        {"size_start": 150, "size_end": 150},
        # Longer than the whole line: the line stays whole.
        {"size_start": 1000},
    ],
)
def test_padding_matches_lets_plot(options: dict) -> None:
    # ggrust writes coordinates with 2 decimals.
    ours, theirs = _cuts(gg, **options), _cuts(lp, **options)
    assert [cut for pair in ours for cut in pair] == pytest.approx(
        [cut for pair in theirs for cut in pair], abs=0.02
    )


def test_padding_formula() -> None:
    """size x 2.2 / 2 + stroke x 2.2 + spacer pixels: the outline of a point of that
    size and stroke."""
    cuts = _cuts(gg, size_start=4, stroke_start=1, size_end=2, spacer=1.5)
    assert cuts[0] == pytest.approx((4 * 1.1 + 2.2 + 1.5, 2 * 1.1 + 1.5))


def test_padding_with_arrow_keeps_the_head_at_the_cut_end() -> None:
    svg = _plot(gg, size_end=4, arrow=gg.arrow()).to_svg()
    unpadded = _plot(gg, arrow=gg.arrow()).to_svg()
    tips = [
        [float(value) for value in re.findall(NUMBER, d)][2:4]
        for d in re.findall(r'<path\b[^>]*\bd="([^"]*)"', svg)
        if d.count("L") == 2
    ]
    unpadded_tips = [
        [float(value) for value in re.findall(NUMBER, d)][2:4]
        for d in re.findall(r'<path\b[^>]*\bd="([^"]*)"', unpadded)
        if d.count("L") == 2
    ]
    assert len(tips) == len(unpadded_tips) == 2
    for tip, unpadded_tip in zip(tips, unpadded_tips, strict=True):
        assert math.dist(tip, unpadded_tip) == pytest.approx(4.4, abs=0.02)


def test_negative_sizes_do_not_extend_the_line() -> None:
    assert _cuts(gg, size_start=-3, stroke_end=-1) == [(0.0, 0.0), (0.0, 0.0)]


def test_negative_spacer_names_the_parameter() -> None:
    with pytest.raises(SpecError, match=r"spacer.*non-negative"):
        _plot(gg, spacer=-1).to_svg()


def test_mapped_padding_warns_and_uses_the_constant() -> None:
    data = {"x": [0.0], "y": [0.0], "xend": [10.0], "yend": [0.0], "s": [3.0]}
    plot = (
        gg.ggplot(data)
        + gg.geom_segment(gg.aes("x", "y", xend="xend", yend="yend", size_start="s"))
        + gg.theme_void()
    )
    with pytest.warns(UserWarning, match=r"size_start.*not supported yet"):
        plot.to_svg()
