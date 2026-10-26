"""Mapped color, size and alpha must match lets-plot point by point (default scales).

lets-plot also draws legend keys as circles after the panel; only the first n circles
(one per data row) are compared. ggrust stores alpha in 8 bits, so opacities may
differ by less than 1/255, and the SVG writers round radii differently (< 0.01 px).
"""

import re
import warnings

import numpy as np
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402


def circles(svg: str) -> list[tuple[str, float, float]]:
    """(fill, radius, opacity) per circle; a fill set on a parent ``<g>`` is inherited."""
    result = []
    group_fill = ""
    for tag in re.findall(r"<g [^>]*>|<circle[^>]*>", svg):
        attributes = dict(re.findall(r'([\w-]+)="([^"]*)"', tag))
        if tag.startswith("<g "):
            group_fill = attributes.get("fill", group_fill)
            continue
        fill = attributes.get("fill", group_fill)
        rgb = re.match(r"rgb\((\d+),(\d+),(\d+)\)", fill)
        if rgb:
            fill = "#{:02x}{:02x}{:02x}".format(*map(int, rgb.groups()))
        opacity = float(attributes.get("fill-opacity", attributes.get("opacity", "1")))
        result.append((fill.lower(), round(float(attributes["r"]), 3), opacity))
    return result


RANDOM = np.random.default_rng(3)
N = 40
VALUES = list(RANDOM.uniform(0, 10, N))
FRAME = {
    "x": [float(index) for index in range(N)],
    "y": [float(index) for index in range(N)],
    "c3": [["T", "B", "NK"][index % 3] for index in range(N)],
    "c12": [f"c{index % 12:02d}" for index in range(N)],
    "v": VALUES,
    "vna": [*VALUES[:-3], None, float("nan"), 4.0],
    "near_constant": [1.0 + index * 1e-14 for index in range(N)],
    "near_constant_negative": [-1.0 - index * 1e-14 for index in range(N)],
    "signed_zeros": [[0.0, -0.0, 1.0][index % 3] for index in range(N)],
    "one": ["only"] * N,
    "digits": [str(index % 4) for index in range(N)],
    "integers": [index % 5 for index in range(N)],
}


def scatter(m, **mapping):
    return m.ggplot(FRAME, m.aes("x", "y", **mapping)) + m.geom_point()


CASES = {
    "color_discrete_3": lambda m: scatter(m, color="c3"),
    "color_discrete_12": lambda m: scatter(m, color="c12"),
    "color_continuous": lambda m: scatter(m, color="v"),
    "color_with_missing": lambda m: scatter(m, color="vna"),
    "fill_discrete": lambda m: scatter(m, fill="c3"),
    "size_continuous": lambda m: scatter(m, size="v"),
    "size_discrete": lambda m: scatter(m, size="c3"),
    "alpha_continuous": lambda m: scatter(m, alpha="v"),
    "color_near_constant": lambda m: scatter(m, color="near_constant"),
    "size_near_constant": lambda m: scatter(m, size="near_constant"),
    "color_near_constant_negative": lambda m: scatter(m, color="near_constant_negative"),
    "two_layers_join_categories": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y"))
        + m.geom_point(m.aes(color="c3"))
        + m.geom_point(m.aes(color="c12"), size=1)
    ),
    "hue_discrete": lambda m: scatter(m, color="c3") + m.scale_color_hue(),
    "hue_twelve_options": lambda m: (
        scatter(m, color="c12")
        + m.scale_color_hue(h=[0, 360], c=60, l=50, h_start=90, direction=-1)
    ),
    "hue_continuous": lambda m: scatter(m, color="v") + m.scale_color_hue(),
    "hue_one_level": lambda m: scatter(m, color="one") + m.scale_fill_hue() + m.scale_color_hue(),
    "manual_cycle": lambda m: (
        scatter(m, color="c12") + m.scale_color_manual(values=["red", "blue"])
    ),
    "manual_dict_breaks_order": lambda m: (
        scatter(m, color="c3") + m.scale_color_manual(values={"NK": "red", "T": "#00ff00"})
    ),
    "manual_continuous": lambda m: (
        scatter(m, color="vna") + m.scale_color_manual(values=["red", "blue", "green"])
    ),
    "limits_make_na": lambda m: (
        scatter(m, color="c3") + m.scale_color_hue(limits=["B", "T"], na_value="orange")
    ),
    "size_na_value": lambda m: scatter(m, size="vna"),
    "hue_reversed_range": lambda m: scatter(m, color="c12") + m.scale_color_hue(h=[200, 100]),
    "hue_reversed_continuous": lambda m: (
        scatter(m, color="v") + m.scale_color_hue(h=[300, 0], direction=-1)
    ),
    "na_value_ignored_on_default_scales": lambda m: (
        scatter(m, color="vna") + m.scale_color_manual(values=None, na_value="red")
    ),
    "discrete_then_continuous": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y"))
        + m.geom_point(m.aes(color="digits"))
        + m.geom_point(m.aes(color="integers"), size=1)
    ),
    "signed_zeros_are_categories": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y"))
        + m.geom_point(m.aes(color="c3"))
        + m.geom_point(m.aes(color="signed_zeros"), size=1)
    ),
    "continuous_then_discrete": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y"))
        + m.geom_point(m.aes(color="vna"))
        + m.geom_point(m.aes(color="c3"), size=1)
    ),
}


@pytest.mark.parametrize("name", CASES)
def test_mapped_styles_match_lets_plot(name):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        theirs = circles(CASES[name](lp).to_svg())
        ours = circles(CASES[name](gg).to_svg())
    assert len(theirs) >= len(ours) > 0
    for index, (their, our) in enumerate(zip(theirs, ours, strict=False)):
        assert our[0] == their[0], f"point {index}: {our} vs {their}"
        assert abs(our[1] - their[1]) < 0.01, f"point {index}: {our} vs {their}"
        assert abs(our[2] - their[2]) < 1 / 255, f"point {index}: {our} vs {their}"
