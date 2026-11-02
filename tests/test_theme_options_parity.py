"""Score-4 theme options (plan step lp4.8) against lets-plot.

Layout is compared through how much an option moves the panel (its size and
offsets relative to the same plot without the option), so font differences of a
few px between the two libraries cancel out.
"""

import re
import warnings

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError  # noqa: E402

DATA = {"x": [1.0, 2.0, 3.0], "y": [1.0, 4.0, 2.0], "g": ["a", "b", "a"]}
NUMBER = r"(-?[\d.]+)"


def _ggrust_panel(svg: str) -> tuple[float, float, float, float]:
    match = re.search(
        rf'<clipPath[^>]*>\s*<rect x="{NUMBER}" y="{NUMBER}" width="{NUMBER}" height="{NUMBER}"',
        svg,
    )
    assert match
    return tuple(float(value) for value in match.groups())


def _lets_plot_panel(svg: str) -> tuple[float, float, float, float]:
    """The panel of a lets-plot SVG: its clip rect, translated by its group."""
    clip = re.search(rf'<rect x="{NUMBER}" y="0\.0" width="{NUMBER}" height="{NUMBER}">', svg)
    assert clip
    width, height = float(clip.group(2)), float(clip.group(3))
    before = svg[: clip.start()]
    left, top = 0.0, 0.0
    for dx, dy in re.findall(rf'transform="translate\({NUMBER}\s*,?\s*{NUMBER}\)"', before)[-2:]:
        left, top = left + float(dx), top + float(dy)
    return (left, top, width, height)


def _panel(
    module: object, theme: dict[str, object], color: str | None = None
) -> tuple[float, float, float, float]:
    mapping = module.aes("x", "y", color=color) if color else module.aes("x", "y")
    plot = module.ggplot(DATA, mapping) + module.geom_point()
    if theme:
        plot += module.theme(**theme)
    svg = plot.to_svg()
    return _ggrust_panel(svg) if module is gg else _lets_plot_panel(svg)


def _change(
    module: object, theme: dict[str, object], color: str | None = None
) -> tuple[float, float]:
    """How much `theme` shrinks the panel: (width, height)."""
    base = {key: value for key, value in theme.items() if key == "legend_position"}
    base, changed = _panel(module, base, color), _panel(module, theme, color)
    return (base[2] - changed[2], base[3] - changed[3])


SIZES = {
    "ticks_x": {"axis_ticks_length_x": 12},
    "ticks_y": {"axis_ticks_length_y": 12, "axis_ticks_y": {"blank": False}},
    "ticks_both": {
        "axis_ticks_length": 9,
        "axis_ticks_length_y": 1,
        "axis_ticks_y": {"blank": False},
    },
    "spacing_x": {"axis_text_spacing_x": 10},
    "spacing_y": {"axis_text_spacing_y": 10},
}


@pytest.mark.parametrize("theme", SIZES.values(), ids=SIZES.keys())
def test_panel_size_changes_match_lets_plot(theme: dict[str, object]) -> None:
    assert _change(gg, theme) == pytest.approx(_change(lp, theme), abs=0.5)


KEYS = {
    "key_width": ({"legend_key_width": 40}, "g"),
    "key_height_bottom": ({"legend_key_height": 40, "legend_position": "bottom"}, "g"),
    "key_size_then_width": ({"legend_key_size": 30, "legend_key_width": 12}, "g"),
    "colorbar_width": ({"legend_key_width": 40}, "y"),
    "colorbar_height_bottom": ({"legend_key_height": 40, "legend_position": "bottom"}, "y"),
}


@pytest.mark.parametrize(("theme", "color"), KEYS.values(), ids=KEYS.keys())
def test_legend_key_sizes_match_lets_plot(theme: dict[str, object], color: str) -> None:
    assert _change(gg, theme, color) == pytest.approx(_change(lp, theme, color), abs=0.5)
    assert _change(gg, theme, color) != (0.0, 0.0)


def _labelled_change(module: object, labels: dict[str, str], theme: dict[str, object]) -> tuple:
    """How much labels plus a theme shrink the panel: (width, height)."""

    def panel(with_labels: bool) -> tuple[float, float, float, float]:
        plot = module.ggplot(DATA, module.aes("x", "y")) + module.geom_point()
        if with_labels:
            plot += module.labs(**labels) + module.theme(**theme)
        svg = plot.to_svg()
        return _ggrust_panel(svg) if module is gg else _lets_plot_panel(svg)

    base, changed = panel(False), panel(True)
    return (base[2] - changed[2], base[3] - changed[3])


HEADINGS = {
    "caption": ({"caption": "Source"}, {}),
    "caption_big": ({"caption": "Source"}, {"plot_caption": {"size": 20}}),
    "title": ({"title": "Title"}, {}),
    "caption_blank": ({"caption": "Source"}, {"plot_caption": "blank"}),
}
MARGINS = {
    "caption": ({"caption": "Source"}, "plot_caption", [10, 0, 6, 0]),
    "title": ({"title": "Title"}, "plot_title", [30, 0, 5, 0]),
    "subtitle": ({"title": "Title", "subtitle": "Sub"}, "plot_subtitle", [12, 0, 3, 0]),
    "title_list_of_two": ({"title": "Title"}, "plot_title", [8, 0]),
    "axis_title_x": ({"x": "x title"}, "axis_title_x", [10, 7, 6, 3]),
    "axis_title_both": ({"x": "x title"}, "axis_title", [4, 0, 9, 0]),
}
AXIS_MARGINS = {
    "y": ("axis_title_y", [3, 8, 6, 5], 0),
    "y_from_axis_title": ("axis_title", [3, 8, 6, 5], 0),
    "x": ("axis_title_x", [10, 7, 6, 3], 1),
}


@pytest.mark.parametrize(("key", "margin", "index"), AXIS_MARGINS.values(), ids=AXIS_MARGINS.keys())
def test_axis_title_margins_add_what_lets_plot_adds(
    key: str, margin: list[float], index: int
) -> None:
    def added(module: object) -> float:
        spaced = _change(module, {key: module.element_text(margin=margin)})
        return spaced[index]

    assert added(gg) == pytest.approx(added(lp), abs=0.01)
    assert added(gg) > 0


def _resolved(module: object, theme: dict[str, object]) -> dict[str, object]:
    return {
        key: module.element_blank() if value == "blank" else module.element_text(**value)
        for key, value in theme.items()
    }


@pytest.mark.parametrize(("labels", "theme"), HEADINGS.values(), ids=HEADINGS.keys())
def test_titles_and_captions_take_lets_plot_space(
    labels: dict[str, str], theme: dict[str, object]
) -> None:
    got = _labelled_change(gg, labels, _resolved(gg, theme))
    want = _labelled_change(lp, labels, _resolved(lp, theme))
    # lets-plot's band is the font size; ggrust's the font's line box (deviations.md).
    assert got == pytest.approx(want, abs=2.5)


@pytest.mark.parametrize(("labels", "key", "margin"), MARGINS.values(), ids=MARGINS.keys())
def test_heading_margins_add_what_lets_plot_adds(
    labels: dict[str, str], key: str, margin: list[float]
) -> None:
    def added(module: object) -> float:
        plain = _labelled_change(module, labels, {})
        spaced = _labelled_change(module, labels, {key: module.element_text(margin=margin)})
        return spaced[1] - plain[1]

    assert added(gg) == pytest.approx(added(lp), abs=0.01)
    assert added(gg) > 0


def test_flipped_axis_title_uses_its_own_side_margins() -> None:
    """coord_flip moves axis_title_x left; lets-plot uses its left/right margins there."""

    def width_change(module: object) -> float:
        def panel(theme: dict[str, object]) -> float:
            plot = module.ggplot(DATA, module.aes("x", "y")) + module.geom_point()
            plot += module.coord_flip() + module.theme(**theme)
            svg = plot.to_svg()
            return (_ggrust_panel(svg) if module is gg else _lets_plot_panel(svg))[2]

        margin = module.element_text(margin=[20, 6, 20, 9])
        return panel({}) - panel({"axis_title_x": margin})

    assert width_change(gg) == pytest.approx(width_change(lp), abs=0.01)
    assert width_change(gg) == pytest.approx(15.0, abs=0.01)


@pytest.mark.parametrize(
    ("linetype", "dasharray"),
    [("dashed", "8 8"), ("dotted", "2 6"), ([5, 3], "10 6"), ("solid", None)],
)
def test_element_line_linetype_dashes_scale_with_width(
    linetype: str | list[int], dasharray: str | None
) -> None:
    """A theme line's dash lengths are in line widths, as lets-plot draws them."""
    plot = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_point()
    plot += gg.theme(panel_grid_major=gg.element_line(linetype=linetype, size=2))
    found = set(re.findall(r'stroke-dasharray="([^"]*)"', plot.to_svg()))
    assert found == ({dasharray} if dasharray else set())


def test_element_rect_linetype_reaches_the_panel_border() -> None:
    plot = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_point() + gg.theme_bw()
    plot += gg.theme(panel_border=gg.element_rect(linetype="dashed"))
    assert "stroke-dasharray" in plot.to_svg()


@pytest.mark.parametrize(
    ("linetype", "message"),
    [("dashedd", 'invalid line type "dashedd"'), ([1] * 9, "at most 8 numbers, got 9")],
)
def test_bad_element_linetype_names_the_option(linetype: object, message: str) -> None:
    plot = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_point()
    plot += gg.theme(panel_border=gg.element_rect(linetype=linetype))
    with pytest.raises(SpecError) as error:
        plot.to_svg()
    assert "theme.panel_border" in str(error.value)
    assert message in str(error.value)
