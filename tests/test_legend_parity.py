"""Discrete legends must match lets-plot: titles, order, labels and key styles.

Both SVGs are read into ``[(title, [(label, [(fill, radius, opacity), ...]), ...]), ...]``:
one entry per legend, one per key, one style per layer drawn in the key. Positions
are not compared (ggrust measures text with the real font); key radii are.
"""

import re
import warnings

import numpy as np
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

TOKEN = re.compile(
    r"<g [^>]*>|</g>|<circle[^>]*>|<text[^>]*class=\"(legend-title|legend-item)\"[^>]*>\s*"
    r"<tspan>([^<]*)</tspan>|<text[^>]*font-size=\"(\d+)\"[^>]*>([^<]*)</text>"
)


def _attributes(tag: str) -> dict:
    return dict(re.findall(r'([\w-]+)="([^"]*)"', tag))


def _key_style(tag: str, inherited: tuple[str, str]) -> tuple[str, float, int]:
    """(fill, radius, opacity in 1/255 steps: ggrust stores alpha in 8 bits)."""
    attributes = _attributes(tag)
    fill = attributes.get("fill", inherited[0]).lower()
    opacity = float(attributes.get("fill-opacity", attributes.get("opacity", inherited[1])))
    return fill, round(float(attributes["r"]), 2), round(opacity * 255)


def legends(svg: str, *, ours: bool) -> list:
    """Legends of an SVG, in drawing order."""
    if ours:
        start = svg.find('<g class="legend">')
    else:
        start = svg.find('class="legend-title"')
        start = svg.rfind("<g", 0, start)
    if start < 0:
        return []
    result = []
    keys: list = []
    groups = [("", "1")]  # inherited (fill, fill-opacity)
    for match in TOKEN.finditer(svg, start):
        tag = match.group(0)
        if tag.startswith("<g "):
            attributes = _attributes(tag)
            fill, opacity = groups[-1]
            groups.append((attributes.get("fill", fill), attributes.get("fill-opacity", opacity)))
        elif tag == "</g>":
            if len(groups) > 1:
                groups.pop()
        elif tag.startswith("<circle"):
            keys.append(_key_style(tag, groups[-1]))
        else:
            kind, text = match.group(1), match.group(2)
            if kind is None:  # ggrust: the title is the 15 px text, labels 13 px
                kind = "legend-title" if match.group(3) == "15" else "legend-item"
                text = match.group(4)
            if kind == "legend-title":
                result.append((text, []))
                keys = []
            elif result:
                result[-1][1].append((text, keys))
                keys = []
    return result


RANDOM = np.random.default_rng(5)
N = 30
FRAME = {
    "x": [float(index) for index in range(N)],
    "y": list(RANDOM.uniform(0, 10, N)),
    "c": [["b", "a", "c"][index % 3] for index in range(N)],
    "s": [f"s{index % 2}" for index in range(N)],
    "z": [[0.0, -0.0, 1.0][index % 3] for index in range(N)],
    "g20": [f"g{index % 20:02d}" for index in range(N)],
}


def point(m, **mapping):
    return m.ggplot(FRAME, m.aes("x", "y", **mapping)) + m.geom_point()


CASES = {
    "default": lambda m: point(m, color="c"),
    "hue": lambda m: point(m, color="c") + m.scale_color_hue(),
    "hue_options": lambda m: (
        point(m, color="c") + m.scale_color_hue(h=[0, 120], c=50, l=80, h_start=30, direction=-1)
    ),
    "manual_list": lambda m: point(m, color="c") + m.scale_color_manual(values=["red", "blue"]),
    "manual_dict": lambda m: (
        point(m, color="c") + m.scale_color_manual(values={"a": "red", "c": "green"})
    ),
    "limits": lambda m: point(m, color="c") + m.scale_color_hue(limits=["c", "a"]),
    "breaks_labels": lambda m: (
        point(m, color="c") + m.scale_color_hue(breaks=["c", "a"], labels=["C", "A"])
    ),
    "breaks_not_in_data": lambda m: point(m, color="c") + m.scale_color_hue(breaks=["z", "a"]),
    "labels_only": lambda m: point(m, color="c") + m.scale_color_hue(labels=["B1", "A1"]),
    "limits_breaks_labels": lambda m: (
        point(m, color="c")
        + m.scale_color_hue(limits=["c", "a", "b"], breaks=["b", "c"], labels=["BB", "CC"])
    ),
    "name_and_na": lambda m: (
        point(m, color="c")
        + m.scale_color_manual(
            values=["red", "blue"], name="Cells", limits=["a", "b"], na_value="pink"
        )
    ),
    "labs_title": lambda m: point(m, color="c") + m.labs(color="Cluster"),
    "size_and_color": lambda m: point(m, color="c", size="s"),
    "alpha_discrete": lambda m: point(m, alpha="s"),
    "constant_color_in_size_key": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y", size="s")) + m.geom_point(color="red", size=8)
    ),
    "two_layers_one_legend": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y"))
        + m.geom_point(m.aes(color="c"))
        + m.geom_point(m.aes(color="s"), size=1)
    ),
    "mixed_with_signed_zeros": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y"))
        + m.geom_point(m.aes(color="c"))
        + m.geom_point(m.aes(color="z"), size=1)
    ),
    "same_title_merges": lambda m: (
        point(m, color="c", size="s") + m.labs(color="Group", size="Group")
    ),
    "show_legend_false": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y"))
        + m.geom_point(m.aes(color="c"), show_legend=False)
        + m.geom_point(m.aes(size="s"))
    ),
    "guides_none": lambda m: point(m, color="c", size="s") + m.guides(color="none"),
    "twenty_keys": lambda m: point(m, color="g20"),
}


@pytest.mark.parametrize("name", CASES)
def test_legends_match_lets_plot(name):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        theirs = legends(CASES[name](lp).to_svg(), ours=False)
        ours = legends(CASES[name](gg).to_svg(), ours=True)
    assert ours == theirs
