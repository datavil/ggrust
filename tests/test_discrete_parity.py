"""Discrete position axes must match lets-plot: category order, limits, reverse,
expansion and numbers passing through.

Compares tick labels and where each point sits in units of the category spacing.
"""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402


def lets_plot_axis(svg: str):
    svg = re.sub(r"\s+", " ", svg)
    ticks = re.findall(
        r'<g transform="translate\(([-\d.]+) 0\.0 \) "> <line[^>]*> </line> '
        r'<g transform="translate\(0\.0 6\.0 \) "> <text[^>]*class="axis-text-x"[^>]*> '
        r"<tspan>([^<]*)</tspan>",
        svg,
    )
    circles = [float(value) for value in re.findall(r'<circle[^>]*cx="([-\d.]+)"', svg)]
    return [(float(x), label) for x, label in ticks], circles


def ggrust_axis(svg: str):
    group = re.search(r'<g class="axis-x">(.*?)</g>', svg, re.S).group(1)
    ticks = re.findall(r'<text[^>]*x="([-\d.]+)"[^>]*>([^<]*)</text>', group)
    circles = [float(value) for value in re.findall(r'<circle[^>]*cx="([-\d.]+)"', svg)]
    return [(float(x), label) for x, label in ticks], circles


def in_category_units(ticks, circles):
    labels = [label for _, label in ticks]
    unit = ticks[1][0] - ticks[0][0] if len(ticks) >= 2 else 1.0
    return labels, [round((circle - ticks[0][0]) / unit, 3) for circle in circles]


FRAME = {"x": ["b", "a", "c", "a"], "y": [1.0, 2.0, 3.0, 4.0]}
OTHER = {"x": ["d", "a"], "y": [5.0, 6.0]}
NUMBERS = {"n": [-1.0, 4.0], "y": [1.0, 2.0]}


def base(m):
    return m.ggplot(FRAME, m.aes("x", "y")) + m.geom_point()


CASES = {
    "plain": base,
    "limits": lambda m: base(m) + m.scale_x_discrete(limits=["c", "b"]),
    "reversed": lambda m: base(m) + m.scale_x_discrete_reversed(),
    "two_layers": lambda m: base(m) + m.geom_point(data=OTHER),
    "numeric_layer": lambda m: base(m) + m.geom_point(m.aes("n", "y"), data=NUMBERS),
    "expand": lambda m: base(m) + m.scale_x_discrete(expand=[0.1, 0]),
    "xlim": lambda m: base(m) + m.xlim("a", "b"),
}


@pytest.mark.parametrize("name", CASES)
def test_discrete_axis_matches_lets_plot(name):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        theirs = in_category_units(*lets_plot_axis(CASES[name](lp).to_svg()))
        ours = in_category_units(*ggrust_axis(CASES[name](gg).to_svg()))
    assert ours == theirs


def lets_plot_arrangement(svg: str) -> str:
    svg = re.sub(r"\s+", " ", svg)
    labels = re.findall(
        r'<g transform="translate\(0\.0 ([-\d.]+) \) ?(?:rotate\(([-\d.]+) \))? ?"> '
        r'<text[^>]*class="axis-text-x"',
        svg,
    )
    angles = {angle for _, angle in labels}
    if "-90.0" in angles:
        return "vertical"
    if "-30.0" in angles:
        return "tilted"
    return "two_rows" if len({offset for offset, _ in labels}) > 1 else "simple"


def ggrust_arrangement(svg: str) -> str:
    group = re.search(r'<g class="axis-x">(.*?)</g>', svg, re.S).group(1)
    labels = re.findall(
        r'<text[^>]*y="([-\d.]+)"[^>]*?(?:transform="rotate\((-?\d+)[^"]*")?>', group
    )
    angles = {angle for _, angle in labels}
    if "-90" in angles:
        return "vertical"
    if "-30" in angles:
        return "tilted"
    return "two_rows" if len({y for y, _ in labels}) > 1 else "simple"


LABEL_SETS = {
    "cells": ["T cells", "B cells", "NK", "Monocytes"],
    "long": ["CD14+ Monocytes", "FCGR3A+ Monocytes", "Megakaryocytes", "Dendritic cells"],
    "genes": [f"GENE{index}" for index in range(12)],
    "mid": [f"category {index}" for index in range(6)],
    "words": ["first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth"],
    "long_last": ["a", "b", "c", "a long last label"],
    "long_ends": ["leftmost label", "m", "n", "rightmost label"],
    "strip": ["B cells", "T cells", "NK", "Monocytes", "Dendritic"],
    "mixed": ["x", "a very long category name", "y", "z"],  # two rows at 300 px
}


@pytest.mark.parametrize("width", range(300, 1101, 100))
@pytest.mark.parametrize("name", LABEL_SETS)
def test_label_arrangement_matches_lets_plot(name, width):
    labels = LABEL_SETS[name]
    frame = {"x": labels, "y": [float(index) for index in range(len(labels))]}
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        theirs, ours = (
            arrangement(
                (m.ggplot(frame, m.aes("x", "y")) + m.geom_point() + m.ggsize(width, 320)).to_svg()
            )
            for m, arrangement in ((lp, lets_plot_arrangement), (gg, ggrust_arrangement))
        )
    assert ours == theirs
