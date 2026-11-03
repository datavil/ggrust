"""Minor ticks and `exponent_format` (plan step lp5.8) against lets-plot.

Ticks are compared by length, direction, color and width (counted per kind);
labels as sorted texts with the superscript written `^n`.
"""

import json
import re
import shutil
import subprocess
import warnings
from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

DATA = {"x": [0, 1, 2, 3, 4, 5], "y": [0, 3, 1, 4, 2, 5]}
BIG = {"x": [0, 1e7, 2e7, 3e7], "y": [1e-9, 2e-9, 3e-9, 4e-9], "c": [1e7, 2e7, 3e7, 4e7]}
SMALL = {"x": [0, 1000, 2000, 3000], "y": [0.001, 0.002, 0.003, 0.004]}

TickKey = tuple[float, str, str, str]


def _rgb(hex_color: str) -> str:
    red, green, blue = (int(hex_color[index : index + 2], 16) for index in (0, 2, 4))
    return f"rgb({red},{green},{blue})"


def _tick(x1: float, y1: float, x2: float, y2: float, stroke: str, width: str) -> TickKey | None:
    length = round(abs(x2 - x1) + abs(y2 - y1), 2)
    if length == 0 or length > 15:
        return None
    width = width if "." in width else width + ".0"
    return (length, "h" if abs(x2 - x1) > 0 else "v", stroke, width)


def _ticks(svg: str) -> dict[TickKey, int]:
    """Short lines (ticks) by length, direction, color and width."""
    keys: list[TickKey | None] = []
    # lets-plot: <line> with x2/y2 relative to a translated group.
    for match in re.finditer(r"<line([^>]*)>", svg):
        attributes = match.group(1)

        def get(name: str, attributes: str = attributes) -> str | None:
            found = re.search(rf'\b{name}="([^"]*)"', attributes)
            return found.group(1) if found else None

        x1, y1, x2, y2 = (float(get(name) or 0) for name in ("x1", "y1", "x2", "y2"))
        keys.append(_tick(x1, y1, x2, y2, get("stroke") or "", get("stroke-width") or ""))
    # ggrust: one-segment paths.
    path = (
        r'<path d="M([-\d.]+) ([-\d.]+)L([-\d.]+) ([-\d.]+)" fill="none" '
        r'stroke="#([0-9a-fA-F]{6})" stroke-width="([\d.]+)"'
    )
    for match in re.finditer(path, svg):
        x1, y1, x2, y2 = (float(value) for value in match.groups()[:4])
        keys.append(_tick(x1, y1, x2, y2, _rgb(match.group(5)), match.group(6)))
    counts: dict[TickKey, int] = {}
    for key in keys:
        if key is not None:
            counts[key] = counts.get(key, 0) + 1
    return counts


def _texts(svg: str) -> list[str]:
    """Every text, superscripts as `^n`."""
    superscript = (
        r'<tspan[^>]*(?:font-size="[^"]*"[^>]*dy="-|baseline-shift="super)[^>]*>(.*?)</tspan>'
    )
    texts = []
    for match in re.finditer(r"<text[^>]*>(.*?)</text>", svg, flags=re.S):
        inner = re.sub(superscript, r"^\1", match.group(1))
        inner = re.sub(r"<[^>]+>", "", inner).replace("​", "").replace(" ^", "^")
        texts.append(inner.strip())
    return sorted(texts)


Build = Callable[[Any], Any]


def _points(module: Any) -> Any:
    return module.ggplot(DATA, module.aes("x", "y")) + module.geom_point()


TICK_CASES: dict[str, Build] = {
    "minor": lambda m: _points(m) + m.theme(axis_minor_ticks=m.element_line()),
    "styled": lambda m: (
        _points(m)
        + m.theme(axis_minor_ticks=m.element_line(color="red", size=2), axis_minor_ticks_length=3)
    ),
    "x only": lambda m: _points(m) + m.theme(axis_minor_ticks_x=m.element_line(color="red")),
    "not from axis_ticks": lambda m: (
        _points(m)
        + m.theme(axis_ticks=m.element_line(color="red"), axis_minor_ticks=m.element_line())
    ),
    "from axis": lambda m: (
        _points(m) + m.theme(axis=m.element_line(color="green"), axis_minor_ticks=m.element_line())
    ),
    "theme_grey": lambda m: (
        _points(m) + m.theme_grey() + m.theme(axis_minor_ticks=m.element_line())
    ),
    "theme_light": lambda m: (
        _points(m) + m.theme_light() + m.theme(axis_minor_ticks=m.element_line())
    ),
    "flavor": lambda m: (
        _points(m) + m.flavor_darcula() + m.theme(axis_minor_ticks=m.element_line())
    ),
    "coord_flip": lambda m: (
        _points(m)
        + m.coord_flip()
        + m.theme(axis_minor_ticks_x=m.element_line(color="red"), axis_minor_ticks_length_x=3)
    ),
    "discrete": lambda m: (
        m.ggplot({"x": ["a", "b", "c"], "y": [1, 2, 3]}, m.aes("x", "y"))
        + m.geom_point()
        + m.theme(axis_minor_ticks=m.element_line())
    ),
    "log": lambda m: _points(m) + m.scale_y_log10() + m.theme(axis_minor_ticks=m.element_line()),
    "facets": lambda m: (
        _points(m) + m.facet_wrap("x", ncol=3) + m.theme(axis_minor_ticks=m.element_line())
    ),
}


@pytest.mark.parametrize("case", TICK_CASES)
def test_minor_ticks_match_lets_plot(case: str) -> None:
    build = TICK_CASES[case]
    assert _ticks(build(gg).to_svg()) == _ticks(build(lp).to_svg())


TEXT_CASES: dict[str, Build] = {}
for _format in [
    "e",
    "pow",
    "pow_full",
    ("e", -3, 3),
    ("pow", -2, 3),
    ("pow_full", None, 3),
    ("e", -2, None),
    ("e", -2.0, 3.0),
]:
    TEXT_CASES[f"big {_format}"] = lambda m, f=_format: (
        m.ggplot(BIG, m.aes("x", "y")) + m.geom_point(m.aes(color="c")) + m.theme(exponent_format=f)
    )
    TEXT_CASES[f"small {_format}"] = lambda m, f=_format: (
        m.ggplot(SMALL, m.aes("x", "y")) + m.geom_point() + m.theme(exponent_format=f)
    )
TEXT_CASES.update(
    {
        "pattern takes the theme style": lambda m: (
            m.ggplot(SMALL, m.aes("x", "y"))
            + m.geom_point()
            + m.scale_x_continuous(format=".2e")
            + m.theme(exponent_format="e")
        ),
        "template": lambda m: (
            m.ggplot(SMALL, m.aes("x", "y"))
            + m.geom_point()
            + m.scale_x_continuous(format="{~g} m")
            + m.theme(exponent_format=("e", -2, 3))
        ),
        "text labels": lambda m: (
            m.ggplot({"x": [1], "y": [1], "l": [1.5e8]})
            + m.geom_text(m.aes("x", "y", label="l"))
            + m.theme(exponent_format="e")
        ),
        "text labels pow_full": lambda m: (
            m.ggplot({"x": [1], "y": [1], "l": [1e8]})
            + m.geom_text(m.aes("x", "y", label="l"))
            + m.theme(exponent_format="pow_full")
        ),
        "log axis": lambda m: (
            m.ggplot({"x": [1, 1e8], "y": [1, 2]}, m.aes("x", "y"))
            + m.geom_point()
            + m.scale_x_log10()
            + m.theme(exponent_format="e")
        ),
        "colorbar breaks": lambda m: (
            m.ggplot(BIG, m.aes("x", "y"))
            + m.geom_point(m.aes(color="c"))
            + m.scale_color_continuous(breaks=[1e7, 3e7])
            + m.theme(exponent_format="e")
        ),
        "size legend": lambda m: (
            m.ggplot(BIG, m.aes("x", "y"))
            + m.geom_point(m.aes(size="c"))
            + m.theme(exponent_format="e")
        ),
        "fixed axis breaks": lambda m: (
            m.ggplot(BIG, m.aes("x", "y"))
            + m.geom_point()
            + m.scale_x_continuous(breaks=[1e7, 2e7])
            + m.theme(exponent_format="e")
        ),
    }
)


@pytest.mark.parametrize("case", TEXT_CASES)
def test_exponent_format_matches_lets_plot(case: str) -> None:
    build = TEXT_CASES[case]
    assert _texts(build(gg).to_svg()) == _texts(build(lp).to_svg())


def test_long_minor_ticks_take_room_as_in_ggplot2() -> None:
    """Deviation: lets-plot reserves the major tick length only."""

    def panel_height(theme: Any) -> float:
        svg = (_points(gg) + theme).to_svg()
        match = re.search(
            r'<clipPath[^>]*>\s*<rect x="[-\d.]+" y="[-\d.]+" width="[-\d.]+" height="([\d.]+)"',
            svg,
        )
        assert match
        return float(match.group(1))

    short = panel_height(gg.theme(axis_minor_ticks=gg.element_line()))
    long = panel_height(gg.theme(axis_minor_ticks=gg.element_line(), axis_minor_ticks_length=10))
    assert short - long == pytest.approx(6.0)


def test_html_zoom_has_a_minor_tick_template() -> None:
    plot = _points(gg) + gg.theme(axis_minor_ticks=gg.element_line()) + gg.ggtb()
    assert '"minor_tick":"\\u003cpath d=' in plot._repr_html_()


@pytest.mark.parametrize(
    ("value", "message"),
    [
        ("pwo", "unknown style 'pwo'; did you mean 'pow'"),
        (("e", 1.5, 3), "got bound 1.5"),
        (("e", -3, 3, 1), "got 4 items"),
    ],
)
def test_bad_exponent_format_names_the_option(value: object, message: str) -> None:
    plot = _points(gg) + gg.theme(exponent_format=value)
    with pytest.raises(SpecError) as error:
        plot.to_svg()
    assert "theme.exponent_format" in str(error.value)
    assert message in str(error.value)


def test_label_text_is_reported_unsupported() -> None:
    plot = _points(gg) + gg.theme(label_text=gg.element_text(color="red"))
    with pytest.warns(gg.GgrustWarning, match="theme.label_text"):
        plot.to_svg()


RUNTIME = Path(__file__).resolve().parents[1] / "js" / "ggrust.js"


@pytest.mark.skipif(shutil.which("node") is None, reason="needs node")
@pytest.mark.parametrize("exponent_format", ["e", "pow_full", ("e", -2, 3)])
def test_zoomed_labels_keep_the_exponent_format(exponent_format: object) -> None:
    """The HTML runtime's regenerated labels equal the engine's at the same limits."""
    data = {"x": [1, 2, 3, 4], "y": [0.0, 1e7, 2e7, 3e7]}
    plot = (
        gg.ggplot(data, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.theme(exponent_format=exponent_format)
        + gg.ggtb()
    )
    found = re.search(r'class="ggrust-zoom">(.*?)</script>', plot._repr_html_())
    assert found
    vertical = json.loads(found.group(1))["plots"][0]["tiles"][0]["vertical"]
    low, high = 2e6, 2.6e7
    script = (
        "global.window = {}; global.document = {};"
        f"eval(require('fs').readFileSync({json.dumps(str(RUNTIME))}, 'utf8') + '(\"test\")');"
        f"const breaks = window.ggrust.breaks('identity', {low}, {high}, {vertical['count']}, "
        f"{json.dumps(vertical['exponent'])});"
        "console.log(JSON.stringify(breaks));"
    )
    output = subprocess.run(["node", "-e", script], capture_output=True, text=True, check=True)
    runtime = [
        label["text"] + ("^" + label["superscript"] if label["superscript"] else "")
        for label in json.loads(output.stdout)["labels"]
    ]
    static = (plot + gg.coord_cartesian(ylim=[low, high])).to_svg()
    x_labels = {"1", "1.5", "2", "2.5", "3", "3.5", "4", "x", "y"}
    engine = [text for text in _texts(static) if text not in x_labels]
    assert sorted(runtime) == engine
    assert any(("e+" if exponent_format != "pow_full" else "·10^") in label for label in runtime)
