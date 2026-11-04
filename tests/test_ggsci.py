"""ggsci journal palettes: colors match ggsci's hex values, overflow warns, errors."""

import re
import warnings

import ggrust as gg
import pytest
from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.core import FeatureSpec
from ggrust.plot.scale import _scale
from test_style_parity import circles

# ggsci 5.2.0 `ggsci_db` (R/palettes.R): family -> palette -> colors, default first.
GGSCI = {
    "npg": {
        "nrc": [
            "#E64B35",
            "#4DBBD5",
            "#00A087",
            "#3C5488",
            "#F39B7F",
            "#8491B4",
            "#91D1C2",
            "#DC0000",
            "#7E6148",
            "#B09C85",
        ],
    },
    "aaas": {
        "default": [
            "#3B4992",
            "#EE0000",
            "#008B45",
            "#631879",
            "#008280",
            "#BB0021",
            "#5F559B",
            "#A20056",
            "#808180",
            "#1B1919",
        ],
    },
    "nejm": {
        "default": [
            "#BC3C29",
            "#0072B5",
            "#E18727",
            "#20854E",
            "#7876B1",
            "#6F99AD",
            "#FFDC91",
            "#EE4C97",
        ],
    },
    "lancet": {
        "lanonc": [
            "#00468B",
            "#ED0000",
            "#42B540",
            "#0099B4",
            "#925E9F",
            "#FDAF91",
            "#AD002A",
            "#ADB6B6",
            "#1B1919",
        ],
    },
    "jama": {
        "default": [
            "#374E55",
            "#DF8F44",
            "#00A1D5",
            "#B24745",
            "#79AF97",
            "#6A6599",
            "#80796B",
        ],
    },
    "jco": {
        "default": [
            "#0073C2",
            "#EFC000",
            "#868686",
            "#CD534C",
            "#7AA6DC",
            "#003C67",
            "#8F7700",
            "#3B3B3B",
            "#A73030",
            "#4A6990",
        ],
    },
    "d3": {
        "category10": [
            "#1F77B4",
            "#FF7F0E",
            "#2CA02C",
            "#D62728",
            "#9467BD",
            "#8C564B",
            "#E377C2",
            "#7F7F7F",
            "#BCBD22",
            "#17BECF",
        ],
        "category20": [
            "#1F77B4",
            "#FF7F0E",
            "#2CA02C",
            "#D62728",
            "#9467BD",
            "#8C564B",
            "#E377C2",
            "#7F7F7F",
            "#BCBD22",
            "#17BECF",
            "#AEC7E8",
            "#FFBB78",
            "#98DF8A",
            "#FF9896",
            "#C5B0D5",
            "#C49C94",
            "#F7B6D2",
            "#C7C7C7",
            "#DBDB8D",
            "#9EDAE5",
        ],
        "category20b": [
            "#393B79",
            "#637939",
            "#8C6D31",
            "#843C39",
            "#7B4173",
            "#5254A3",
            "#8CA252",
            "#BD9E39",
            "#AD494A",
            "#A55194",
            "#6B6ECF",
            "#B5CF6B",
            "#E7BA52",
            "#D6616B",
            "#CE6DBD",
            "#9C9EDE",
            "#CEDB9C",
            "#E7CB94",
            "#E7969C",
            "#DE9ED6",
        ],
        "category20c": [
            "#3182BD",
            "#E6550D",
            "#31A354",
            "#756BB1",
            "#636363",
            "#6BAED6",
            "#FD8D3C",
            "#74C476",
            "#9E9AC8",
            "#969696",
            "#9ECAE1",
            "#FDAE6B",
            "#A1D99B",
            "#BCBDDC",
            "#BDBDBD",
            "#C6DBEF",
            "#FDD0A2",
            "#C7E9C0",
            "#DADAEB",
            "#D9D9D9",
        ],
    },
    "igv": {
        "default": [
            "#5050FF",
            "#CE3D32",
            "#749B58",
            "#F0E685",
            "#466983",
            "#BA6338",
            "#5DB1DD",
            "#802268",
            "#6BD76B",
            "#D595A7",
            "#924822",
            "#837B8D",
            "#C75127",
            "#D58F5C",
            "#7A65A5",
            "#E4AF69",
            "#3B1B53",
            "#CDDEB7",
            "#612A79",
            "#AE1F63",
            "#E7C76F",
            "#5A655E",
            "#CC9900",
            "#99CC00",
            "#A9A9A9",
            "#CC9900",
            "#99CC00",
            "#33CC00",
            "#00CC33",
            "#00CC99",
            "#0099CC",
            "#0A47FF",
            "#4775FF",
            "#FFC20A",
            "#FFD147",
            "#990033",
            "#991A00",
            "#996600",
            "#809900",
            "#339900",
            "#00991A",
            "#009966",
            "#008099",
            "#003399",
            "#1A0099",
            "#660099",
            "#990080",
            "#D60047",
            "#FF1463",
            "#00D68F",
            "#14FFB1",
        ],
        "alternating": [
            "#5773CC",
            "#FFB900",
        ],
    },
}


def frame(count: int) -> dict[str, list]:
    """One point per category, in category order."""
    return {
        "x": [float(index) for index in range(count)],
        "y": [float(index % 5) for index in range(count)],
        "group": [f"g{index:02d}" for index in range(count)],
    }


def point_colors(scale: FeatureSpec, count: int, aesthetic: str = "color") -> list[str]:
    """The fill of each point, in data order (upper-case hex)."""
    mapping = gg.aes("x", "y", **{aesthetic: "group"})
    shape = {} if aesthetic == "color" else {"shape": 21}
    plot = gg.ggplot(frame(count), mapping) + gg.geom_point(**shape) + scale
    return [fill.upper() for fill, _, _ in circles(plot.to_svg())[:count]]


CASES = [
    (family, palette, colors)
    for family, palettes in GGSCI.items()
    for palette, colors in palettes.items()
]


@pytest.mark.parametrize(("family", "palette", "colors"), CASES)
@pytest.mark.parametrize("aesthetic", ["color", "fill"])
def test_every_palette_matches_ggsci(family, palette, colors, aesthetic):
    scale = getattr(gg, f"scale_{aesthetic}_{family}")(palette=palette)
    with warnings.catch_warnings():
        warnings.simplefilter("error", gg.GgrustWarning)
        assert point_colors(scale, len(colors), aesthetic) == colors


@pytest.mark.parametrize("family", list(GGSCI))
def test_default_palette_is_ggsci_default(family):
    default = next(iter(GGSCI[family].values()))
    assert point_colors(getattr(gg, f"scale_color_{family}")(), 3) == default[:3]


def test_alpha_rounds_like_r_rgb():
    plot = (
        gg.ggplot(frame(3), gg.aes("x", "y", color="group"))
        + gg.geom_point()
        + gg.scale_color_npg(alpha=0.5)
    )
    found = circles(plot.to_svg())[:3]
    assert [fill.upper() for fill, _, _ in found] == GGSCI["npg"]["nrc"][:3]
    # R `rgb(alpha = 0.5 * 255)` is #..80: 128 / 255.
    assert all(abs(opacity - 128 / 255) < 1e-3 for _, _, opacity in found)


def test_too_short_palette_warns_and_generates():
    with pytest.warns(gg.GgrustWarning) as record:
        colors = point_colors(gg.scale_color_jama(), 9)
    messages = [str(item.message) for item in record]
    assert any(
        re.search(r"at 'scales\[0\]': the 'jama' palette has 7 colors but 'color' has 9", text)
        and "scale_*_igv()" in text
        for text in messages
    ), messages
    assert colors[:7] == GGSCI["jama"]["default"]
    assert len(set(colors)) == 9


def test_overflow_cycle_repeats_the_palette():
    with pytest.warns(gg.GgrustWarning, match="has 2 colors but 'fill' has 5"):
        colors = point_colors(gg.scale_fill_igv(palette="alternating", overflow="cycle"), 5, "fill")
    assert colors == ["#5773CC", "#FFB900"] * 2 + ["#5773CC"]


def test_limits_count_toward_the_palette():
    limits = [f"g{index:02d}" for index in range(12)]
    with pytest.warns(gg.GgrustWarning, match="has 10 colors but 'color' has 12"):
        point_colors(gg.scale_color_npg(limits=limits), 3)


def test_later_ggsci_scale_replaces_brewer():
    colors = point_colors(gg.scale_color_brewer(palette="Set2") + gg.scale_color_d3(), 3)
    assert colors == GGSCI["d3"]["category10"][:3]


@pytest.mark.parametrize(
    ("make", "error", "path", "pattern"),
    [
        (
            lambda: gg.scale_color_d3(palette="category21"),
            SpecError,
            "palette",
            r"d3 palette 'category21' was not found; use 'category10', .*; "
            r"did you mean 'category20'\?$",
        ),
        (lambda: gg.scale_fill_igv(palette=2), UnsupportedValueError, "palette", "got int"),
        (lambda: gg.scale_color_npg(alpha=0), SpecError, "alpha", r"alpha must be in \(0, 1\]"),
        (lambda: gg.scale_fill_npg(alpha=1.5), SpecError, "alpha", r"in \(0, 1\], got 1.5"),
        (lambda: gg.scale_color_nejm(alpha="0.5"), UnsupportedValueError, "alpha", "got str"),
    ],
)
def test_bad_options_fail_at_spec_building(make, error, path, pattern):
    with pytest.raises(error) as info:
        make()
    assert type(info.value) is error
    assert info.value.path == path
    assert re.search(pattern, info.value.reason)


@pytest.mark.parametrize(
    ("options", "path", "pattern"),
    [
        ({"palette": "nrcc"}, "scales[0].palette", r"npg palette 'nrcc' was not found; use 'nrc'"),
        ({"alpha": -1}, "scales[0].alpha", r"alpha must be in \(0, 1\], got -1"),
        ({"overflow": "wrap"}, "scales[0].overflow", r"overflow expected one of"),
    ],
)
def test_engine_checks_raw_specs(options, path, pattern):
    scale = _scale("color", scale_mapper_kind="color_npg", **options)
    plot = gg.ggplot(frame(3), gg.aes("x", "y", color="group")) + gg.geom_point() + scale
    with pytest.raises(SpecError) as info:
        plot.to_svg()
    assert info.value.path == path
    assert re.search(pattern, info.value.reason)


def test_overflow_fails_at_spec_building():
    with pytest.raises(SpecError) as info:
        gg.scale_color_npg(overflow="cycel")
    assert info.value.path == "overflow"
    assert re.search(r"got 'cycel'; did you mean 'cycle'\?$", info.value.reason)
    with pytest.raises(SpecError):
        gg.scale_color_npg(overflow=1)  # ty: ignore[invalid-argument-type]
    assert point_colors(gg.scale_color_npg(overflow="C"), 3) == GGSCI["npg"]["nrc"][:3]


def test_numpy_alpha_is_accepted():
    np = pytest.importorskip("numpy")
    for alpha in (np.float32(0.5), np.float64(0.5), np.int64(1)):
        gg.scale_color_npg(alpha=alpha)
