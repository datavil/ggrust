"""``geom_beeswarm`` / ``geom_quasirandom``: points land where R's ggbeeswarm puts them.

The reference positions come from ggbeeswarm 0.7.2 (``ggplot_build``), written by
``tests/fixtures/generate_ggbeeswarm_reference.R``.
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
    from ggrust.plot.core import LayerSpec

REFERENCE = json.loads(
    (Path(__file__).parent / "fixtures" / "ggbeeswarm_reference.json").read_text()
)
FRAME = pl.DataFrame(REFERENCE["data"])
ROWS = FRAME.height

B, Q = gg.geom_beeswarm, gg.geom_quasirandom
CASES: dict[str, tuple[tuple[str, ...], LayerSpec]] = {
    "swarm": (("g", "v"), B()),
    "swarm_cex": (("g", "v"), B(cex=2.5)),
    "swarm_descending": (("g", "v"), B(priority="descending")),
    "swarm_none": (("g", "v"), B(priority="none")),
    "swarm_density": (("g", "v"), B(priority="density")),
    "swarm_side_right": (("g", "v"), B(side=1)),
    "swarm_side_left": (("g", "v"), B(side=-1)),
    "compactswarm": (("g", "v"), B(method="compactswarm")),
    "compactswarm_side": (("g", "v"), B(method="compactswarm", side=1)),
    "center": (("g", "v"), B(method="center")),
    "hex": (("g", "v"), B(method="hex")),
    "square": (("g", "v"), B(method="square")),
    "square_side": (("g", "v"), B(method="square", side=-1)),
    "hex_side": (("g", "v"), B(method="hex", side=1)),
    "swarm_dodge": (("g", "v", "k"), B(dodge_width=0.8)),
    "swarm_flipped": (("v", "g"), B()),
    "swarm_continuous_x": (("n", "v"), B()),
    "quasirandom": (("g", "v"), Q()),
    "quasirandom_bandwidth": (("g", "v"), Q(bandwidth=1.5)),
    "quasirandom_width": (("g", "v"), Q(width=0.2)),
    "quasirandom_nbins": (("g", "v"), Q(nbins=100)),
    # ggbeeswarm 0.7.2 ignores varwidth (a bug); this is vipor's documented scaling.
    "quasirandom_varwidth_vipor": (("g", "v"), Q(varwidth=True)),
    "smiley": (("g", "v"), Q(method="smiley")),
    "frowney": (("g", "v"), Q(method="frowney")),
    "quasirandom_dodge": (("g", "v", "k"), Q(dodge_width=0.8)),
    "quasirandom_flipped": (("v", "g"), Q()),
    "quasirandom_continuous_x": (("n", "v"), Q()),
}


def drawn_points(plot: gg.PlotSpec) -> np.ndarray:
    """Centres of the first ``ROWS`` circles (the layer's points, in row order)."""
    found = re.findall(r'<circle[^>]*cx="([-\d.]+)"[^>]*cy="([-\d.]+)"', plot.to_svg())
    return np.array([(float(x), float(y)) for x, y in found])[:ROWS]


def plot_of(name: str) -> gg.PlotSpec:
    columns, layer = CASES[name]
    color = {"color": columns[2]} if len(columns) == 3 else {}
    return gg.ggplot(FRAME, gg.aes(columns[0], columns[1], **color)) + layer


def data_error(pixels: np.ndarray, data: np.ndarray) -> float:
    """Largest distance, in data units, from the best linear pixel mapping."""
    design = np.vstack([data, np.ones(len(data))]).T
    coefficients, *_ = np.linalg.lstsq(design, pixels, rcond=None)
    return float(np.abs(design @ coefficients - pixels).max() / abs(coefficients[0]))


@pytest.mark.parametrize("name", list(CASES))
def test_points_match_ggbeeswarm(name: str) -> None:
    expected = REFERENCE["cases"][name]
    points = drawn_points(plot_of(name))
    assert len(points) == ROWS
    # SVG coordinates carry two decimals: about 3e-5 data units here.
    assert data_error(points[:, 0], np.array(expected["x"])) < 1e-3
    assert data_error(points[:, 1], np.array(expected["y"])) < 1e-3


def test_orientation_y_is_the_flipped_layout() -> None:
    flipped = drawn_points(gg.ggplot(FRAME, gg.aes("v", "g")) + gg.geom_beeswarm())
    explicit = drawn_points(gg.ggplot(FRAME, gg.aes("v", "g")) + gg.geom_beeswarm(orientation="y"))
    np.testing.assert_allclose(flipped, explicit)


def test_position_dodge_sets_dodge_width() -> None:
    dodged = gg.geom_quasirandom(position=gg.position_dodge(width=0.8)).as_dict()
    assert dodged["dodge_width"] == 0.8
    assert "position" not in dodged or dodged["position"] is None
    assert gg.geom_beeswarm(position="dodge").as_dict()["dodge_width"] == 0.95
    by_position = drawn_points(
        gg.ggplot(FRAME, gg.aes("g", "v", color="k"))
        + gg.geom_beeswarm(position=gg.position_dodge(width=0.8))
    )
    np.testing.assert_allclose(by_position, drawn_points(plot_of("swarm_dodge")))


@pytest.mark.parametrize(
    "layer",
    [
        gg.geom_quasirandom(method="pseudorandom", seed=3),
        gg.geom_beeswarm(priority="random", seed=3),
        gg.geom_quasirandom(method="pseudorandom"),
    ],
    ids=["pseudorandom", "random_priority", "default_seed"],
)
def test_random_layouts_are_seeded(layer: LayerSpec) -> None:
    plot = gg.ggplot(FRAME, gg.aes("g", "v")) + layer
    assert plot.to_svg() == plot.to_svg()


def test_seed_changes_pseudorandom_points() -> None:
    def build(seed: int) -> np.ndarray:
        layer = gg.geom_quasirandom(method="pseudorandom", seed=seed)
        return drawn_points(gg.ggplot(FRAME, gg.aes("g", "v")) + layer)

    assert not np.allclose(build(1), build(2))


def test_missing_values_and_facets_render() -> None:
    frame = FRAME.with_columns(
        pl.when(pl.int_range(pl.len()) % 7 == 0).then(None).otherwise(pl.col("v")).alias("v")
    )
    plot = gg.ggplot(frame, gg.aes("g", "v")) + gg.geom_beeswarm(na_rm=True) + gg.facet_wrap("k")
    assert plot.to_svg().count("<circle") >= frame["v"].drop_nulls().len()
    with pytest.warns(UserWarning, match="missing"):
        assert "<circle" in (gg.ggplot(frame, gg.aes("g", "v")) + gg.geom_quasirandom()).to_svg()


@pytest.mark.parametrize(
    ("build", "error", "parts"),
    [
        (lambda: gg.geom_beeswarm(method="swram"), SpecError, ["swram", "did you mean 'swarm'"]),
        (lambda: gg.geom_beeswarm(priority="ascend"), SpecError, ["priority", "ascending"]),
        (lambda: gg.geom_beeswarm(side=2), SpecError, ["side", "-1"]),
        (lambda: gg.geom_beeswarm(cex=0), SpecError, ["cex", "positive"]),
        (lambda: gg.geom_beeswarm(cex="big"), UnsupportedValueError, ["cex", "str"]),
        (lambda: gg.geom_beeswarm(method=3), UnsupportedValueError, ["method", "int"]),
        (lambda: gg.geom_quasirandom(method="tukey"), SpecError, ["tukey", "not supported"]),
        (
            lambda: gg.geom_quasirandom(method="smily"),
            SpecError,
            ["smily", "did you mean 'smiley'"],
        ),
        (lambda: gg.geom_quasirandom(width=-1), SpecError, ["width", "positive"]),
        (lambda: gg.geom_quasirandom(nbins=1), SpecError, ["nbins", "at least 2"]),
        (lambda: gg.geom_quasirandom(seed=1.5), SpecError, ["seed", "whole number"]),
        (lambda: gg.geom_quasirandom(bandwidth=0), SpecError, ["bandwidth", "positive"]),
        (lambda: gg.geom_quasirandom(dodge_width=0), SpecError, ["dodge_width", "positive"]),
    ],
)
def test_bad_parameters_fail_early(build: object, error: type[Exception], parts: list[str]) -> None:
    with pytest.raises(error) as raised:
        build()  # type: ignore[operator]
    for part in parts:
        assert part in str(raised.value)


@pytest.mark.parametrize(
    ("layer", "parts"),
    [
        (gg.layer("beeswarm", stat="beeswarm", method="swram"), ["method", "swarm"]),
        (gg.layer("quasirandom", stat="quasirandom", nbins=1.5), ["nbins", "whole number"]),
        (gg.layer("beeswarm", stat="beeswarm", side=3), ["side", "-1"]),
    ],
)
def test_engine_rejects_bad_parameters(layer: LayerSpec, parts: list[str]) -> None:
    with pytest.raises(SpecError) as raised:
        (gg.ggplot(FRAME, gg.aes("g", "v")) + layer).to_svg()
    for part in parts:
        assert part in str(raised.value)


def test_numpy_numbers_are_accepted() -> None:
    layer = gg.geom_quasirandom(nbins=np.int64(10), width=np.float64(0.3), seed=np.int64(2))
    assert "<circle" in (gg.ggplot(FRAME, gg.aes("g", "v")) + layer).to_svg()


def test_large_swarm_stays_fast() -> None:
    import time

    values = np.random.default_rng(1).normal(0, 1, 20_000)
    frame = pl.DataFrame({"g": ["a"] * len(values), "v": values})
    start = time.perf_counter()
    (gg.ggplot(frame, gg.aes("g", "v")) + gg.geom_beeswarm()).to_svg()
    assert time.perf_counter() - start < 5
