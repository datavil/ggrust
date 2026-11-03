"""Text halos (``halo_width``, ``halo_color``) must match lets-plot.

lets-plot draws a halo as an extra ``<text>`` under the text: no fill, a stroke
``2 x halo_width`` px wide with round joins, in ``halo_color`` (default: the panel
background, else the plot background) faded by ``alpha``. Only ``geom_text`` and
``geom_text_repel`` have halos.
"""

import io
import re
import warnings
from collections.abc import Callable
from types import ModuleType
from typing import Any

import pytest
from PIL import Image

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

DATA: dict[str, list[Any]] = {"x": [1.0, 2.0], "y": [1.0, 2.0], "l": ["Ab", "C\nd"]}
TEXT = re.compile(r"<text\b([^>]*)>")

Layer = Callable[[ModuleType], Any]


def _plot(module: ModuleType, layer: Layer, theme: Layer | None = None) -> Any:
    plot = module.ggplot(DATA, module.aes("x", "y", label="l")) + layer(module)
    return plot + theme(module) if theme else plot


def _hex(color: str) -> str:
    match = re.fullmatch(r"rgb\((\d+),\s*(\d+),\s*(\d+)\)", color)
    if match:
        return "#" + "".join(f"{int(part):02x}" for part in match.groups())
    return color.lower()


def _halos(svg: str, module: ModuleType) -> list[tuple[str, float, float]]:
    """(stroke color, stroke opacity, stroke width) of each halo text, in order."""
    halos = []
    for tag in TEXT.findall(svg):
        attributes = dict(re.findall(r'([\w-]+)="([^"]*)"', tag))
        if module is lp:
            style = attributes.get("style", "")
            if "fill:none" not in style:
                continue
        elif attributes.get("fill") != "none":
            continue
        halos.append(
            (
                _hex(attributes["stroke"]),
                round(float(attributes.get("stroke-opacity", 1.0)), 2),
                float(attributes["stroke-width"]),
            )
        )
    return halos


def _kinds(svg: str, module: ModuleType) -> list[str]:
    """'halo' or 'text' for each panel text that draws a label of DATA."""
    kinds = []
    for match in re.finditer(r"<text\b([^>]*)>\s*(?:<tspan>)?(Ab|C|d)<", svg):
        tag = match.group(1)
        halo = "fill:none" in tag if module is lp else 'fill="none"' in tag
        kinds.append("halo" if halo else "text")
    return kinds


CASES: dict[str, tuple[Layer, Layer | None]] = {
    "default": (lambda m: m.geom_text(halo_width=3), None),
    "color_alpha": (lambda m: m.geom_text(halo_width=3, halo_color="red", alpha=0.5), None),
    "panel_fill": (
        lambda m: m.geom_text(halo_width=2),
        lambda m: m.theme(panel_background=m.element_rect(fill="yellow")),
    ),
    "plot_fill": (
        lambda m: m.geom_text(halo_width=2),
        lambda m: m.theme(plot_background=m.element_rect(fill="cyan")),
    ),
    "grey": (lambda m: m.geom_text(halo_width=1.5), lambda m: m.theme_grey()),
    "translucent_color": (lambda m: m.geom_text(halo_width=1.5, halo_color="#ff000080"), None),
    "color_only": (lambda m: m.geom_text(halo_color="red"), None),
    "repel": (lambda m: m.geom_text_repel(halo_width=3, halo_color="red", seed=1), None),
}


@pytest.mark.parametrize("name", list(CASES))
def test_halo_paint_matches_lets_plot(name: str) -> None:
    layer, theme = CASES[name]
    ours = _plot(gg, layer, theme).to_svg()
    theirs = _plot(lp, layer, theme).to_svg()
    assert _halos(ours, gg) == _halos(theirs, lp)
    # One halo per line (3), none without a width.
    assert len(_halos(ours, gg)) == (0 if name == "color_only" else 3)


def test_halos_draw_under_each_rows_lines_like_lets_plot() -> None:
    layer: Layer = lambda m: m.geom_text(halo_width=3)  # noqa: E731
    expected = ["halo", "text", "halo", "halo", "text", "text"]
    assert _kinds(_plot(gg, layer).to_svg(), gg) == expected
    assert _kinds(_plot(lp, layer).to_svg(), lp) == expected


def test_label_has_no_halo_and_warns() -> None:
    with pytest.warns(UserWarning, match="halo_width"):
        svg = _plot(gg, lambda m: m.geom_label(halo_width=3)).to_svg()
    assert _halos(svg, gg) == []


def _red_pixels(plot: Any) -> int:
    buffer = io.BytesIO()
    plot.to_png(buffer, scale=1)
    image = Image.open(io.BytesIO(buffer.getvalue())).convert("RGB")
    pixels = image.tobytes()
    return sum(
        1
        for index in range(0, len(pixels), 3)
        if pixels[index] > 200 and pixels[index + 1] < 60 and pixels[index + 2] < 60
    )


def test_png_draws_the_halo() -> None:
    without = _red_pixels(_plot(gg, lambda m: m.geom_text(size=12)))
    with_halo = _red_pixels(
        _plot(gg, lambda m: m.geom_text(size=12, halo_width=3, halo_color="red"))
    )
    assert without == 0
    assert with_halo > 200


def test_html_carries_the_halo() -> None:
    html = _plot(gg, lambda m: m.geom_text(halo_width=3, halo_color="red")).to_html()
    assert html is not None
    assert html.count('fill="none" stroke="#ff0000" stroke-width="6"') == 3
