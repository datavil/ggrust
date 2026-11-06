"""Side band axis theme elements: R ggside's ``ggside.axis.*`` (step 9.8).

``ggside_axis_*_y`` styles only the x side band's own (y) axis, ``_x`` only the y
side band's own (x) axis, side suffixes pick the axis on that screen side, the
most specific element wins and anything unset falls back to the main ``axis_*``
elements. Checked on the SVG: the ``side-axis`` groups, the main axes, the room
the band labels take.
"""

from __future__ import annotations

import re
import warnings

import ggrust as gg
import polars as pl
import pytest

CELLS = ["c1", "c2", "c3", "c4"]
HEATMAP = pl.DataFrame(
    {
        "cell": [cell for cell in CELLS for _ in range(3)],
        "gene": ["g1", "g2", "g3"] * 4,
        "expr": [float(index % 5) for index in range(12)],
        "cell_type": [kind for kind in ["T", "T", "B", "NK"] for _ in range(3)],
        "batch": [batch for batch in ["b1", "b2", "b1", "b2"] for _ in range(3)],
        "module": ["M1", "M2", "M1"] * 4,
    }
)
X_TRACKS = ("cell_type", "batch")
Y_TRACKS = ("module",)
MAIN_LABELS = ("c1", "c2", "c3", "c4", "g1", "g2", "g3")


def _heatmap() -> gg.PlotSpec:
    """Two x side tracks (on top, named on their left axis), one y side track
    (on the right, named on its bottom axis)."""
    return (
        gg.ggplot(HEATMAP, gg.aes("cell", "gene", fill="expr"))
        + gg.geom_tile()
        + gg.new_scale_fill()
        + gg.geom_xsidetile(gg.aes(fill="cell_type"))
        + gg.new_scale_fill()
        + gg.geom_xsidetile(gg.aes(fill="batch"))
        + gg.new_scale_fill()
        + gg.geom_ysidetile(gg.aes(fill="module"))
        + gg.ggsize(600, 400)
    )


def _render(plot: gg.PlotSpec) -> str:
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        return plot.to_svg()


def _side_axes(svg: str) -> list[str]:
    """The ``side-axis`` groups: the y band's (x) axis first, then the x band's."""
    return re.findall(r'<g class="side-axis">(.*?)</g>', svg, re.S)


def _band_axis(svg: str, horizontal: bool) -> str:
    """The side axis group with a horizontal (y band) or vertical (x band) line or
    labels, found by its tracks' names."""
    names = Y_TRACKS if horizontal else X_TRACKS
    groups = [group for group in _side_axes(svg) if any(f">{n}<" in group for n in names)]
    if groups:
        return groups[0]
    # No labels left: tell the groups apart by their paths' direction.
    for group in _side_axes(svg):
        for _x1, y1, _x2, y2 in re.findall(r'd="M([\d.]+) ([\d.]+)L([\d.]+) ([\d.]+)"', group):
            is_vertical_axis_tick = y1 == y2  # ticks of a vertical axis are horizontal
            if is_vertical_axis_tick != horizontal:
                return group
    return ""


def _texts(svg: str) -> list[str]:
    """Every text of the plot."""
    return re.findall(r">([^<>]+)</text>", svg)


def _axis_texts(svg: str) -> list[str]:
    """The texts of the band axes (track names; legends repeat them as titles)."""
    return [text for group in _side_axes(svg) for text in _texts(group)]


def _strokes(group: str) -> list[str]:
    return re.findall(r'stroke="(#[0-9a-f]+)"', group)


def _outside_side_axes(svg: str) -> str:
    return re.sub(r'<g class="side-axis">.*?</g>', "", svg, flags=re.S)


def _panel_x(svg: str) -> float:
    found = re.search(r'<g class="panel" clip-path="url\(#([^)]+)\)"', svg)
    assert found
    rect = re.search(rf'<clipPath id="{found.group(1)}"><rect x="([-\d.]+)"', svg)
    assert rect
    return float(rect.group(1))


def test_spec_carries_the_ggside_axis_options() -> None:
    spec = gg.theme(
        ggside_axis_text_y=gg.element_blank(),
        ggside_axis_ticks_length_x_bottom=7,
        ggside_axis_line_y_left=gg.element_line(color="red"),
    ).as_dict()
    assert spec["ggside_axis_text_y"] == {"blank": True}
    assert spec["ggside_axis_ticks_length_x_bottom"] == 7
    assert spec["ggside_axis_line_y_left"]["color"] == "red"


def test_unset_elements_leave_the_output_unchanged() -> None:
    plot = _heatmap()
    assert _render(plot + gg.theme()) == _render(plot)
    # Main axis options still reach the band axes.
    svg = _render(plot + gg.theme(axis_text=gg.element_text(color="red")))
    for horizontal in (True, False):
        assert 'fill="#ff0000"' in _band_axis(svg, horizontal)


def test_text_y_hides_only_the_x_band_track_names() -> None:
    plain = _render(_heatmap())
    svg = _render(_heatmap() + gg.theme(ggside_axis_text_y=gg.element_blank()))
    assert _axis_texts(svg) == list(Y_TRACKS)
    for label in MAIN_LABELS:
        assert label in _texts(svg), label
    # The hidden names free their room: the panel starts further left.
    assert _panel_x(svg) < _panel_x(plain) - 20

    # The main axes keep their look (they move with the panel).
    def styles(text: str) -> list[str]:
        return [
            re.sub(r' (x|y|transform)="[^"]*"', "", label)
            for label in re.findall(r"<text[^>]*>[cg]\d</text>", text)
        ]

    assert styles(svg) == styles(plain)


def test_text_x_hides_only_the_y_band_track_names() -> None:
    svg = _render(_heatmap() + gg.theme(ggside_axis_text_x=gg.element_blank()))
    assert sorted(_axis_texts(svg)) == sorted(X_TRACKS)
    for label in MAIN_LABELS:
        assert label in _texts(svg), label


@pytest.mark.parametrize(
    ("key", "hidden", "kept"),
    [
        ("ggside_axis_text_y_left", X_TRACKS, Y_TRACKS),
        ("ggside_axis_text_x_bottom", Y_TRACKS, X_TRACKS),
        ("ggside_axis_text_y_right", (), X_TRACKS + Y_TRACKS),
        ("ggside_axis_text_x_top", (), X_TRACKS + Y_TRACKS),
        ("ggside_axis_text", X_TRACKS + Y_TRACKS, ()),
    ],
)
def test_side_suffixes_pick_the_axis_on_that_side(
    key: str, hidden: tuple[str, ...], kept: tuple[str, ...]
) -> None:
    # The x band's axis is on the left, the y band's at the bottom (where the main
    # perpendicular axes are).
    svg = _render(_heatmap() + gg.theme(**{key: gg.element_blank()}))
    assert sorted(_axis_texts(svg)) == sorted(kept), key
    assert not set(hidden) & set(_axis_texts(svg))
    for name in MAIN_LABELS:
        assert name in _texts(svg), (key, name)


def test_the_most_specific_element_wins() -> None:
    svg = _render(
        _heatmap()
        + gg.theme(
            ggside_axis_text=gg.element_text(size=20),
            ggside_axis_text_y=gg.element_blank(),
            ggside_axis_text_y_left=gg.element_text(color="red"),
        )
    )
    x_band = _band_axis(svg, horizontal=False)
    for name in X_TRACKS:
        assert re.search(rf'font-size="20"[^>]*fill="#ff0000"[^>]*>{name}<', x_band), name
    y_band = _band_axis(svg, horizontal=True)
    assert re.search(r'font-size="20"[^>]*fill="#474747"[^>]*>module<', y_band)
    # The main labels keep the theme's 13 px.
    assert re.search(r'font-size="13"[^>]*>c1<', svg)


def test_ticks_and_lines_style_one_band_each() -> None:
    svg = _render(
        _heatmap()
        + gg.theme(
            ggside_axis_ticks_y=gg.element_line(color="red"),
            ggside_axis_line_y=gg.element_line(color="blue"),
            ggside_axis_ticks_x=gg.element_line(color="green"),
            ggside_axis_ticks_length_x=10,
        )
    )
    x_band, y_band = _band_axis(svg, False), _band_axis(svg, True)
    assert _strokes(x_band).count("#ff0000") == 2  # one tick per track
    assert "#0000ff" in _strokes(x_band)
    assert "#0000ff" not in _strokes(y_band) and "#ff0000" not in _strokes(y_band)
    assert "#008000" in _strokes(y_band)
    tick = re.search(
        r'd="M([\d.]+) ([\d.]+)L([\d.]+) ([\d.]+)" fill="none" stroke="#008000"', y_band
    )
    assert tick
    assert float(tick.group(4)) - float(tick.group(2)) == pytest.approx(10)
    # The main axes keep their own look.
    outside = _outside_side_axes(svg)
    for color in ("#ff0000", "#0000ff", "#008000"):
        assert f'stroke="{color}"' not in outside, color


def test_blank_ticks_and_line_drop_from_one_band_only() -> None:
    svg = _render(
        _heatmap()
        + gg.theme(
            ggside_axis_ticks_x=gg.element_blank(),
            ggside_axis_line_x=gg.element_blank(),
        )
    )
    plain = _render(_heatmap())
    assert "<path" not in _band_axis(svg, horizontal=True)

    # The x band's axis keeps its look (it moves with the freed room).
    def look(group: str) -> str:
        return re.sub(r' (x|y|d)="[^"]*"', "", group)

    assert look(_band_axis(svg, horizontal=False)) == look(_band_axis(plain, horizontal=False))
    assert _outside_side_axes(svg).count("<path") == _outside_side_axes(plain).count("<path")


def test_ggside_axis_options_raise_no_warnings_and_typos_still_warn() -> None:
    _render(_heatmap() + gg.theme(ggside_axis_line_y=gg.element_line(arrow=gg.arrow())))
    _render(_heatmap() + gg.theme(ggside_axis_text_x=gg.element_text(angle=45)))
    with pytest.raises(TypeError, match="Did you mean 'ggside_axis_text'"):
        gg.theme(ggside_axis_txt=gg.element_blank())  # ty: ignore[unknown-argument]


def _module_label_bottom(svg: str) -> float:
    """The lowest point of the y band's vertical "module" label (rotated -90)."""
    found = re.search(
        r'<text x="[\d.]+" y="[\d.]+"[^>]*rotate\(-90 [\d.]+ ([\d.]+)\)">module<', svg
    )
    assert found
    return float(found.group(1))


@pytest.mark.parametrize(
    "options",
    [
        {"axis_text_x": "blank", "ggside_axis_text_x": gg.element_text()},
        {"ggside_axis_ticks_length_x_bottom": 60},
        {
            "axis_ticks": "blank",
            "ggside_axis_ticks_x": gg.element_line(),
            "ggside_axis_ticks_length_x": 30,
        },
    ],
)
def test_band_axis_room_comes_from_the_band_elements(options: dict[str, object]) -> None:
    svg = _render(_heatmap() + gg.theme(**options))  # ty: ignore[invalid-argument-type]
    # The vertical label runs down from its anchor by its width (about 45 px at 13 px).
    assert _module_label_bottom(svg) + 45 <= 400, options
