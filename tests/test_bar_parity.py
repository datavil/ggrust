"""Bars and histograms must match lets-plot: rectangles, paint, axis labels, titles.

Rectangles are compared as fractions of the panel (the panels differ by a few px
because the fonts do), in drawing order. Legends are turned off so only the
layer's rectangles are compared.
"""

import re
import warnings

import polars as pl
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

DATA = {
    "g": ["a", "b", "a", "c", "b", "a", "c", "a"],
    "h": ["u", "u", "v", "v", "u", "v", "u", "u"],
    "v": [1.0, 2.5, 3.0, 4.2, 4.9, 7.5, 2.2, 6.1],
    "w": [1.0, 2.0, 0.5, 1.0, 3.0, 1.0, 2.0, 1.0],
    "n": [1.0, 3.0, 3.0, 4.0, 7.0, 7.0, 1.0, 4.0],
}
SIGNED = {
    "x": [1.0, 2.0, 3.0, 1.0, 2.0, 3.0],
    "y": [2.0, -1.0, 3.0, -2.0, 1.5, 1.0],
    "k": list("ppqqpq"),
}
EMPTY = pl.DataFrame({"x": [1.0]}).filter(pl.col("x") > 5)
TAG = re.compile(r"<(/?)(g|rect|text|clipPath|tspan)\b([^>]*?)(/?)>([^<]*)")


def _attributes(tag: str) -> dict:
    return dict(re.findall(r'([\w-]+)="([^"]*)"', tag))


def _hex(color: str | None) -> str | None:
    match = re.fullmatch(r"rgb\((\d+),\s*(\d+),\s*(\d+)\)", color or "")
    if match:
        return "#" + "".join(f"{int(part):02x}" for part in match.groups())
    return color.lower() if color else None


def _lets_plot_bars(svg: str) -> tuple[list[tuple], tuple[float, float]]:
    """Bars (relative to the panel, which lets-plot translates to) and panel size."""
    clip = re.search(r'<rect x="[\d.]+" y="0\.0" width="([\d.]+)" height="([\d.]+)">', svg)
    width, height = float(clip.group(1)), float(clip.group(2))
    bars = []
    for tag in re.findall(r"<rect [^>]*stroke-width[^>]*>", svg):
        a = _attributes(tag)
        if float(a["stroke-width"]) == 0.0:
            continue  # backgrounds (legend box, panel)
        bars.append(
            (
                float(a["x"]) / width,
                float(a["y"]) / height,
                float(a["width"]) / width,
                float(a["height"]) / height,
                _hex(a["fill"]),
                round(float(a.get("fill-opacity", 1)), 2),
                _hex(a["stroke"]),
                round(float(a["stroke-width"]), 3),
            )
        )
    return bars, (width, height)


def _ggrust_bars(svg: str) -> list[tuple]:
    clip = _attributes(re.search(r"<clipPath[^>]*>\s*<rect([^>]*)/>", svg).group(1))
    left, top = float(clip["x"]), float(clip["y"])
    width, height = float(clip["width"]), float(clip["height"])
    panel = svg[svg.index('<g class="panel"') :]
    panel = panel[: panel.index('<g class="axis-x"')]
    bars, stack = [], []
    for closing, kind, attributes, _, _ in TAG.findall(panel):
        if kind == "g":
            if closing:
                stack.pop()
            else:
                stack.append(_attributes(attributes))
            continue
        if kind != "rect":
            continue
        a = {}
        for group in stack:
            a.update(group)
        a.update(_attributes(attributes))
        bars.append(
            (
                (float(a["x"]) - left) / width,
                (float(a["y"]) - top) / height,
                float(a["width"]) / width,
                float(a["height"]) / height,
                _hex(a.get("fill")),
                round(float(a.get("fill-opacity", 1)), 2),
                _hex(a.get("stroke")),
                round(float(a.get("stroke-width", 0)), 3),
            )
        )
    return bars


def _texts(svg: str) -> list[str]:
    """Visible texts in document order (tick labels, titles)."""
    flat = re.sub(r"\s*\n\s*", "", svg)
    return [text for text in re.findall(r">([^<>]+)</t(?:span|ext)>", flat) if text.strip()]


def _both(build):
    """Render `build(module)` with lets-plot and ggrust, legends off."""
    rendered = []
    for module in (lp, gg):
        plot = build(module) + module.theme(legend_position="none")
        rendered.append(plot.to_svg())
    return rendered


CASES = {
    "count": lambda m: m.ggplot(DATA, m.aes("g")) + m.geom_bar(),
    "stack": lambda m: m.ggplot(DATA, m.aes("g", fill="h")) + m.geom_bar(),
    "dodge": lambda m: m.ggplot(DATA, m.aes("g", fill="h")) + m.geom_bar(position="dodge"),
    "dodge_width": lambda m: (
        m.ggplot(DATA, m.aes("g", fill="h"))
        + m.geom_bar(position=m.position_dodge(width=0.6), width=0.5)
    ),
    "stack_vjust_all": lambda m: (
        m.ggplot(DATA, m.aes("g", fill="h"))
        + m.geom_bar(position=m.position_stack(vjust=0.5, mode="all"))
    ),
    "numeric_x": lambda m: m.ggplot(DATA, m.aes("n")) + m.geom_bar(),
    "weight": lambda m: m.ggplot(DATA, m.aes("g", weight="w")) + m.geom_bar(),
    "styled": lambda m: (
        m.ggplot(DATA, m.aes("g"))
        + m.geom_bar(width=0.5, alpha=0.4, color="black", size=1.5, fill="#ff8800")
    ),
    "identity_signed": lambda m: m.ggplot(SIGNED, m.aes("x", "y")) + m.geom_bar(stat="identity"),
    "identity_signed_stack": lambda m: (
        m.ggplot(SIGNED, m.aes("x", "y", fill="k")) + m.geom_bar(stat="identity")
    ),
    "fill_count": lambda m: m.ggplot(DATA, m.aes("g", fill="..count..")) + m.geom_bar(),
    "proportion": lambda m: (
        m.ggplot(DATA, m.aes("g", y="..prop..", fill="h")) + m.geom_bar(position="dodge")
    ),
    "histogram": lambda m: m.ggplot(DATA, m.aes("v")) + m.geom_histogram(bins=5),
    "histogram_default": lambda m: m.ggplot(DATA, m.aes("v")) + m.geom_histogram(),
    "histogram_groups": lambda m: m.ggplot(DATA, m.aes("v", fill="h")) + m.geom_histogram(bins=6),
    "histogram_boundary": lambda m: (
        m.ggplot(DATA, m.aes("v")) + m.geom_histogram(binwidth=1, boundary=0)
    ),
    "histogram_center": lambda m: (
        m.ggplot(DATA, m.aes("v")) + m.geom_histogram(binwidth=1.5, center=0)
    ),
    "histogram_breaks": lambda m: (
        m.ggplot(DATA, m.aes("v")) + m.geom_histogram(breaks=[0, 2, 3, 5, 8])
    ),
    "histogram_density": lambda m: (
        m.ggplot(DATA, m.aes("v", y="..density..")) + m.geom_histogram(bins=4)
    ),
    "histogram_dodge": lambda m: (
        m.ggplot(DATA, m.aes("v", fill="h")) + m.geom_histogram(bins=4, position="dodge")
    ),
    "count_log10": lambda m: m.ggplot(DATA, m.aes("g")) + m.geom_bar() + m.scale_y_log10(),
    "count_sqrt": lambda m: (
        m.ggplot(DATA, m.aes("g")) + m.geom_bar() + m.scale_y_continuous(trans="sqrt")
    ),
    "count_reverse": lambda m: m.ggplot(DATA, m.aes("g")) + m.geom_bar() + m.scale_y_reverse(),
    "count_y_limits": lambda m: (
        m.ggplot(DATA, m.aes("g")) + m.geom_bar() + m.scale_y_continuous(limits=[2, 5])
    ),
    "histogram_log10": lambda m: (
        m.ggplot(DATA, m.aes("v")) + m.geom_histogram(bins=3) + m.scale_y_log10()
    ),
    "histogram_x_limits": lambda m: (
        m.ggplot(DATA, m.aes("v")) + m.geom_histogram(bins=4) + m.xlim(0, 10)
    ),
    "histogram_shared_range": lambda m: (
        m.ggplot(DATA)
        + m.geom_histogram(m.aes("v"), bins=5)
        + m.geom_histogram(m.aes("n"), bins=5, fill="red")
    ),
    "signed_zero": lambda m: m.ggplot({"x": [0.0, -0.0, 1.0, -0.0]}, m.aes("x")) + m.geom_bar(),
    "bars_past_x_limits": lambda m: (
        m.ggplot({"x": [1.0, 2, 3], "y": [5.0, 6, 7]}, m.aes("x", "y"))
        + m.geom_bar(stat="identity")
        + m.scale_x_continuous(limits=[1.5, 3])
    ),
    "histogram_threshold": lambda m: (
        m.ggplot(DATA, m.aes("v", fill="h")) + m.geom_histogram(bins=8, threshold=0)
    ),
}


@pytest.mark.parametrize("name", sorted(CASES))
def test_bars_match_lets_plot(name):
    expected_svg, actual_svg = _both(CASES[name])
    expected, _ = _lets_plot_bars(expected_svg)
    actual = _ggrust_bars(actual_svg)
    assert len(actual) == len(expected), (actual, expected)
    for got, want in zip(actual, expected, strict=True):
        # Geometry to 1e-6 of the panel; empty bars are 0.1 px on both sides.
        assert got[:4] == pytest.approx(want[:4], abs=1e-3), (got, want)
        assert got[4:] == want[4:], (got, want)
    # Same tick labels and titles (lets-plot writes the y title first).
    assert sorted(_texts(actual_svg)) == sorted(_texts(expected_svg))


def test_dodged_points_match_lets_plot():
    data = {"g": list("aabbab"), "h": list("uvuvww"), "y": [1.0, 2, 3, 4, 5, 6]}
    expected_svg, actual_svg = _both(
        lambda m: (
            m.ggplot(data, m.aes("g", "y", color="h"))
            + m.geom_point(position=m.position_dodge(width=0.5))
        )
    )
    _, (width, _) = _lets_plot_bars(expected_svg)
    expected = [float(x) / width for x in re.findall(r'<circle[^>]*cx="([\d.]+)"', expected_svg)]
    clip = _attributes(re.search(r"<clipPath[^>]*>\s*<rect([^>]*)/>", actual_svg).group(1))
    actual = [
        (float(x) - float(clip["x"])) / float(clip["width"])
        for x in re.findall(r'<circle[^>]*cx="([\d.]+)"', actual_svg)
    ]
    assert actual == pytest.approx(expected, abs=1e-3)


@pytest.mark.parametrize("geom", ["geom_bar", "geom_histogram"])
def test_empty_data_draws_an_empty_plot(geom):
    """lets-plot draws no bars; its y title on empty data ("y") is not matched."""
    expected_svg, actual_svg = _both(lambda m: m.ggplot(EMPTY, m.aes("x")) + getattr(m, geom)())
    assert _lets_plot_bars(expected_svg)[0] == _ggrust_bars(actual_svg) == []


def test_discrete_order_ignores_the_stat_grouping():
    """Deliberate difference: lets-plot orders x by the stat output (first sample's
    order: b, a); ggrust keeps the data's order (a, b), as ggplot2 does."""
    data = {"x": ["a", "b", "b", "a"], "fill": ["s2", "s1", "s2", "s2"]}
    svg = (gg.ggplot(data, gg.aes("x", fill="fill")) + gg.geom_bar()).to_svg()
    labels = [text for text in _texts(svg) if text in ("a", "b")]
    assert labels == ["a", "b"]
