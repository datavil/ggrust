"""``geom_pointdensity`` / ``stat_pointdensity``: densities match R's ggpointdensity.

The reference densities come from ggpointdensity 0.2.1 (``ggplot_build``), written
by ``tests/fixtures/generate_ggpointdensity_reference.R``.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import TYPE_CHECKING

import ggrust as gg
import numpy as np
import polars as pl
import pytest
from ggrust.errors import SpecError, UnsupportedValueError

if TYPE_CHECKING:
    from collections.abc import Callable

REFERENCE = json.loads(
    (Path(__file__).parent / "fixtures" / "ggpointdensity_reference.json").read_text()
)
FRAME = pl.DataFrame(REFERENCE["data"])
EXTRA = pl.DataFrame(REFERENCE["extra"])

# Label color that tells the layer's texts from axis texts.
MARK = "#123456"

Case = tuple[bool, dict[str, object], bool]
# name -> (grouped by g, stat options, with the range-widening layer)
CASES: dict[str, Case] = {
    "neighbors": (False, {"method": "neighbors"}, False),
    "neighbors_adjust": (False, {"method": "neighbors", "adjust": 0.4}, False),
    "neighbors_groups": (True, {"method": "neighbors"}, False),
    "neighbors_wide": (False, {"method": "neighbors"}, True),
    "auto": (False, {}, False),
    "kde2d": (False, {"method": "kde2d"}, False),
    "kde2d_adjust": (False, {"method": "kde2d", "adjust": 2}, False),
    "kde2d_groups": (True, {"method": "kde2d"}, False),
    "kde2d_wide": (False, {"method": "kde2d"}, True),
}


def densities(
    frame: pl.DataFrame,
    options: dict[str, object],
    *,
    grouped: bool = False,
    wide: bool = False,
    variable: str = "..density..",
) -> np.ndarray:
    """The stat's ``variable`` for every row, read back from text labels in row order.

    Labels carry 6 significant digits (lets-plot's default number format).
    """
    mapping = gg.aes("x", "y", group="g") if grouped else gg.aes("x", "y")
    plot = gg.ggplot(frame, mapping) + gg.stat_pointdensity(
        gg.aes(label=variable), geom="text", color=MARK, **options
    )
    if wide:
        plot += gg.geom_point(data=EXTRA)
    texts = re.findall(rf'<text[^>]*fill="{MARK}"[^>]*>([^<]+)</text>', plot.to_svg())
    return np.array([float(text.replace(",", "").replace("n/a", "nan")) for text in texts])


@pytest.mark.parametrize("name", list(CASES))
def test_densities_match_ggpointdensity(name: str) -> None:
    grouped, options, wide = CASES[name]
    expected = np.array(REFERENCE["cases"][name])
    actual = densities(FRAME, options, grouped=grouped, wide=wide)
    assert actual.shape == expected.shape
    if name.startswith("kde2d"):
        np.testing.assert_allclose(actual, expected, rtol=1e-5)
    else:
        np.testing.assert_array_equal(actual, expected)


def test_constant_x_or_y_follows_ggpointdensity() -> None:
    # Rscript, ggpointdensity 0.2.1, method = "neighbors": a zero y range counts
    # duplicates only (1 1 1 1 1); a zero x range counts every point on the same x.
    constant_y = pl.DataFrame({"x": [1.0, 2.0, 3.0, 4.0, 10.0], "y": [5.0] * 5})
    assert densities(constant_y, {"method": "neighbors"}).tolist() == [1.0] * 5
    constant_x = pl.DataFrame({"x": [1.0] * 5, "y": [0.0, 1.0, 2.0, 4.0, 8.0]})
    assert densities(constant_x, {"method": "neighbors"}).tolist() == [5.0] * 5


def test_ndensity_is_density_over_group_maximum() -> None:
    for method in ("neighbors", "kde2d"):
        density = densities(FRAME, {"method": method}, grouped=True)
        normalized = densities(FRAME, {"method": method}, grouped=True, variable="..ndensity..")
        groups = FRAME["g"].to_numpy()
        for group in np.unique(groups):
            inside = groups == group
            expected = density[inside] / density[inside].max()
            np.testing.assert_allclose(normalized[inside], expected, rtol=1e-5)


def test_auto_switches_to_kde2d_above_20000_rows() -> None:
    generator = np.random.default_rng(5)
    for rows, method in ((20_000, "neighbors"), (20_001, "kde2d")):
        frame = pl.DataFrame(
            {"x": generator.normal(size=rows), "y": generator.normal(size=rows)}
        ).with_columns(g=pl.lit("a"))
        auto = densities(frame, {})
        np.testing.assert_array_equal(auto, densities(frame, {"method": method}))


def test_color_maps_to_density_by_default() -> None:
    data = {"x": [0.0, 0.1, 0.2, 5.0], "y": [0.0, 0.1, 0.0, 5.0]}
    plot = gg.ggplot(data, gg.aes("x", "y")) + gg.geom_pointdensity()
    svg = plot.to_svg()
    fills = re.findall(r'<circle[^>]*fill="(#[0-9a-fA-F]{6})"', svg)
    assert len(fills) == 4
    assert fills[0] == fills[1] == fills[2] != fills[3]
    assert "density" in svg  # the colorbar title
    constant = (gg.ggplot(data, gg.aes("x", "y")) + gg.geom_pointdensity(color="red")).to_svg()
    assert "density" not in constant


def test_layer_spec_round_trip() -> None:
    layer = gg.geom_pointdensity(method="kde2d", adjust=0.5, size=2).as_dict()
    assert layer["geom"] == "point"
    assert layer["stat"] == "pointdensity"
    assert (layer["method"], layer["adjust"]) == ("kde2d", 0.5)
    assert gg.stat_pointdensity().as_dict()["geom"] == "point"


def test_missing_values_and_single_points() -> None:
    frame = pl.DataFrame(
        {"x": [0.0, 0.1, None, 0.2, 9.0], "y": [0.0, 0.0, 0.0, None, 9.0], "g": list("aaaab")}
    )
    with pytest.warns(UserWarning, match="missing"):
        values = densities(frame.drop("g"), {"method": "neighbors"})
    # Rows with a missing x or y draw no label; the two near points see each other.
    assert values.tolist() == [2.0, 2.0, 1.0]
    # A group with one point gets no density (R: NA).
    grouped = densities(frame, {"method": "neighbors"}, grouped=True)
    np.testing.assert_array_equal(grouped, [2.0, 2.0, np.nan])


def test_facets_and_png_render(tmp_path: Path) -> None:
    plot = (
        gg.ggplot(FRAME, gg.aes("x", "y"))
        + gg.geom_pointdensity(size=1)
        + gg.facet_wrap("g")
        + gg.scale_color_viridis()
    )
    plot.to_png(tmp_path / "facets.png")
    assert (tmp_path / "facets.png").read_bytes()[:8] == b"\x89PNG\r\n\x1a\n"


ERRORS: list[tuple[Callable[[], object], type[Exception], str]] = [
    (lambda: gg.geom_pointdensity(method="neighbours"), SpecError, "did you mean 'neighbors'"),
    (lambda: gg.stat_pointdensity(method="knn"), SpecError, "unknown method 'knn'"),
    (lambda: gg.geom_pointdensity(method=2), UnsupportedValueError, "got int"),  # type: ignore[arg-type]
    (lambda: gg.geom_pointdensity(adjust=0), SpecError, "adjust must be a positive number"),
    (lambda: gg.geom_pointdensity(adjust=float("inf")), SpecError, "finite positive number"),
    (lambda: gg.geom_pointdensity(adjust="1"), UnsupportedValueError, "positive number"),  # type: ignore[arg-type]
]


@pytest.mark.parametrize(("build", "error", "message"), ERRORS)
def test_bad_parameters_fail_early(
    build: Callable[[], object], error: type[Exception], message: str
) -> None:
    with pytest.raises(error, match=re.escape(message)):
        build()


def test_engine_rejects_bad_options_with_their_path() -> None:
    layer = gg.geom_point(stat="pointdensity", method="kde2", adjust=1)
    with pytest.raises(SpecError, match=r"layers\[0\]\.method.*did you mean 'kde2d'"):
        (gg.ggplot(FRAME, gg.aes("x", "y")) + layer).to_svg()
    layer = gg.geom_point(stat="pointdensity", adjust=-1)
    with pytest.raises(SpecError, match=r"layers\[0\]\.adjust"):
        (gg.ggplot(FRAME, gg.aes("x", "y")) + layer).to_svg()
