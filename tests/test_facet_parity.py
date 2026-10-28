"""facet_wrap must match lets-plot: panels, strip labels, data in each panel.

Data marks (points, rectangles) are compared as fractions of their panel, panel by
panel in row-major order. Panel sizes may differ by a few pixels because ggrust
measures text with the embedded font (axis label widths), so they are compared
with a small relative tolerance.
"""

import re
import warnings

import numpy as np
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from test_bar_parity import _attributes, _hex  # noqa: E402

RANDOM = np.random.default_rng(7)
N = 120
POINTS = {
    "x": RANDOM.normal(size=N).round(3),
    "y": (RANDOM.normal(size=N) * np.repeat([1, 10, 100, 1000], N // 4)).round(3),
    "g": np.repeat(["d", "b", "c", "a"], N // 4).tolist(),
    "h": RANDOM.choice(["p", "q"], N).tolist(),
    "cell": RANDOM.choice(["T", "B", "NK"], N).tolist(),
    "k": RANDOM.choice([1.0, 2.5, 10.0], N).tolist(),
}
OTHER = {"x": [0.0, 1.0], "y": [0.0, 5.0]}


def _lets_plot_panels(svg: str) -> list[dict]:
    panels = []
    for match in re.finditer(r'<g clip-path="url\(#(\w+)\)"', svg):
        body = svg[match.end() : svg.index("</defs>", match.end())]
        clip = _attributes(re.search(r"<rect ([^>]*)>\s*</rect>\s*</clipPath>", body).group(1))
        width, height = float(clip["width"]), float(clip["height"])
        marks = body[: body.index("<defs>")]
        panels.append(
            {
                "size": (width, height),
                "points": sorted(
                    (round(float(a["cx"]) / width, 3), round(float(a["cy"]) / height, 3))
                    for a in map(_attributes, re.findall(r"<circle [^>]*>", marks))
                ),
                "rects": sorted(
                    (
                        round(float(a["x"]) / width, 3),
                        round(float(a["y"]) / height, 3),
                        round(float(a["width"]) / width, 3),
                        round(float(a["height"]) / height, 3),
                    )
                    for a in map(_attributes, re.findall(r"<rect [^>]*>", marks))
                ),
            }
        )
    return panels


def _ggrust_panels(svg: str) -> list[dict]:
    panels = []
    for match in re.finditer(r'<clipPath id="[^"]+"><rect ([^>]*)/></clipPath>', svg):
        clip = _attributes(match.group(1))
        left, top = float(clip["x"]), float(clip["y"])
        width, height = float(clip["width"]), float(clip["height"])
        start = svg.index('<g class="panel"', match.end())
        marks = svg[start : svg.index('<g class="', start + 1)]
        panels.append(
            {
                "size": (width, height),
                "points": sorted(
                    (
                        round((float(a["cx"]) - left) / width, 3),
                        round((float(a["cy"]) - top) / height, 3),
                    )
                    for a in map(_attributes, re.findall(r"<circle [^>]*>", marks))
                ),
                "rects": sorted(
                    (
                        round((float(a["x"]) - left) / width, 3),
                        round((float(a["y"]) - top) / height, 3),
                        round(float(a["width"]) / width, 3),
                        round(float(a["height"]) / height, 3),
                    )
                    for a in map(_attributes, re.findall(r"<rect [^>]*>", marks))
                ),
            }
        )
    return panels


def _lets_plot_strips(svg: str) -> list[str]:
    return [
        "".join(re.findall(r"<tspan[^>]*>([^<]*)</tspan>", text))
        for text in re.findall(r'class="facet-strip-text-x"[^>]*>(.*?)</text>', svg, re.S)
    ]


def _ggrust_strips(svg: str) -> list[str]:
    strips = []
    for start in [match.end() for match in re.finditer(r'<g class="strips">', svg)]:
        end = svg.find('<g class="', start)
        block = svg[start : end if end >= 0 else len(svg)]
        for text in re.findall(r"<text [^>]*>(.*?)</text>", block, re.S):
            strips.append(re.sub(r"<[^>]+>", "", text))
    return strips


CASES = {
    "points": lambda m: m.ggplot(POINTS, m.aes("x", "y")) + m.geom_point() + m.facet_wrap("g"),
    "points_ncol": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y", color="h")) + m.geom_point() + m.facet_wrap("g", ncol=3)
    ),
    "free": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y")) + m.geom_point() + m.facet_wrap("g", scales="free")
    ),
    "free_y_vertical": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y"))
        + m.geom_point()
        + m.facet_wrap("g", nrow=3, dir="v", scales="free_y")
    ),
    "descending": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y")) + m.geom_point() + m.facet_wrap("g", order=-1)
    ),
    "data_order": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y")) + m.geom_point() + m.facet_wrap("g", order=0)
    ),
    "two_variables": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y")) + m.geom_point() + m.facet_wrap(["h", "g"], ncol=4)
    ),
    "numbers_formatted": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y")) + m.geom_point() + m.facet_wrap("k", format="{.1f} u")
    ),
    "layer_without_facet_column": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y"))
        + m.geom_point()
        + m.geom_point(data=OTHER, color="red")
        + m.facet_wrap("h")
    ),
    "stacked_bars_free_x": lambda m: (
        m.ggplot(POINTS, m.aes("cell", fill="h"))
        + m.geom_bar()
        + m.facet_wrap("g", scales="free_x")
    ),
    "histogram_shared_bins": lambda m: (
        m.ggplot(POINTS, m.aes("x")) + m.geom_histogram(bins=12) + m.facet_wrap("h")
    ),
    "flipped": lambda m: (
        m.ggplot(POINTS, m.aes("cell", "x")) + m.geom_point() + m.coord_flip() + m.facet_wrap("h")
    ),
    "histogram_in_some_panels": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y"))
        + m.geom_point()
        + m.geom_histogram(
            m.aes(y="..count.."), data={"x": [0.0, 0.5, 1.0], "h": ["p"] * 3}, bins=3
        )
        + m.facet_wrap("h")
    ),
    "color_breaks_lead": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y", color="cell"))
        + m.geom_point()
        + m.scale_color_manual(values=["red", "green", "blue"], breaks=["NK", "B"])
        + m.facet_wrap("g")
    ),
    "dodge_missing_middle_category": lambda m: (
        m.ggplot(
            {
                "c": ["T"] * 4 + ["NK"] * 4 + ["B"],
                "s": list("rrss") * 2 + ["r"],
                "g": ["b"] * 8 + ["a"],
            },
            m.aes("c", fill="s"),
        )
        + m.geom_bar(position="dodge")
        + m.facet_wrap("g")
    ),
    "theme_bw": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y")) + m.geom_point() + m.facet_wrap("h") + m.theme_bw()
    ),
}


def _flat(marks: list[tuple]) -> list[float]:
    return [value for mark in marks for value in mark]


def _matched(got: list[tuple], want: list[tuple]) -> list[float]:
    """`got` reordered to follow `want`, each mark paired with its nearest one."""
    left = list(got)
    ordered = []
    for mark in want:
        nearest = min(
            left, key=lambda other: sum(abs(a - b) for a, b in zip(other, mark, strict=True))
        )
        left.remove(nearest)
        ordered.append(nearest)
    return _flat(ordered)


def _both(build):
    return [(build(module) + module.theme(legend_position="none")).to_svg() for module in (lp, gg)]


@pytest.mark.parametrize("name", sorted(CASES))
def test_facets_match_lets_plot(name):
    expected_svg, actual_svg = _both(CASES[name])
    assert _ggrust_strips(actual_svg) == _lets_plot_strips(expected_svg)
    expected, actual = _lets_plot_panels(expected_svg), _ggrust_panels(actual_svg)
    assert len(actual) == len(expected)
    for index, (got, want) in enumerate(zip(actual, expected, strict=True)):
        assert got["size"] == pytest.approx(want["size"], rel=0.03), index
        for kind in ("points", "rects"):
            assert len(got[kind]) == len(want[kind]), (index, kind)
            assert _matched(got[kind], want[kind]) == pytest.approx(_flat(want[kind]), abs=2e-3)


def test_default_size_grows_with_the_grid():
    svg = (gg.ggplot(POINTS, gg.aes("x", "y")) + gg.geom_point() + gg.facet_wrap("g")).to_svg()
    expected = lp.ggplot(POINTS, lp.aes("x", "y")) + lp.geom_point() + lp.facet_wrap("g")
    size = re.search(r'width="([\d.]+)" height="([\d.]+)"', expected.to_svg()).groups()
    assert re.search(r'width="([\d.]+)" height="([\d.]+)"', svg).groups() == tuple(
        str(int(float(value))) for value in size
    )


def test_style_levels_keep_the_data_order_across_panels():
    data = {"x": [1.0, 2.0, 3.0], "y": [1.0, 2.0, 3.0], "g": ["a", "b", "b"], "c": ["z", "y", "z"]}
    svg = (
        gg.ggplot(data, gg.aes("x", "y", color="c")) + gg.geom_point() + gg.facet_wrap("g")
    ).to_svg()
    legend = svg[svg.rindex('class="legend') :]
    labels = re.findall(r">([yz])</text>", legend)
    assert labels == ["z", "y"]


def test_errors_name_the_facet_option():
    plot = gg.ggplot(POINTS, gg.aes("x", "y")) + gg.geom_point()
    with pytest.raises(ValueError, match=r"facet\.facets.*'missing'"):
        (plot + gg.facet_wrap("missing")).to_svg()
    with pytest.raises(ValueError, match=r"facet\.scales"):
        (plot + gg.facet_wrap("g", scales="loose")).to_svg()


def _fills(svg: str) -> list[str]:
    """Circle fills in drawing order (a fill set on the enclosing group counts)."""
    fills = []
    for group, body in re.findall(r"<g([^>]*)>((?:\s*<circle [^>]*>(?:\s*</circle>)?)+)", svg):
        shared = _attributes(group).get("fill")
        for circle in re.findall(r"<circle [^>]*>", body):
            fills.append(_hex(_attributes(circle).get("fill", shared)))
    return fills


def test_breaks_lead_the_color_domain_in_every_panel():
    expected_svg, actual_svg = _both(CASES["color_breaks_lead"])
    assert _fills(actual_svg) == _fills(expected_svg)
