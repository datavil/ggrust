"""``geom_streamline`` / ``stat_streamline``: the layer draws what ``velocity_grid``
plus ``streamplot`` return when called by hand.
"""

from __future__ import annotations

import re

import ggrust as gg
import numpy as np
import polars as pl
import pytest
from ggrust.errors import SpecError, UnsupportedValueError


def _wind() -> pl.DataFrame:
    """A field on a regular 31 x 21 grid, rows in shuffled order."""
    east, north = np.meshgrid(np.linspace(-3, 3, 31), np.linspace(-2, 2, 21))
    frame = pl.DataFrame(
        {
            "east": east.ravel(),
            "north": north.ravel(),
            "u": (-north - 0.3 * east).ravel(),
            "v": (east - 0.3 * north).ravel(),
        }
    )
    return frame.sample(fraction=1.0, shuffle=True, seed=3)


def _scattered(n: int = 1500, seed: int = 4) -> pl.DataFrame:
    generator = np.random.default_rng(seed)
    x, y = generator.normal(size=(2, n))
    return pl.DataFrame({"x": x, "y": y, "u": 1 - y**2, "v": 0.5 * x})


def _lines(plot: gg.PlotSpec, index: int = 0) -> list[pl.DataFrame]:
    """The layer's lines, one frame of x, y per group, in group order."""
    data = plot.layer_data(index)
    return [part.select("x", "y") for part in data.partition_by("group", maintain_order=True)]


def _expected(streams: gg.StreamplotSet) -> list[pl.DataFrame]:
    return [
        part.select("x", "y") for part in streams.lines.partition_by("group", maintain_order=True)
    ]


def _assert_same_lines(actual: list[pl.DataFrame], expected: list[pl.DataFrame]) -> None:
    assert len(actual) == len(expected) > 3
    for left, right in zip(actual, expected, strict=True):
        assert left.height == right.height
        np.testing.assert_array_equal(left.to_numpy(), right.to_numpy())


def test_scattered_rows_equal_velocity_grid_then_streamplot() -> None:
    frame = _scattered()
    options = {"grid_density": 0.8, "smooth": 0.6, "n_neighbors": 25, "min_mass": 0.5}
    stream_options = {"density": 1.5, "maxlength": 3.0, "integration_direction": "forward"}
    plot = gg.ggplot(frame, gg.aes("x", "y", u="u", v="v")) + gg.geom_streamline(
        **options, **stream_options, cutoff_percentile=10
    )
    grid = gg.velocity_grid(
        frame.select("x", "y"),
        frame.select("u", "v"),
        density=0.8,
        smooth=0.6,
        n_neighbors=25,
        min_mass=0.5,
        cutoff_percentile=10,
    )
    streams = gg.streamplot(grid.x, grid.y, grid.u, grid.v, **stream_options)
    _assert_same_lines(_lines(plot), _expected(streams))


def test_grid_rows_are_the_field_as_given() -> None:
    frame = _wind()
    plot = gg.ggplot(frame, gg.aes("east", "north")) + gg.geom_streamline(
        gg.aes(u="u", v="v"), density=(1.2, 0.8), broken_streamlines=False, minlength=0.2
    )
    ordered = frame.sort("north", "east")
    x = ordered["east"].unique().sort().to_numpy()
    y = ordered["north"].unique().sort().to_numpy()
    u = ordered["u"].to_numpy().reshape(len(y), len(x))
    v = ordered["v"].to_numpy().reshape(len(y), len(x))
    streams = gg.streamplot(x, y, u, v, density=(1.2, 0.8), broken_streamlines=False, minlength=0.2)
    _assert_same_lines(_lines(plot), _expected(streams))


def test_gridded_false_smooths_a_grid_and_true_accepts_holes() -> None:
    frame = _wind()
    smoothed = gg.ggplot(frame, gg.aes("east", "north", u="u", v="v")) + gg.geom_streamline(
        gridded=False, n_neighbors=10
    )
    grid = gg.velocity_grid(frame.select("east", "north"), frame.select("u", "v"), n_neighbors=10)
    _assert_same_lines(_lines(smoothed), _expected(gg.streamplot(grid.x, grid.y, grid.u, grid.v)))
    holed = frame.filter(~((pl.col("east").abs() < 0.05) & (pl.col("north").abs() < 0.05)))
    plot = gg.ggplot(holed, gg.aes("east", "north", u="u", v="v"))
    with_holes = plot + gg.geom_streamline(gridded=True)
    # Without gridded=True the incomplete grid is scattered: 650 rows give n_neighbors 13.
    as_points = plot + gg.geom_streamline()
    assert _lines(with_holes)[0].height > 0
    assert len(_lines(with_holes)) != len(_lines(as_points)) or not all(
        left.equals(right)
        for left, right in zip(_lines(with_holes), _lines(as_points), strict=False)
    )


def test_groups_and_facets_get_their_own_lines() -> None:
    left = _scattered(800, seed=5).with_columns(side=pl.lit("left"))
    right = (
        _scattered(800, seed=6)
        .with_columns(pl.col("x") + 6, side=pl.lit("right"))
        .with_columns(pl.col("u") * -1)
    )
    frame = pl.concat([left, right])
    grouped = gg.ggplot(frame, gg.aes("x", "y", u="u", v="v", color="side")) + gg.geom_streamline(
        n_neighbors=16
    )
    data = grouped.layer_data()
    for side, part in [("left", left), ("right", right)]:
        grid = gg.velocity_grid(part.select("x", "y"), part.select("u", "v"), n_neighbors=16)
        streams = gg.streamplot(grid.x, grid.y, grid.u, grid.v)
        mine = data.filter(pl.col("color") == side)
        lines = [p.select("x", "y") for p in mine.partition_by("group", maintain_order=True)]
        _assert_same_lines(lines, _expected(streams))
    faceted = (
        gg.ggplot(frame, gg.aes("x", "y", u="u", v="v"))
        + gg.geom_streamline(n_neighbors=16)
        + gg.facet_wrap("side")
    )
    panels = faceted.layer_data()
    assert panels["panel"].unique().sort().to_list() == [0, 1]
    left_lines = [
        p.select("x", "y")
        for p in panels.filter(pl.col("panel") == 0).partition_by("group", maintain_order=True)
    ]
    grid = gg.velocity_grid(left.select("x", "y"), left.select("u", "v"), n_neighbors=16)
    _assert_same_lines(left_lines, _expected(gg.streamplot(grid.x, grid.y, grid.u, grid.v)))


def _heads(svg: str, color: str) -> int:
    """Closed arrowheads (filled paths) of ``color`` in an SVG."""
    return len(re.findall(rf'<path[^>]*fill="{color}"', svg, flags=re.IGNORECASE))


def test_one_arrowhead_per_line_and_arrow_none() -> None:
    frame = _wind()
    base = gg.ggplot(frame, gg.aes("east", "north", u="u", v="v"))
    lines = len(_lines(base + gg.geom_streamline()))
    svg = (base + gg.geom_streamline(color="#123456")).to_svg()
    assert _heads(svg, "#123456") == lines
    assert _heads((base + gg.geom_streamline(color="#123456", arrow=None)).to_svg(), "#123456") == 0
    open_heads = (base + gg.geom_streamline(color="#123456", arrow=gg.arrow())).to_svg()
    assert _heads(open_heads, "#123456") == 0


def test_stat_streamline_with_geom_path() -> None:
    frame = _wind()
    plot = gg.ggplot(frame, gg.aes("east", "north", u="u", v="v"))
    via_stat = plot + gg.stat_streamline(geom="path")
    _assert_same_lines(_lines(via_stat), _lines(plot + gg.geom_streamline()))
    assert via_stat.to_svg()


def test_standalone_functions_unchanged() -> None:
    frame = _scattered()
    grid = gg.velocity_grid(frame.select("x", "y"), frame.select("u", "v"))
    streams = gg.streamplot(grid.x, grid.y, grid.u, grid.v)
    assert streams.lines.columns == ["x", "y", "group"]
    assert streams.arrows.columns == ["x", "y", "xend", "yend", "group"]


@pytest.mark.parametrize(
    ("options", "error", "message"),
    [
        ({"integration_direction": "forwards"}, SpecError, "did you mean 'forward'"),
        ({"density": 0}, SpecError, "density must be a positive number"),
        ({"density": (1, 2, 3)}, SpecError, "a pair"),
        ({"smooth": -1}, SpecError, "smooth must be a positive number"),
        ({"n_neighbors": 2.5}, SpecError, "whole number"),
        ({"n_neighbors": "10"}, UnsupportedValueError, "whole number"),
        ({"gridded": "yes"}, UnsupportedValueError, "True, False or None"),
    ],
)
def test_options_fail_early(
    options: dict[str, object], error: type[Exception], message: str
) -> None:
    with pytest.raises(error, match=re.escape(message)):
        gg.geom_streamline(**options)  # type: ignore[arg-type]


@pytest.mark.parametrize(
    ("options", "path", "message"),
    [
        ({"density": 0.01}, "layers[0].density", "density"),
        ({"grid_density": 0.01}, "layers[0].grid_density", "grid_density must be"),
        ({"cutoff_percentile": 120}, "layers[0].cutoff_percentile", "between 0 and 100"),
        ({"n_neighbors": 5000}, "layers[0].n_neighbors", "number of finite points"),
        ({"gridded": True}, "layers[0].data", "values must be equally spaced"),
    ],
)
def test_engine_errors_name_the_layer_option(
    options: dict[str, object], path: str, message: str
) -> None:
    frame = _scattered()
    plot = gg.ggplot(frame, gg.aes("x", "y", u="u", v="v")) + gg.geom_streamline(**options)  # type: ignore[arg-type]
    with pytest.raises(SpecError) as caught:
        plot.to_svg()
    assert path in str(caught.value)
    assert message in str(caught.value)


def test_gridded_true_rejects_repeated_positions() -> None:
    frame = _wind()
    plot = gg.ggplot(pl.concat([frame, frame.head(1)]), gg.aes("east", "north", u="u", v="v"))
    with pytest.raises(SpecError, match=re.escape("layers[0].gridded")):
        (plot + gg.geom_streamline(gridded=True)).to_svg()
    # By default the repeated row makes the rows scattered: smoothed, no error.
    assert (plot + gg.geom_streamline()).to_svg()


def test_missing_u_v_is_reported() -> None:
    frame = _scattered()
    plot = gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_streamline()
    with pytest.raises(SpecError, match="missing: u, v"):
        plot.to_svg()


def test_small_group_without_n_neighbors_explains_the_default() -> None:
    frame = _scattered(30)
    plot = gg.ggplot(frame, gg.aes("x", "y", u="u", v="v")) + gg.geom_streamline()
    with pytest.raises(SpecError, match=re.escape("int(n_points / 50)")):
        plot.to_svg()


def test_negative_zero_and_log_scale_grids() -> None:
    frame = _wind().with_columns(
        pl.when(pl.col("east") == 0).then(pl.lit(-0.0)).otherwise(pl.col("east")).alias("east")
    )
    plot = gg.ggplot(frame, gg.aes("east", "north", u="u", v="v"))
    _assert_same_lines(
        _lines(plot + gg.geom_streamline(gridded=True)),
        _lines(gg.ggplot(_wind(), gg.aes("east", "north", u="u", v="v")) + gg.geom_streamline()),
    )
    logged = frame.with_columns(pl.col("east") + 4)
    log_plot = (
        gg.ggplot(logged, gg.aes("east", "north", u="u", v="v"))
        + gg.geom_streamline()
        + gg.scale_x_log10()
    )
    with pytest.raises(SpecError, match="pass gridded=False"):
        log_plot.to_svg()
