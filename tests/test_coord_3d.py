"""coord_3d (plan: 3D plots, tier 1): static 3D views of geom_point and geom_path."""

from __future__ import annotations

import re
from pathlib import Path

import ggrust as gg
import polars as pl
import pytest
from ggrust.errors import SpecError

FRAME = pl.DataFrame(
    {
        "x": [1.0, 2.0, 3.0, 4.0],
        "y": [2.0, 1.0, 4.0, 3.0],
        "z": [10.0, 40.0, 20.0, 30.0],
        "kind": ["a", "b", "a", "b"],
    }
)


def plot_3d(**coord: float | bool) -> gg.PlotSpec:
    return (
        gg.ggplot(FRAME, gg.aes("x", "y", z="z", color="kind"))
        + gg.geom_point()
        + gg.coord_3d(**coord)
    )


def texts(svg: str) -> list[str]:
    return re.findall(r"<text[^>]*>([^<]*)</text>", svg)


def test_spec_round_trips_the_view() -> None:
    spec = gg.coord_3d(azimuth=10, elevation=-5, perspective=True).as_dict()
    assert spec == {"name": "3d", "azimuth": 10, "elevation": -5, "perspective": True}


def test_every_output_renders(tmp_path: Path) -> None:
    plot = plot_3d()
    assert plot.to_svg().startswith("<svg")
    assert plot.to_html()
    for suffix in ["png", "pdf"]:
        path = tmp_path / f"plot.{suffix}"
        getattr(plot, f"to_{suffix}")(path)
        assert path.stat().st_size > 1000


def test_box_shows_three_axis_titles_and_z_labels() -> None:
    svg = (plot_3d() + gg.scale_z_continuous("height", breaks=[10, 25, 40])).to_svg()
    found = texts(svg)
    for title in ["x", "y", "height", "kind"]:
        assert title in found
    for label in ["10", "25", "40"]:
        assert label in found


def test_labs_names_the_z_axis() -> None:
    assert "Depth" in texts((plot_3d() + gg.labs(z="Depth")).to_svg())


def test_points_draw_back_to_front() -> None:
    """From the front (azimuth 0) the row with the largest x is nearest: drawn last."""
    frame = pl.DataFrame({"x": [3.0, 1.0, 2.0], "y": [0.0, 0.0, 0.0], "z": [0.0, 0.0, 0.0]})
    colors = ["#ff0000", "#00ff00", "#0000ff"]
    plot = (
        gg.ggplot(frame.with_columns(pl.Series("color", colors)), gg.aes("x", "y", z="z"))
        + gg.geom_point(gg.aes(color="color"))
        + gg.scale_color_identity()
        + gg.coord_3d(azimuth=0, elevation=0)
    )
    svg = plot.to_svg().lower()
    positions = [svg.find(color.lower()) for color in ["#00ff00", "#0000ff", "#ff0000"]]
    assert all(position >= 0 for position in positions)
    assert positions == sorted(positions)


def test_view_changes_the_picture() -> None:
    assert plot_3d().to_svg() != plot_3d(azimuth=30).to_svg()
    assert plot_3d().to_svg() != plot_3d(perspective=True).to_svg()


def test_facets_and_paths_render() -> None:
    plot = (
        gg.ggplot(FRAME, gg.aes("x", "y", z="z"))
        + gg.geom_path()
        + gg.geom_point()
        + gg.coord_3d()
        + gg.facet_wrap("kind")
    )
    svg = plot.to_svg()
    assert svg.count(">z<") == 2


def test_2d_geom_under_coord_3d_is_an_error() -> None:
    plot = gg.ggplot(FRAME, gg.aes("x", "y", z="z")) + gg.geom_bar() + gg.coord_3d()
    with pytest.raises(SpecError, match=r"layers\[0\]\.geom.*geom_bar cannot draw under coord_3d"):
        plot.to_svg()


def test_missing_z_is_an_error() -> None:
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point() + gg.coord_3d()
    with pytest.raises(SpecError, match="requires aesthetic z"):
        plot.to_svg()


def test_categorical_z_is_an_error() -> None:
    plot = gg.ggplot(FRAME, gg.aes("x", "y", z="kind")) + gg.geom_point() + gg.coord_3d()
    with pytest.raises(SpecError, match="needs numbers on z"):
        plot.to_svg()


def test_non_finite_angle_is_an_error() -> None:
    with pytest.raises(SpecError, match=r"azimuth"):
        gg.coord_3d(azimuth=float("inf"))


def test_z_without_coord_3d_warns() -> None:
    plot = gg.ggplot(FRAME, gg.aes("x", "y", z="z")) + gg.geom_point()
    with pytest.warns(gg.GgrustWarning, match="draws z only under coord_3d"):
        plot.to_svg()


def test_html_notes_that_3d_is_static() -> None:
    with pytest.warns(gg.GgrustWarning, match="static"):
        (plot_3d() + gg.ggtb()).to_html()


def test_constant_z_is_accepted() -> None:
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point(z=1) + gg.coord_3d()
    assert plot.to_svg().count("<circle") == 4


def test_a_stat_under_coord_3d_is_an_error() -> None:
    plot = gg.ggplot(FRAME, gg.aes("x", "y", z="z")) + gg.geom_point(stat="count") + gg.coord_3d()
    with pytest.raises(SpecError, match=r"layers\[0\]\.stat.*cannot carry z"):
        plot.to_svg()


def test_an_edge_seen_end_on_has_no_labels() -> None:
    """Looking straight down, z runs at the viewer: its labels would pile up."""
    plot = plot_3d(elevation=90) + gg.scale_z_continuous(breaks=[10, 20, 30, 40])
    found = texts(plot.to_svg())
    assert not {"10", "20", "30", "40"} & set(found)
    assert "z" in found


def test_html_legend_is_not_clickable() -> None:
    """2D plots tag their marks with legend keys; 3D ones would leave dead keys."""
    html = plot_3d().to_html() or ""
    assert not re.search(r"<g[^>]*data-keys", html)
    assert "data-key=" not in html


def test_z_na_value_draws_dropped_rows() -> None:
    plain = gg.ggplot(FRAME, gg.aes("x", "y", z="z")) + gg.geom_point() + gg.coord_3d()
    limited = plain + gg.scale_z_continuous(limits=[15, 35])
    assert limited.to_svg().count("<circle") == 2
    kept = plain + gg.scale_z_continuous(limits=[15, 35], na_value=15)
    assert kept.to_svg().count("<circle") == 4


def surface_grid(xs: list[float], ys: list[float]) -> pl.DataFrame:
    return pl.DataFrame(
        {
            "x": [x for _ in ys for x in xs],
            "y": [y for y in ys for _ in xs],
            "z": [x + y for y in ys for x in xs],
        }
    )


def test_surface_draws_one_quad_per_cell() -> None:
    plot = (
        gg.ggplot(surface_grid([0, 1, 2], [0, 1, 2]), gg.aes("x", "y", z="z"))
        + gg.geom_surface(fill="#123456")
        + gg.coord_3d()
    )
    assert plot.to_svg().lower().count('fill="#123456"') == 4


def test_surface_skips_cells_with_a_missing_corner() -> None:
    frame = surface_grid([0, 1, 2], [0, 1, 2]).with_columns(
        pl.when((pl.col("x") == 2) & (pl.col("y") == 2))
        .then(None)
        .otherwise(pl.col("z"))
        .alias("z")
    )
    plot = (
        gg.ggplot(frame, gg.aes("x", "y", z="z"))
        + gg.geom_surface(fill="#123456", na_rm=True)
        + gg.coord_3d()
    )
    assert plot.to_svg().lower().count('fill="#123456"') == 3


def test_surface_cells_draw_far_to_near_with_mean_fill() -> None:
    """From the front (azimuth 0) the cell at larger x is nearer: drawn last.
    Its corners are red, red, blue, blue: its fill is their mean."""
    frame = surface_grid([0, 1, 2], [0, 1]).with_columns(
        pl.when(pl.col("x") == 2)
        .then(pl.lit("#0000ff"))
        .otherwise(pl.lit("#ff0000"))
        .alias("paint")
    )
    plot = (
        gg.ggplot(frame, gg.aes("x", "y", z="z", fill="paint"))
        + gg.geom_surface(size=0)
        + gg.scale_fill_identity()
        + gg.coord_3d(azimuth=0, elevation=20)
    )
    svg = plot.to_svg().lower()
    far, near = svg.find('fill="#ff0000"'), svg.find('fill="#800080"')
    assert 0 <= far < near


def test_surface_without_coord_3d_is_an_error() -> None:
    plot = gg.ggplot(surface_grid([0, 1], [0, 1]), gg.aes("x", "y", z="z")) + gg.geom_surface()
    with pytest.raises(SpecError, match="geom_surface draws only under coord_3d"):
        plot.to_svg()


def test_scattered_surface_data_is_an_error() -> None:
    frame = pl.DataFrame(
        {"x": [0.1 * i for i in range(30)], "y": [0.37 * i % 3 for i in range(30)]}
    )
    plot = (
        gg.ggplot(frame.with_columns(z=pl.col("x")), gg.aes("x", "y", z="z"))
        + gg.geom_surface()
        + gg.coord_3d()
    )
    with pytest.raises(SpecError, match="needs x and y on a grid"):
        plot.to_svg()


def test_surface_merges_near_equal_coordinates() -> None:
    """x from two arithmetic sources differs by ~1e-16: still one grid."""
    steps = [round(0.1 * (i + 1), 10) for i in range(10)]
    noisy = [sum([0.1] * (i + 1)) for i in range(10)]
    frame = pl.DataFrame(
        {
            "x": [(steps if j % 2 else noisy)[i] for j in range(10) for i in range(10)],
            "y": [float(j) for j in range(10) for _ in range(10)],
        }
    ).with_columns(z=pl.col("x") + pl.col("y"))
    plot = (
        gg.ggplot(frame, gg.aes("x", "y", z="z")) + gg.geom_surface(fill="#123456") + gg.coord_3d()
    )
    assert plot.to_svg().lower().count('fill="#123456"') == 81


def test_sparse_partial_grid_is_accepted() -> None:
    """A diagonal band of a 100 x 100 grid uses few of its cells."""
    rows = [(float(x), float(y)) for x in range(100) for y in range(100) if abs(x - y) <= 1]
    frame = pl.DataFrame(rows, schema=["x", "y"], orient="row").with_columns(z=pl.col("x"))
    plot = (
        gg.ggplot(frame, gg.aes("x", "y", z="z")) + gg.geom_surface(fill="#123456") + gg.coord_3d()
    )
    # Only the diagonal cells have four corners in the band.
    assert plot.to_svg().lower().count('fill="#123456"') == 99
