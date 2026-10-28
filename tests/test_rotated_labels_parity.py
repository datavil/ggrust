"""Axis labels with a theme angle/hjust/vjust must match lets-plot: same breaks,
rotation, text anchors, and a panel within 10 px (the label band is measured
with real font metrics in ggrust, estimated in lets-plot).
"""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from test_bar_parity import _attributes, _texts  # noqa: E402

GENES = ["CD3E", "MS4A1", "LYZ", "NKG7", "GNLY", "FCGR3A", "CST3", "PPBP", "HLA-DRA"]
CELLS = ["T cells", "B cells", "Monocytes"]
HEAT = {
    "gene": [gene for gene in GENES for _ in CELLS],
    "cell": CELLS * len(GENES),
    "v": [(index * 7 % 11) / 3 for index in range(len(GENES) * len(CELLS))],
}
WIDE = {"x": [1000.0, 25000, 50000, 75000, 100000], "y": [1.0, 2, 3, 4, 5]}


def heat(m, **theme):
    return m.ggplot(HEAT, m.aes("gene", "cell", fill="v")) + m.geom_tile() + m.theme(**theme)


CASES = {
    "x45": lambda m: heat(m, axis_text_x=m.element_text(angle=45)),
    "x90": lambda m: heat(m, axis_text_x=m.element_text(angle=90)),
    "x90_hjust1": lambda m: heat(m, axis_text_x=m.element_text(angle=90, hjust=1)),
    "x90_vjust": lambda m: heat(m, axis_text_x=m.element_text(angle=90, hjust=1, vjust=0.5)),
    "x_minus30_y30": lambda m: heat(
        m, axis_text_x=m.element_text(angle=-30), axis_text_y=m.element_text(angle=30)
    ),
    "y90": lambda m: heat(m, axis_text_y=m.element_text(angle=90)),
    "y_hjust0": lambda m: heat(m, axis_text_y=m.element_text(hjust=0)),
    # Unrotated labels need room: lets-plot drops overlapping ones, ggrust keeps them.
    "x_hjust1": lambda m: heat(m, axis_text_x=m.element_text(hjust=1)) + m.ggsize(1000, 400),
    "x180": lambda m: heat(m, axis_text_x=m.element_text(angle=180)) + m.ggsize(1000, 400),
    "continuous45": lambda m: (
        m.ggplot(WIDE, m.aes("x", "y"))
        + m.geom_point()
        + m.theme(axis_text=m.element_text(angle=45))
    ),
    "continuous_y90": lambda m: (
        m.ggplot(WIDE, m.aes("x", "y"))
        + m.geom_point()
        + m.theme(axis_text_y=m.element_text(angle=90))
    ),
}


def _label_angles_and_anchors(svg: str, ours: bool) -> list[tuple]:
    """(angle, text-anchor) of each axis tick label, in document order."""
    found = []
    if ours:
        for axis in ("axis-x", "axis-y"):
            group = svg[svg.index(f'<g class="{axis}"') :]
            group = group[: group.index("</g>")]
            for tag in re.findall(r"<text [^>]*>", group):
                a = _attributes(tag)
                angle = re.search(r"rotate\((-?[\d.]+)", a.get("transform", "rotate(0"))
                found.append((float(angle.group(1)), a.get("text-anchor", "start")))
    else:
        for axis in ("axis-text-x", "axis-text-y"):
            for match in re.finditer(rf'<text [^>]*class="{axis}"[^>]*>', svg):
                before = svg[: match.start()]
                transform = before[before.rindex("<g ") :]
                angle = re.search(r"rotate\((-?[\d.]+)", transform)
                a = _attributes(match.group(0))
                found.append(
                    (float(angle.group(1)) if angle else 0.0, a.get("text-anchor", "start"))
                )
    return [(round(angle) % 360, anchor) for angle, anchor in found]


def _panel(svg: str, ours: bool) -> tuple[float, float]:
    if ours:
        clip = _attributes(re.search(r"<clipPath[^>]*>\s*<rect([^>]*)/>", svg).group(1))
    else:
        clip = _attributes(
            re.search(r'<rect x="[\d.]+" y="0\.0" width="[\d.]+" height="[\d.]+">', svg).group(0)
        )
    return float(clip["width"]), float(clip["height"])


@pytest.mark.parametrize("name", sorted(CASES))
def test_rotated_labels_match_lets_plot(name):
    expected_svg = (CASES[name](lp) + lp.theme(legend_position="none")).to_svg()
    actual_svg = (CASES[name](gg) + gg.theme(legend_position="none")).to_svg()
    assert sorted(_texts(actual_svg)) == sorted(_texts(expected_svg))
    assert _label_angles_and_anchors(actual_svg, True) == _label_angles_and_anchors(
        expected_svg, False
    )
    (actual_width, actual_height), (width, height) = (
        _panel(actual_svg, True),
        _panel(expected_svg, False),
    )
    # Bands from real font metrics vs lets-plot's estimates: 90 degree x labels are
    # up to ~8 px shorter, 90 degree y labels sit ~6 px further from the title.
    assert actual_width == pytest.approx(width, abs=10)
    assert actual_height == pytest.approx(height, abs=10)
