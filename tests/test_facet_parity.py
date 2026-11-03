"""facet_wrap and facet_grid must match lets-plot: panels, strip labels, data in each panel.

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
# Long strip labels for labwidth (and one with a newline).
LABELS = {
    "x": RANDOM.normal(size=N).round(3).tolist(),
    "y": RANDOM.normal(size=N).round(3).tolist(),
    "tissue": RANDOM.choice(["peripheral blood", "bone marrow", "lymph node"], N).tolist(),
    "state": RANDOM.choice(["resting cells", "activated\ncells"], N).tolist(),
}


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
        for text in re.findall(r'class="facet-strip-text-[xy]"[^>]*>(.*?)</text>', svg, re.S)
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
    "grid": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y")) + m.geom_point() + m.facet_grid(x="g", y="h")
    ),
    "grid_free": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y"))
        + m.geom_point()
        + m.facet_grid(x="h", y="g", scales="free")
    ),
    "grid_columns_free_y": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y")) + m.geom_point() + m.facet_grid(x="g", scales="free_y")
    ),
    "grid_rows_numbers": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y")) + m.geom_point() + m.facet_grid(y="k")
    ),
    "grid_discrete_free_x": lambda m: (
        m.ggplot(POINTS, m.aes("cell", "y"))
        + m.geom_point()
        + m.facet_grid(x="h", y="g", scales="free_x")
    ),
    "grid_empty_panel_and_layer_without_column": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y"))
        + m.geom_point(data={"x": [0.0, 1.0], "y": [0.0, 1.0], "a": ["u", "v"], "b": ["s", "t"]})
        + m.geom_point(data=OTHER, color="red")
        + m.facet_grid(x="a", y="b")
    ),
    "grid_theme_grey": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y"))
        + m.geom_point()
        + m.facet_grid(x="h", y="cell")
        + m.theme_grey()
    ),
    "grid_orders_and_formats": lambda m: (
        m.ggplot(POINTS, m.aes("x", "y"))
        + m.geom_point()
        + m.facet_grid(x="g", y="k", x_order=-1, y_order=0, x_format="{} group", y_format=".2f")
    ),
    "grid_labwidth": lambda m: (
        m.ggplot(LABELS, m.aes("x", "y"))
        + m.geom_point()
        + m.facet_grid(x="tissue", y="state", x_labwidth=8, y_labwidth=6)
        + m.theme_grey()
    ),
    "wrap_labwidth_and_newlines": lambda m: (
        m.ggplot(LABELS, m.aes("x", "y"))
        + m.geom_point()
        + m.facet_wrap(["tissue", "state"], labwidth=[10, None], ncol=3)
        # Taller plot: the strips are 4 lines, each a little taller than lets-plot's.
        + m.ggsize(800, 900)
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


@pytest.mark.parametrize(
    ("layer", "scales"),
    [("geom_histogram", "free_x"), ("geom_histogram", "free"), ("geom_density", "free_x")],
)
def test_free_scales_bin_each_panel_over_its_own_range(layer, scales):
    """Deliberate difference (lets-plot #1443): on a free axis a binned stat uses the
    panel's range, as ggplot2; lets-plot bins every panel over the whole plot's range."""
    data = {"v": [0.0, 1.0, 1.5, 2.0, 3.0, 100.0, 104.0, 105.0, 110.0], "g": list("aaaaabbbb")}
    options = {"bins": 6} if layer == "geom_histogram" else {}
    svg = (
        gg.ggplot(data, gg.aes("v"))
        + getattr(gg, layer)(**options)
        + gg.facet_wrap("g", scales=scales)
    ).to_svg()
    panels = _ggrust_panels(svg)
    if layer == "geom_histogram":
        # Six bins per panel, spanning most of each panel (not 3 % of it).
        for panel in panels:
            bars = [rect for rect in panel["rects"] if rect[3] > 0]
            assert len(panel["rects"]) == 6
            assert sum(rect[2] for rect in panel["rects"]) > 0.8
            assert bars
    else:
        # Each density curve spans its own panel.
        paths = re.findall(r'<g class="panel".*?</g>', svg, flags=re.DOTALL)
        assert len(paths) == 2
    # The shared-scale plot keeps the plot-wide bins (lets-plot parity).
    shared = (
        gg.ggplot(data, gg.aes("v")) + getattr(gg, layer)(**options) + gg.facet_wrap("g")
    ).to_svg()
    if layer == "geom_histogram":
        assert all(
            sum(rect[2] for rect in panel["rects"]) > 0.8 for panel in _ggrust_panels(shared)
        )


def test_grid_free_x_bins_each_column_over_its_own_range():
    """As above for facet_grid: every panel of a grid column bins over the column's
    range, and the panels of a column share one x scale."""
    data = {
        "v": [0.0, 1.0, 2.0, 3.0, 100.0, 104.0, 108.0, 110.0] * 2,
        "g": list("aaaabbbb") * 2,
        "r": ["u"] * 8 + ["v"] * 8,
    }
    svg = (
        gg.ggplot(data, gg.aes("v"))
        + gg.geom_histogram(bins=6)
        + gg.facet_grid(x="g", y="r", scales="free_x")
    ).to_svg()
    panels = _ggrust_panels(svg)
    assert len(panels) == 4
    for panel in panels:
        assert len(panel["rects"]) == 6
        assert sum(rect[2] for rect in panel["rects"]) > 0.8
    # Same data in both rows: the panels of a column are identical.
    assert panels[0]["rects"] == panels[2]["rects"]
    assert panels[1]["rects"] == panels[3]["rects"]


def test_grid_errors_name_the_facet_option():
    plot = gg.ggplot(POINTS, gg.aes("x", "y")) + gg.geom_point()
    with pytest.raises(gg.errors.SpecError, match=r"facet\.x.*'missing'"):
        (plot + gg.facet_grid(x="missing")).to_svg()
    with pytest.raises(gg.errors.SpecError, match=r"facet\.y.*'nope'"):
        (plot + gg.facet_grid(x="g", y="nope")).to_svg()
    with pytest.raises(gg.errors.SpecError, match=r"facet\.y.*given twice"):
        (plot + gg.facet_grid(x="g", y="g")).to_svg()
    with pytest.raises(gg.errors.SpecError, match=r"give x \(columns\), y \(rows\) or both"):
        (plot + gg.facet_grid()).to_svg()
    with pytest.raises(gg.errors.SpecError, match=r"facet\.scales"):
        (plot + gg.facet_grid(x="g", scales="loose")).to_svg()


def test_grid_with_default_options_does_not_warn():
    plot = gg.ggplot(POINTS, gg.aes("x", "y")) + gg.geom_point() + gg.facet_grid(x="g", y="h")
    with warnings.catch_warnings():
        warnings.simplefilter("error", gg.GgrustWarning)
        plot.to_svg()


def test_facet_label_options_do_not_warn():
    base = gg.ggplot(POINTS, gg.aes("x", "y")) + gg.geom_point()
    facets = [
        gg.facet_grid(
            x="g", y="k", x_order=0, y_order=-1, x_format="{}", y_format=".1f", x_labwidth=3
        ),
        gg.facet_wrap(["g", "h"], labwidth=[4, 2], format=["{}", None]),
    ]
    with warnings.catch_warnings():
        warnings.simplefilter("error", gg.GgrustWarning)
        for facet in facets:
            (base + facet).to_svg()


def test_grid_row_strips_turn_and_the_legend_stays_right_of_them():
    plot = (
        gg.ggplot(POINTS, gg.aes("x", "y", color="cell"))
        + gg.geom_point()
        + gg.facet_grid(y="h")
        + gg.theme(strip_background_y=gg.element_rect(fill="#eeeeee"))
    )
    svg = plot.to_svg()
    assert _ggrust_strips(svg) == ["p", "q"]
    strip = _attributes(re.search(r'<g class="strips">\s*<g[^>]*>\s*<rect ([^>]*)/>', svg).group(1))
    assert float(strip["height"]) > 3 * float(strip["width"])
    text = svg[svg.index('<g class="strips">') :]
    assert "rotate(90" in text[: text.index("</text>")]
    legend = svg[svg.index('class="legend') :]
    legend_x = min(float(x) for x in re.findall(r'<rect x="([\d.]+)"', legend))
    assert legend_x > float(strip["x"]) + float(strip["width"])
