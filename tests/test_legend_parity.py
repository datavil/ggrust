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
            # A zero-size key is invisible: lets-plot writes it, ggrust does not.
            if float(_attributes(tag)["r"]) > 0:
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
    "p": [float(index) * 3.5 for index in range(N)],
    "w": [10 ** (index / 6) for index in range(N)],
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
    "wrapped_labels": lambda m: (
        m.ggplot(
            {
                "x": [1.0, 2.0, 3.0, 4.0],
                "y": [1.0, 2.0, 3.0, 4.0],
                "label": [
                    "a" * 60,
                    "short",
                    "this is a fairly long label with spaces in it ok",
                    "one\ntwo",
                ],
            },
            m.aes("x", "y", color="label"),
        )
        + m.geom_point()
    ),
    "scale_guide_none": lambda m: point(m, color="c", size="s") + m.scale_color_hue(guide="none"),
    "size_continuous": lambda m: point(m, size="y"),
    "alpha_continuous": lambda m: point(m, alpha="y"),
    "color_continuous_as_legend": lambda m: point(m, color="y") + m.guides(color="legend"),
    "viridis_discrete": lambda m: point(m, color="c") + m.scale_color_viridis(option="C"),
    "gradient_discrete": lambda m: (
        point(m, color="c") + m.scale_color_gradient(low="red", high="blue")
    ),
    "size_breaks_labels": lambda m: point(m, size="y") + m.guides(size="legend"),
    "guide_legend_title": lambda m: (
        point(m, color="c") + m.guides(color={"name": "legend", "title": "G"})
    ),
    # Phase 1.5c: size and alpha scales.
    "size_sqrt": lambda m: point(m, size="p") + m.scale_size(trans="sqrt"),
    "size_sqrt_breaks": lambda m: (
        point(m, size="p") + m.scale_size(trans="sqrt", breaks=[25, 50, 75, 100])
    ),
    "size_log10": lambda m: point(m, size="w") + m.scale_size(trans="log10"),
    "size_reverse": lambda m: point(m, size="p") + m.scale_size(trans="reverse"),
    "size_range": lambda m: point(m, size="p") + m.scale_size(range=[1, 4]),
    "size_range_discrete": lambda m: point(m, size="c") + m.scale_size(range=[2, 6]),
    # scale_size_area is a recorded deviation (area, not diameter, follows the
    # value; lets-plot#1452): see test_identity_parity.py.
    "alpha_range": lambda m: point(m, alpha="p") + m.scale_alpha(range=[0.5, 0.9]),
    "size_manual": lambda m: point(m, size="c") + m.scale_size_manual(values=[2, 4, 8]),
    "alpha_manual": lambda m: (
        point(m, alpha="c") + m.scale_alpha_manual(values={"a": 0.2, "b": 0.6, "c": 1})
    ),
    "size_limits_trans": lambda m: point(m, size="p") + m.scale_size(trans="sqrt", limits=[10, 90]),
    "override_size": lambda m: (
        point(m, color="c") + m.guides(color=m.guide_legend(override_aes={"size": 8}))
    ),
    "override_lists": lambda m: (
        point(m, color="c")
        + m.guides(
            color=m.guide_legend(override_aes={"color": ["red", None, "blue"], "size": [2, 6]})
        )
    ),
    "override_merged": lambda m: (
        point(m, color="c", size="c")
        + m.guides(
            color=m.guide_legend(override_aes={"size": 2}),
            size=m.guide_legend(override_aes={"alpha": 0.3}),
        )
    ),
    "override_other_legend": lambda m: (
        point(m, color="c", size="s")
        + m.guides(size=m.guide_legend(override_aes={"color": "#4daf4a"}))
    ),
    "override_in_scale_guide": lambda m: (
        point(m, color="c") + m.scale_color_hue(guide=m.guide_legend(override_aes={"alpha": 0.5}))
    ),
    "override_empty_list": lambda m: (
        point(m, color="c") + m.guides(color=m.guide_legend(override_aes={"size": []}))
    ),
}


# Cases with an explicit na_value and values mapped to it: ggrust's extra NA key.
NA_KEY_CASES = {"name_and_na": [("#ffc0cb", 5.5, 255)]}

# Cases where two levels share a label: lets-plot merges them into one key (#1129),
# ggrust keeps one key per drawn color, as ggplot2 (deviations.md). Index and key
# of the extra ggrust key.
REPEATED_LABEL_CASES = {
    "mixed_with_signed_zeros": (3, ("0", [("#984ea3", 5.5, 255), ("#984ea3", 5.5, 255)]))
}


@pytest.mark.parametrize("name", CASES)
def test_legends_match_lets_plot(name):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        theirs = legends(CASES[name](lp).to_svg(), ours=False)
        ours = legends(CASES[name](gg).to_svg(), ours=True)
    if name in NA_KEY_CASES:
        # ggrust adds an "NA" key for an explicit na_value (deviations.md, ua.8).
        title, keys = ours[-1]
        assert keys[-1] == ("NA", NA_KEY_CASES[name])
        ours = [*ours[:-1], (title, keys[:-1])]
    if name in REPEATED_LABEL_CASES:
        index, extra = REPEATED_LABEL_CASES[name]
        title, keys = ours[0]
        assert keys[index] == extra
        ours = [(title, keys[:index] + keys[index + 1 :]), *ours[1:]]
    assert ours == theirs


def test_a_legend_never_covers_the_panel():
    """A plot too narrow for its legend keeps the panel's minimum width; the legend
    then runs past the plot edge (as in lets-plot) instead of covering the data."""
    import re as regex

    labels = [f"Cell type number {index:02d}" for index in range(45)]
    plot = (
        gg.ggplot(
            {"x": list(range(45)), "y": list(range(45)), "c": labels}, gg.aes("x", "y", color="c")
        )
        + gg.geom_point()
        + gg.ggsize(200, 300)
    )
    svg = plot.to_svg()
    clip = regex.search(r'<clipPath[^>]*><rect x="([\d.]+)" y="[\d.]+" width="([\d.]+)"', svg)
    panel_right = float(clip.group(1)) + float(clip.group(2))
    legend = svg[svg.find('<g class="legend">') :]
    legend_left = float(regex.search(r'<rect x="([\d.-]+)"', legend).group(1))
    assert legend_left >= panel_right


def repeated_label_keys(plot) -> list:
    return [
        (label, [style[0] for style in styles])
        for label, styles in legends(plot.to_svg(), ours=True)[0][1]
    ]


def test_repeated_labels_keep_their_own_keys():
    """lets-plot #1129: two levels labelled alike merge and a color vanishes;
    ggrust keeps one key per level, as ggplot2."""
    frame = {"x": [1, 2, 3], "y": [1, 2, 3], "g": ["a", "b", "c"]}
    plot = (
        gg.ggplot(frame, gg.aes("x", "y", color="g"))
        + gg.geom_point()
        + gg.scale_color_manual(values=["red", "green", "blue"], labels=["A", "A", "C"])
    )
    assert repeated_label_keys(plot) == [
        ("A", ["#ff0000"]),
        ("A", ["#008000"]),
        ("C", ["#0000ff"]),
    ]
    # A second layer joins the keys by label and occurrence.
    assert repeated_label_keys(plot + gg.geom_point(size=1)) == [
        ("A", ["#ff0000", "#ff0000"]),
        ("A", ["#008000", "#008000"]),
        ("C", ["#0000ff", "#0000ff"]),
    ]


def test_repeated_labels_toggle_on_their_own_in_html():
    """Interactive legend ids tell repeated labels apart: clicking one key hides
    only its own marks."""
    frame = {"x": [1.0, 2.0, 3.0, 4.0], "y": [1.0, 2.0, 3.0, 4.0], "g": ["a", "b", "c", "d"]}
    plot = (
        gg.ggplot(frame, gg.aes("x", "y", color="g"))
        + gg.geom_point()
        + gg.scale_color_manual(
            values=["red", "blue", "green", "black"], labels=["z", "z", "y", "y"]
        )
    )
    html = plot.to_html()
    key_ids = re.findall(r'data-key="([^"]*)"', html)
    legend_ids = [key for key in key_ids if "z" in key or "y" in key]
    assert len(set(legend_ids)) == 4, legend_ids
    for key in set(legend_ids):
        # Every legend key's id names marks too (the batch's key table).
        assert html.count(key.split("&quot;")[1]) >= 2
