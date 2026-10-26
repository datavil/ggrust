"""ColorBrewer scales must color every point like lets-plot (all palettes it ships)."""

import warnings

import pytest
from test_colorbar_parity import _ours, _theirs
from test_style_parity import circles

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

PALETTES = {
    "seq": [
        "Blues",
        "BuGn",
        "BuPu",
        "GnBu",
        "Greens",
        "Greys",
        "Oranges",
        "OrRd",
        "PuBu",
        "PuBuGn",
        "PuRd",
        "Purples",
        "RdPu",
        "Reds",
        "YlGn",
        "YlGnBu",
        "YlOrBr",
        "YlOrRd",
    ],
    "div": ["BrBG", "PiYG", "PRGn", "PuOr", "RdBu", "RdGy", "RdYlBu", "RdYlGn", "Spectral"],
    "qual": ["Accent", "Dark2", "Paired", "Pastel1", "Pastel2", "Set1", "Set2", "Set3"],
}
N = 30
FRAME = {
    "x": [float(index) for index in range(N)],
    "y": [float(index % 7) for index in range(N)],
    "c4": [f"g{index % 4}" for index in range(N)],
    "c15": [f"g{index % 15:02d}" for index in range(N)],
    "v": [float((index * 7) % 23) for index in range(N)],
}


def fills(module, scale, column) -> list[str]:
    plot = module.ggplot(FRAME, module.aes("x", "y", color=column)) + module.geom_point() + scale
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        return [fill for fill, _, _ in circles(plot.to_svg())[:N]]


@pytest.mark.parametrize("palette", [name for names in PALETTES.values() for name in names])
@pytest.mark.parametrize("column", ["c4", "c15", "v"])
def test_every_palette_matches_lets_plot(palette, column):
    theirs = fills(lp, lp.scale_color_brewer(palette=palette), column)
    ours = fills(gg, gg.scale_color_brewer(palette=palette), column)
    assert ours == theirs


OPTIONS = {
    "default_discrete": dict(),
    "type_only": dict(type="div"),
    "index": dict(type="seq", palette=3),
    "index_wraps": dict(type="qual", palette=11),
    "reversed": dict(palette="Spectral", direction=-1),
    "overflow_cycle": dict(palette="Set2", overflow="cycle"),
    "overflow_interpolate": dict(palette="Dark2", overflow="i"),
    "overflow_generate": dict(palette="Blues", overflow="generate"),
    "name_wins_over_type": dict(type="seq", palette="Set3"),
    "type_ignored_with_a_name": dict(type="sequential", palette="Blues"),
    "huge_index": dict(type="div", palette=2**31),
    "na_and_limits": dict(palette="Set2", limits=["g00", "g01"], na_value="black"),
}


@pytest.mark.parametrize("name", OPTIONS)
@pytest.mark.parametrize("column", ["c15", "v"])
def test_options_match_lets_plot(name, column):
    if name == "na_and_limits" and column == "v":
        pytest.skip("category limits on continuous data are an error in ggrust")
    theirs = fills(lp, lp.scale_color_brewer(**OPTIONS[name]), column)
    ours = fills(gg, gg.scale_color_brewer(**OPTIONS[name]), column)
    assert ours == theirs


def test_continuous_brewer_colorbar_matches_lets_plot():
    def build(module):
        return (
            module.ggplot(FRAME, module.aes("x", "y", color="v"))
            + module.geom_point()
            + module.scale_color_brewer(palette="YlGnBu", direction=-1)
        )

    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        theirs, ours = _theirs(build(lp).to_svg()), _ours(build(gg).to_svg())
    assert [(title, segments, labels) for title, segments, _, labels in ours] == [
        (title, segments, labels) for title, segments, _, labels in theirs
    ]
