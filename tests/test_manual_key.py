"""Manual legends (`manual_key=`, `layer_key()`) match lets-plot: titles, labels, order.

Legend text is read from both SVGs as a flat list of ``("title" | "item", text)`` in
drawing order. Key drawings are checked against lets-plot in the docs gallery and by
eye; here the key colors of a few cases are checked directly.
"""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

DATA = {"x": [1, 2, 3, 4], "y": [1, 3, 2, 4], "c": ["p", "q", "p", "q"]}


def legend_text(svg: str, *, ours: bool) -> list[tuple[str, str]]:
    """Legend titles and labels in drawing order."""
    if not ours:
        found = re.findall(r'class="(legend-title|legend-item)"[^>]*>\s*<tspan>([^<]*)', svg)
        return [("title" if kind == "legend-title" else "item", text) for kind, text in found]
    result = []
    for legend in re.findall(
        r'<g class="legend">(.*?)</g>\s*(?=<g class="legend">|</svg>)', svg, re.S
    ):
        for size, text in re.findall(r'<text[^>]*font-size="(\d+)"[^>]*>([^<]*)</text>', legend):
            result.append(("title" if size == "15" else "item", text))
    return result


def build(module, case):
    base = module.ggplot(DATA, module.aes("x", "y"))
    return case(module, base)


CASES = {
    "labels in layer order": lambda m, b: (
        b + m.geom_point(manual_key="pts") + m.geom_line(manual_key="line")
    ),
    "index orders keys": lambda m, b: (
        b
        + m.geom_point(manual_key=m.layer_key("a", index=5))
        + m.geom_line(manual_key="b")
        + m.geom_area(manual_key=m.layer_key("c", index=0))
    ),
    "groups make legends": lambda m, b: (
        b + m.geom_point(manual_key=m.layer_key("pts", group="G")) + m.geom_line(manual_key="line")
    ),
    "labs titles the manual legend": lambda m, b: (
        b
        + m.geom_point(manual_key="pts")
        + m.geom_line(manual_key="line")
        + m.labs(manual="My title")
    ),
    "guides titles a group": lambda m, b: (
        b
        + m.geom_point(manual_key=m.layer_key("pts", group="G"))
        + m.guides(G=m.guide_legend("Group title"))
    ),
    "same label merges": lambda m, b: (
        b + m.geom_point(manual_key="both") + m.geom_line(manual_key="both")
    ),
    "scale legend first": lambda m, b: (
        b
        + m.geom_point(m.aes(color="c"), manual_key="pts")
        + m.geom_line(manual_key=m.layer_key("line", group="Other"))
    ),
    "shows without show_legend": lambda m, b: (
        b + m.geom_point(manual_key="pts", show_legend=False) + m.geom_line(manual_key="line")
    ),
    "guides none hides": lambda m, b: b + m.geom_point(manual_key="pts") + m.guides(manual="none"),
    "group titled like a scale joins it": lambda m, b: (
        b + m.geom_point(m.aes(color="c")) + m.geom_line(manual_key=m.layer_key("line", group="c"))
    ),
}


@pytest.mark.parametrize("name", CASES)
def test_manual_legend_text_matches_lets_plot(name):
    theirs = legend_text(build(lp, CASES[name]).to_svg(), ours=False)
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        ours = legend_text(build(gg, CASES[name]).to_svg(), ours=True)
    assert ours == theirs


def test_layer_key_matches_lets_plot_as_dict():
    assert gg.layer_key("a").as_dict() == lp.layer_key("a").as_dict()
    ours = gg.layer_key("a", "g", index=2, color="red", size=3)
    assert ours.as_dict() == lp.layer_key("a", "g", index=2, color="red", size=3).as_dict()


def test_key_takes_layer_constants_and_layer_key_values():
    plot = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_point(
        color="#ff0000", size=5, manual_key=gg.layer_key("pts", color="#00ff00")
    )
    legend = plot.to_svg().split('<g class="legend">', 1)[1]
    assert 'fill="#00ff00"' in legend
    assert 'r="5.5"' in legend  # size 5 from the layer: 5 * 2.2 / 2, as lets-plot


@pytest.mark.parametrize(
    ("manual_key", "path", "message"),
    [
        ({"group": "g"}, "layers[0].manual_key.label", "needs a text label"),
        ({"label": "a", "colr": "red"}, "layers[0].manual_key.colr", "unknown aesthetic 'colr'"),
        ({"label": "a", "x": 1}, "layers[0].manual_key.x", "not a style aesthetic"),
        ({"label": "a", "color": "reddish"}, "layers[0].manual_key.color", "invalid color"),
        ({"label": "a", "index": "first"}, "layers[0].manual_key.index", "whole number"),
        (3, "layers[0].manual_key", "expected a label"),
    ],
)
def test_bad_manual_keys_are_spec_errors(manual_key, path, message):
    layer = gg.geom_point()
    layer.props()["manual_key"] = manual_key
    with pytest.raises(SpecError, match=re.escape(message)) as caught:
        (gg.ggplot(DATA, gg.aes("x", "y")) + layer).to_svg()
    assert caught.value.path == path


def test_layer_key_needs_a_text_label():
    with pytest.raises(TypeError, match="label must be a string"):
        gg.layer_key(3)  # type: ignore[arg-type]
