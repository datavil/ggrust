"""Grid captions: ``gggrid(...) + labs(caption=...)`` drawn as lets-plot
``CompositeFigureBuildInfo`` draws them (bottom band, right-justified)."""

import re
import sys
import warnings
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent))

import ggrust as gg
import lets_plot as lp
from test_grid_parity import _ggrust_panel_rects, _lets_plot_panel_rects

DATA = {"x": [1, 2, 3], "y": [3, 1, 2]}


def _grid(module, caption: str, *extra: object) -> object:
    """Two points plots side by side, captioned, at 600 x 300."""
    plot = module.ggplot(DATA, module.aes("x", "y")) + module.geom_point()
    grid = module.gggrid([plot, plot], ncol=2) + module.labs(caption=caption)
    for feature in (module.ggsize(600, 300), *extra):
        grid = grid + feature
    return grid


def _ggrust_text(svg: str, content: str) -> tuple[float, float]:
    """Centre x and baseline y of the ggrust text ``content``."""
    match = re.search(rf'<text x="([-\d.]+)" y="([-\d.]+)"[^>]*>{re.escape(content)}</text>', svg)
    assert match, content
    return float(match.group(1)), float(match.group(2))


def _lets_plot_caption(svg: str) -> tuple[float, float]:
    """Right end x and baseline y of lets-plot's caption."""
    match = re.search(
        r'<g transform="translate\(([-\d.e]+) ([-\d.e]+) \) ">\s*<text[^>]*class="plot-caption"',
        svg,
    )
    assert match
    return float(match.group(1)), float(match.group(2))


@pytest.mark.parametrize("titled", [False, True], ids=["plain", "titled"])
def test_grid_caption_matches_lets_plot(titled: bool) -> None:
    """Caption baseline and the band it takes from the panels within font metrics of
    lets-plot (a single plot's caption band differs by the same 1.5 px)."""

    def svgs(module, caption: str | None) -> str:
        extra = (module.ggtitle("Title"),) if titled else ()
        plot = module.ggplot(DATA, module.aes("x", "y")) + module.geom_point()
        grid = module.gggrid([plot, plot], ncol=2) + module.ggsize(600, 300)
        for feature in extra:
            grid = grid + feature
        return (grid + module.labs(caption=caption) if caption else grid).to_svg()

    ours, theirs = svgs(gg, "Source: everyday data"), svgs(lp, "Source: everyday data")
    _, our_y = _ggrust_text(ours, "Source: everyday data")
    their_right, their_y = _lets_plot_caption(theirs)
    assert our_y == pytest.approx(their_y, abs=1.5)
    assert their_right == pytest.approx(594.0)
    our_band = _ggrust_panel_rects(svgs(gg, None))[0][3] - _ggrust_panel_rects(ours)[0][3]
    their_band = _lets_plot_panel_rects(svgs(lp, None))[0][3] - _lets_plot_panel_rects(theirs)[0][3]
    assert our_band == pytest.approx(their_band, abs=2.0)


def test_grid_caption_is_right_justified_and_not_warned() -> None:
    """The caption is drawn, no longer warned, at the bottom right."""
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        svg = _grid(gg, "cap").to_svg()
    assert not [warning for warning in caught if "caption" in str(warning.message)]
    x, y = _ggrust_text(svg, "cap")
    assert 560.0 < x < 594.0
    assert 280.0 < y < 300.0


def test_grid_caption_follows_theme() -> None:
    """``plot_caption`` styles it, ``element_blank()`` removes it and its band."""
    styled = _grid(gg, "cap", gg.theme(plot_caption=gg.element_text(size=20))).to_svg()
    assert re.search(r'font-size="20"[^>]*>cap</text>', styled)
    blank = _grid(gg, "cap", gg.theme(plot_caption=gg.element_blank())).to_svg()
    assert ">cap</text>" not in blank
    plain = gg.gggrid([gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_point()] * 2, ncol=2)
    assert _ggrust_panel_rects(blank) == _ggrust_panel_rects((plain + gg.ggsize(600, 300)).to_svg())


def test_grid_caption_with_tags() -> None:
    """Tags still number the cells of a captioned grid."""
    plot = gg.ggplot(DATA, gg.aes("x", "y")) + gg.geom_point()
    svg = (gg.gggrid([plot, plot], tag_levels="a") + gg.labs(caption="cap")).to_svg()
    assert ">a</text>" in svg and ">b</text>" in svg and ">cap</text>" in svg
