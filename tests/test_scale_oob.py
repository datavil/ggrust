"""``oob=`` on continuous color and fill scales (ggplot2 ``oob``; not in lets-plot).

``"censor"`` (the default, lets-plot's only behavior) draws values outside ``limits``
as ``na_value``; ``"squish"`` clamps them to the nearest limit so they take the end
colors. Missing values stay ``na_value``. ``oob`` merges like ``limits``.
"""

import io
import re
from collections.abc import Callable

import ggrust as gg
import numpy as np
import pytest
from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.core import FeatureSpec, PlotSpec
from PIL import Image

VALUES = [-5.0, -2.5, 0.0, 2.5, 5.0, None]
FRAME = {"x": list(range(len(VALUES))), "y": [0] * len(VALUES), "v": VALUES}
LIMITS = [-2.5, 2.5]
NA_GREY = "#808080"
BLACK, WHITE = "#000000", "#ffffff"


def _tiles(*scales: FeatureSpec) -> PlotSpec:
    plot = gg.ggplot(FRAME, gg.aes("x", "y", fill="v")) + gg.geom_tile()
    for scale in scales:
        plot += scale
    return plot


def _tile_fills(plot: PlotSpec) -> list[str]:
    """Tile fills in data order (lowercase hex)."""
    panel = plot.to_svg().split('<g class="panel"', 1)[1]
    fills = re.findall(r'<rect[^>]*fill="(#[0-9a-fA-F]{6})"', panel)
    return [fill.lower() for fill in fills[: len(VALUES)]]


def _colorbar(plot: PlotSpec) -> str:
    """The legend part of the SVG (colorbar image and labels)."""
    svg = plot.to_svg()
    match = re.search(r'<g class="legend.*', svg, re.S)
    assert match is not None
    return match.group(0)


def _black_white(**options: object) -> FeatureSpec:
    return gg.scale_fill_gradient(low=BLACK, high=WHITE, limits=LIMITS, **options)


def test_censor_is_the_default_and_draws_na_value() -> None:
    for scale in (_black_white(), _black_white(oob="censor")):
        fills = _tile_fills(_tiles(scale))
        assert fills[0] == fills[4] == fills[5] == NA_GREY
        assert fills[1] == BLACK
        assert fills[3] == WHITE


def test_squish_clamps_to_the_end_colors_and_keeps_missing_na() -> None:
    fills = _tile_fills(_tiles(_black_white(oob="squish")))
    assert fills[:5] == [BLACK, BLACK, fills[2], WHITE, WHITE]
    assert fills[2] not in {BLACK, WHITE, NA_GREY}
    assert fills[5] == NA_GREY


def test_squish_leaves_the_colorbar_unchanged() -> None:
    assert _colorbar(_tiles(_black_white(oob="squish"))) == _colorbar(_tiles(_black_white()))


def test_squish_one_open_limit() -> None:
    scale = gg.scale_fill_gradient(low=BLACK, high=WHITE, limits=[None, 2.5], oob="squish")
    fills = _tile_fills(_tiles(scale))
    assert fills[0] == BLACK  # the data minimum, no lower limit
    assert fills[3] == fills[4] == WHITE


SCALES: list[Callable[..., FeatureSpec]] = [
    gg.scale_color_gradient,
    gg.scale_color_gradient2,
    gg.scale_color_gradientn,
    gg.scale_color_viridis,
    gg.scale_color_continuous,
    gg.scale_color_brewer,
    gg.scale_color_grey,
    gg.scale_fill_gradient,
    gg.scale_fill_gradient2,
    gg.scale_fill_gradientn,
    gg.scale_fill_viridis,
    gg.scale_fill_continuous,
    gg.scale_fill_brewer,
    gg.scale_fill_grey,
]


@pytest.mark.parametrize("scale", SCALES, ids=lambda scale: scale.__name__)
def test_every_continuous_color_scale_squishes(scale: Callable[..., FeatureSpec]) -> None:
    aesthetic = "fill" if "fill" in scale.__name__ else "color"
    options: dict[str, object] = {"limits": LIMITS, "oob": "squish"}
    if scale.__name__.endswith("gradientn"):
        options["colors"] = [BLACK, WHITE]
    spec = scale(**options)
    assert spec.as_dict()["oob"] == "squish"
    base = gg.ggplot(FRAME, gg.aes("x", "y", **{aesthetic: "v"}))
    base += gg.geom_tile() if aesthetic == "fill" else gg.geom_point(size=8)
    squished = (base + spec).to_svg().lower()
    assert squished.count(NA_GREY) == 1, scale.__name__  # only the missing value
    censored = (base + scale(**{**options, "oob": "censor"})).to_svg().lower()
    assert censored.count(NA_GREY) == 3, scale.__name__


def test_oob_survives_a_later_palette_like_limits() -> None:
    plot = _tiles(
        gg.scale_fill_continuous(limits=LIMITS, oob="squish"),
        gg.scale_fill_gradientn(colors=[BLACK, WHITE]),
    )
    assert _tile_fills(plot) == _tile_fills(_tiles(_black_white(oob="squish")))


def test_a_later_oob_wins() -> None:
    plot = _tiles(_black_white(oob="squish"), gg.scale_fill_gradient(oob="censor"))
    fills = _tile_fills(plot)
    assert fills[0] == fills[4] == NA_GREY


def test_squish_renders_end_colors_in_png() -> None:
    buffer = io.BytesIO()
    _tiles(_black_white(oob="squish")).to_png(buffer, scale=1)
    image = np.asarray(Image.open(io.BytesIO(buffer.getvalue())).convert("RGB"))
    grey = np.all(image == [0x80, 0x80, 0x80], axis=-1).sum()
    black = np.all(image == [0, 0, 0], axis=-1).sum()
    buffer = io.BytesIO()
    _tiles(_black_white()).to_png(buffer, scale=1)
    censored = np.asarray(Image.open(io.BytesIO(buffer.getvalue())).convert("RGB"))
    # Two more tiles of NA grey when censored; one more black tile when squished.
    assert np.all(censored == [0x80, 0x80, 0x80], axis=-1).sum() > 2 * grey
    assert black > np.all(censored == [0, 0, 0], axis=-1).sum()


def test_unknown_oob_fails_early_with_a_suggestion() -> None:
    with pytest.raises(
        SpecError, match=r"oob='sqush'.*'censor', 'squish'; did you mean 'squish'\?"
    ):
        gg.scale_fill_gradient(oob="sqush")  # ty: ignore[invalid-argument-type]


def test_non_string_oob_is_a_type_error() -> None:
    with pytest.raises(UnsupportedValueError, match=r"expected an oob name, got int"):
        gg.scale_fill_gradient(oob=1)  # ty: ignore[invalid-argument-type]


def test_engine_checks_oob_at_its_spec_path() -> None:
    raw = FeatureSpec("scale", None, aesthetic="fill", oob="sqush")
    with pytest.raises(SpecError, match=r"scales\[0\]\.oob.*did you mean 'squish'\?"):
        _tiles(raw).to_svg()
    with pytest.raises(SpecError, match=r"scales\[0\]\.oob.*expected an oob name"):
        _tiles(FeatureSpec("scale", None, aesthetic="fill", oob=1)).to_svg()
