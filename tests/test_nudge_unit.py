"""`nudge_unit` on text-like geoms and `alpha_stroke` on labels, as lets-plot.

`nudge_unit="px"` adds the nudge in pixels after the coord, `"size"` in point
diameters (2.2 px); a positive `nudge_y` moves up, also under `coord_flip`
(lets-plot `NudgePos` adds the offset in client space). Checked against lets-plot
SVG text positions, and against the unnudged positions for every geom.
"""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

DATA = {"x": [1.0, 2.0, 3.0], "y": [1.0, 3.0, 2.0], "t": ["a", "b", "c"]}
POINT_UNIT_SIZE = 2.2


def ggrust_texts(svg: str) -> list[tuple[float, float]]:
    return [(float(x), float(y)) for x, y in re.findall(r'<text x="([-\d.]+)" y="([-\d.]+)"', svg)]


def lets_plot_texts(svg: str) -> list[tuple[float, float]]:
    """Text anchors of a lets-plot SVG: the plot offset plus each text's translate."""
    offset = re.search(r'<g transform="translate\(([-\d.]+) ([-\d.]+) \) ">', svg)
    assert offset is not None
    origin = (float(offset.group(1)), float(offset.group(2)))
    anchors = re.findall(r'<g transform="translate\(([-\d.]+) ([-\d.]+) \) ">\s*<text', svg)
    return [(origin[0] + float(x), origin[1] + float(y)) for x, y in anchors]


def plot(module, geom: str, flip: bool = False, **options: object):
    spec = (
        module.ggplot(DATA, module.aes("x", "y", label="t"))
        + getattr(module, geom)(**options)
        + module.theme_void()
        + module.ggsize(240, 200)
    )
    return spec + module.coord_flip() if flip else spec


@pytest.mark.parametrize("flip", [False, True])
@pytest.mark.parametrize(("unit", "scale"), [("px", 1.0), ("size", POINT_UNIT_SIZE)])
@pytest.mark.parametrize("geom", ["geom_text", "geom_label"])
def test_pixel_units_move_right_and_up(geom: str, unit: str, scale: float, flip: bool) -> None:
    base = ggrust_texts(plot(gg, geom, flip).to_svg())
    nudged = ggrust_texts(plot(gg, geom, flip, nudge_x=5, nudge_y=4, nudge_unit=unit).to_svg())
    assert len(base) == len(nudged) == 3
    for (x0, y0), (x1, y1) in zip(base, nudged, strict=True):
        assert x1 - x0 == pytest.approx(5 * scale, abs=0.02)
        assert y1 - y0 == pytest.approx(-4 * scale, abs=0.02)


@pytest.mark.parametrize("flip", [False, True])
@pytest.mark.parametrize("unit", ["px", "size", "identity"])
def test_text_positions_match_lets_plot(unit: str, flip: bool) -> None:
    nudge = (
        {"nudge_x": 0.2, "nudge_y": -0.3} if unit == "identity" else {"nudge_x": 6, "nudge_y": 3}
    )
    ours = ggrust_texts(plot(gg, "geom_text", flip, nudge_unit=unit, **nudge).to_svg())
    theirs = lets_plot_texts(plot(lp, "geom_text", flip, nudge_unit=unit, **nudge).to_svg())
    assert len(ours) == len(theirs) == 3
    for (x0, y0), (x1, y1) in zip(ours, theirs, strict=True):
        assert x0 == pytest.approx(x1, abs=0.1)
        # lets-plot puts the text in a group at the anchor; ggrust's y is the
        # middle-aligned baseline, a constant few hundredths away.
        assert y0 == pytest.approx(y1, abs=0.1)


@pytest.mark.parametrize("geom", ["geom_text_repel", "geom_label_repel"])
def test_repel_starts_at_the_pixel_nudge(geom: str) -> None:
    options = {"seed": 1, "max_iter": 0}
    base = ggrust_texts(plot(gg, geom, **options).to_svg())
    nudged = ggrust_texts(plot(gg, geom, nudge_y=40, nudge_unit="px", **options).to_svg())
    assert len(base) == len(nudged) == 3
    assert all(y1 < y0 - 10 for (_, y0), (_, y1) in zip(base, nudged, strict=True))


def test_bracket_moves_bar_and_label_in_pixels() -> None:
    data = {"xmin": [1.0], "xmax": [2.0], "y": [3.0], "label": ["*"]}

    def svg(**options: object) -> str:
        mapping = gg.aes(xmin="xmin", xmax="xmax", y="y", label="label")
        layer = gg.geom_bracket(mapping, data=data, **options)
        points = gg.geom_point(gg.aes(x=[1, 2], y=[0, 4]))
        spec = gg.ggplot() + layer + points + gg.theme_void()
        return spec.to_svg()

    base, nudged = svg(), svg(nudge_x=3, nudge_y=7, nudge_unit="px")
    assert ggrust_texts(nudged)[0][0] == pytest.approx(ggrust_texts(base)[0][0] + 3, abs=0.02)
    assert ggrust_texts(nudged)[0][1] == pytest.approx(ggrust_texts(base)[0][1] - 7, abs=0.02)
    bar = re.compile(r'<path d="M([-\d.]+) ([-\d.]+)L[^"]*" fill="none"')
    assert float(bar.search(nudged).group(2)) == pytest.approx(  # type: ignore[union-attr]
        float(bar.search(base).group(2)) - 7,  # type: ignore[union-attr]
        abs=0.02,
    )


def test_unknown_unit_is_an_error() -> None:
    with pytest.raises(SpecError, match=r"nudge_unit.*'identity', 'size' or 'px'"):
        plot(gg, "geom_text", nudge_x=1, nudge_unit="pt").to_svg()


def test_no_unsupported_warning() -> None:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        plot(gg, "geom_label", nudge_unit="px", alpha_stroke=True).to_svg()
        plot(gg, "geom_label_repel", nudge_unit="size", alpha_stroke=True).to_svg()


def label_colors(svg: str) -> tuple[list[str], list[str]]:
    texts = re.findall(r'<text [^>]*fill="([^"]+)"(?: fill-opacity="([^"]+)")?', svg)
    # Label boxes only (closed paths), not repel segments.
    strokes = re.findall(
        r'Z" fill="[^"]*"(?: fill-opacity="[^"]*")? stroke="#ff0000"'
        r'(?: stroke-opacity="([^"]+)")?',
        svg,
    )
    return [opacity for _, opacity in texts], strokes


@pytest.mark.parametrize("geom", ["geom_label", "geom_label_repel"])
def test_alpha_stroke_fades_text_and_border(geom: str) -> None:
    def colors(alpha_stroke: bool | None) -> tuple[list[str], list[str]]:
        options = {"color": "red", "alpha": 0.4, "alpha_stroke": alpha_stroke}
        if geom == "geom_label_repel":
            options["seed"] = 1
        return label_colors(plot(gg, geom, **options).to_svg())

    plain_texts, plain_borders = colors(None)
    faded_texts, faded_borders = colors(True)
    assert len(plain_borders) == len(faded_borders) == 3
    assert set(plain_texts) == {""} and set(plain_borders) == {""}
    assert set(faded_texts) == {"0.4"} and set(faded_borders) == {"0.4"}


@pytest.mark.parametrize("geom", ["geom_text", "geom_label"])
def test_zoom_anchor_is_the_point_without_the_pixel_nudge(geom: str) -> None:
    """Interactive zoom moves a text with its anchor, so a pixel nudge stays in pixels."""

    def anchors(**options: object) -> list[tuple[float, float]]:
        html = (plot(gg, geom, **options) + gg.ggtb())._repr_html_()
        found = re.findall(r'<g data-anchor="([-\d.]+) ([-\d.]+)"', html)
        return [(float(x), float(y)) for x, y in found]

    base = anchors()
    assert len(base) == 3
    assert anchors(nudge_x=9, nudge_y=9, nudge_unit="px") == base
    moved = anchors(nudge_x=0.5, nudge_unit="identity")
    assert all(x1 > x0 for (x0, _), (x1, _) in zip(base, moved, strict=True))
