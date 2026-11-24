"""Transparent backgrounds: ``flavor_transparent()`` and ``ggsave(bg=)``.

Both recolor the backgrounds the theme fills with paper (white) and keep the ones with
a color of their own (``theme_grey()``'s grey panel); ``bg="transparent"`` and
``flavor_transparent()`` give the same picture.
"""

from collections.abc import Callable
from pathlib import Path

import pytest
from ggrust import (
    aes,
    element_rect,
    facet_wrap,
    flavor_darcula,
    flavor_transparent,
    geom_point,
    gggrid,
    ggplot,
    ggsave,
    ggsize,
    inset_element,
    theme,
    theme_bw,
    theme_classic,
    theme_grey,
    theme_light,
    theme_minimal2,
)
from ggrust.errors import ExportError, SpecError
from ggrust.plot.core import FeatureSpec, PlotSpec
from PIL import Image

FRAME = {"x": [1, 2, 3, 4], "y": [1, 3, 2, 4], "group": ["a", "a", "b", "b"]}
CORNER = (2, 2)
TRANSPARENT = (0, 0, 0, 0)


def _plot() -> PlotSpec:
    """A faceted plot with a legend, so every background is drawn."""
    return (
        ggplot(FRAME, aes("x", "y", color="group"))
        + geom_point(size=4)
        + facet_wrap("group")
        + ggsize(400, 300)
    )


def _pixel(path: Path, xy: tuple[int, int] = CORNER) -> tuple[int, ...]:
    """The RGBA pixel of a saved image at ``xy``."""
    with Image.open(path) as image:
        return image.convert("RGBA").getpixel(xy)  # ty: ignore[invalid-return-type]


def _panel_centre(path: Path, left: float = 0.0, width: float = 1.0) -> tuple[int, ...]:
    """A pixel inside the first panel of the plot spanning ``left`` to ``left + width``.

    The fractions are of the image width, so the pixel scale does not matter.
    """
    with Image.open(path) as image:
        x = int(image.width * (left + 0.2 * width))
        y = int(image.height * 0.3)
        return image.convert("RGBA").getpixel((x, y))  # ty: ignore[invalid-return-type]


def test_flavor_transparent_clears_white_and_keeps_theme_colors(tmp_path: Path) -> None:
    out = tmp_path / "plot.png"
    ggsave(_plot() + theme_bw() + flavor_transparent(), out)
    assert _pixel(out) == TRANSPARENT
    assert _panel_centre(out) == TRANSPARENT
    # theme_grey's grey panel is a color of its own: it stays.
    ggsave(_plot() + theme_grey() + flavor_transparent(), out)
    assert _pixel(out) == TRANSPARENT
    assert _panel_centre(out) == (235, 235, 235, 255)


def test_flavor_transparent_survives_a_later_named_theme() -> None:
    plot = _plot() + flavor_transparent() + theme_bw()
    assert plot.as_dict()["theme"] == {"name": "bw", "background_fill": "transparent"}


def test_flavor_transparent_combines_with_a_color_flavor(tmp_path: Path) -> None:
    plot = _plot() + flavor_darcula() + flavor_transparent()
    assert plot.as_dict()["theme"] == {"flavor": "darcula", "background_fill": "transparent"}
    out = tmp_path / "plot.png"
    ggsave(plot, out)
    assert _pixel(out) == TRANSPARENT


def test_theme_options_win_over_flavor_transparent(tmp_path: Path) -> None:
    ivory = theme(panel_background=element_rect(fill="#FFFFF0"))
    for plot in (_plot() + flavor_transparent() + ivory, _plot() + ivory + flavor_transparent()):
        out = tmp_path / "plot.png"
        ggsave(plot, out)
        assert _pixel(out) == TRANSPARENT
        assert _panel_centre(out) == (255, 255, 240, 255)


def test_flavor_transparent_on_a_grid_reaches_figures_with_own_themes(tmp_path: Path) -> None:
    grid = gggrid([_plot() + theme_bw(), _plot() + theme_light()]) + flavor_transparent()
    out = tmp_path / "grid.png"
    ggsave(grid + ggsize(800, 300), out)
    assert _pixel(out) == TRANSPARENT
    assert _panel_centre(out, 0.0, 0.5) == TRANSPARENT
    assert _panel_centre(out, 0.5, 0.5) == TRANSPARENT


@pytest.mark.parametrize("theme_function", [theme_minimal2, theme_grey, theme_bw, theme_classic])
def test_ggsave_bg_transparent_matches_flavor_transparent(
    tmp_path: Path, theme_function: Callable[[], FeatureSpec]
) -> None:
    plot = _plot() + theme_function()
    with_bg, with_flavor = tmp_path / "bg.png", tmp_path / "flavor.png"
    ggsave(plot, with_bg, bg="transparent")
    ggsave(plot + flavor_transparent(), with_flavor)
    with Image.open(with_bg) as first, Image.open(with_flavor) as second:
        assert first.convert("RGBA").tobytes() == second.convert("RGBA").tobytes()


def test_ggsave_bg_color_recolors_white_and_keeps_theme_colors(tmp_path: Path) -> None:
    out = tmp_path / "plot.png"
    ggsave(_plot() + theme_grey(), out, bg="navy")
    assert _pixel(out) == (0, 0, 128, 255)
    assert _panel_centre(out) == (235, 235, 235, 255)


def test_ggsave_bg_leaves_the_plot_unchanged(tmp_path: Path) -> None:
    plot = _plot()
    before = plot.as_dict()
    ggsave(plot, tmp_path / "plot.svg", bg="transparent")
    assert plot.as_dict() == before


def test_ggsave_bg_rejects_a_bad_color(tmp_path: Path) -> None:
    with pytest.raises(ExportError, match=r"'bg' 'notacolor' is not a color"):
        ggsave(_plot(), tmp_path / "plot.png", bg="notacolor")


def test_ggsave_bg_rejects_an_empty_string(tmp_path: Path) -> None:
    with pytest.raises(ExportError, match=r"'bg' is empty"):
        ggsave(_plot(), tmp_path / "plot.png", bg=" ")


@pytest.mark.parametrize("use_bg", [False, True])
def test_insets_of_a_plot_follow_the_transparent_background(tmp_path: Path, use_bg: bool) -> None:
    inner = ggplot({"x": [1, 2], "y": [1, 2]}, aes("x", "y")) + geom_point() + theme_bw()
    host = (
        ggplot({"x": [1, 2], "y": [1, 2]}, aes("x", "y"))
        + geom_point()
        + theme_bw()
        + ggsize(400, 300)
        + inset_element(inner, 0.4, 0.4, 1.0, 1.0, align_to="full")
    )
    out = tmp_path / "inset.png"
    if use_bg:
        ggsave(host, out, bg="transparent")
    else:
        ggsave(host + flavor_transparent(), out)
    with Image.open(out) as image:
        rgba = image.convert("RGBA")
        # Inside the inset: its panel, and its plot margin.
        for x, y in [(0.72, 0.30), (0.45, 0.05)]:
            assert rgba.getpixel((int(rgba.width * x), int(rgba.height * y)))[3] == 0


def test_ggsave_bg_rejects_a_non_string(tmp_path: Path) -> None:
    with pytest.raises(ExportError, match=r"'bg' must be a color string.*got int"):
        ggsave(_plot(), tmp_path / "plot.png", bg=3)  # ty: ignore[invalid-argument-type]


def test_ggsave_bg_does_not_apply_to_plot_files(tmp_path: Path) -> None:
    with pytest.raises(ExportError, match=r"'bg' applies to rendered files, not \.ggrs"):
        ggsave(_plot(), tmp_path / "plot.ggrs", bg="white")


def test_other_plot_errors_pass_through_with_bg(tmp_path: Path) -> None:
    bad = _plot() + theme(panel_background=element_rect(fill="notacolor"))
    with pytest.raises(SpecError, match=r"theme\.panel_background"):
        ggsave(bad, tmp_path / "plot.png", bg="white")
