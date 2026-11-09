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
