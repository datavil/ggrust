"""Read-only accessors on plots and grids, ``with_*`` copies, ``layer_data`` and ``build``."""

from __future__ import annotations

import datetime
import doctest
import inspect
import re

import numpy as np
import polars as pl
import pytest
from ggrust import (
    aes,
    coord_cartesian,
    coord_flip,
    facet_wrap,
    geom_boxplot,
    geom_density,
    geom_histogram,
    geom_line,
    geom_point,
    geom_rect,
    geom_violin,
    gggrid,
    ggplot,
    ggsize,
    ggtitle,
    guides,
    labs,
    layer_tooltips,
    new_scale_color,
    scale_color_brewer,
    scale_x_continuous,
    scale_x_log10,
    theme,
    theme_bw,
)
from ggrust.errors import ColumnNotFoundError, PlotIndexError, SpecError
from ggrust.plot.core import LayerSpec, PlotSpec
from ggrust.plot.subplots import SupPlotsSpec

CARS = pl.DataFrame(
    {
        "weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1, 2.2, 3.0],
        "mpg": [31.0, 28.0, 24.0, 22.0, 19.0, 15.0, 33.0, 25.0],
        "origin": ["EU", "US", "US", "JP", "US", "US", "JP", "EU"],
    }
)


def full_plot() -> PlotSpec:
    """A plot touching every accessor."""
    return (
        ggplot(CARS, aes("weight", "mpg"))
        + geom_point(aes(color="origin"), size=3, alpha=0.5)
        + geom_line(data={"weight": [2, 4], "mpg": [30, 16]})
        + scale_color_brewer(palette="Set2")
        + theme_bw()
        + theme(legend_position="bottom")
        + labs(title="Fuel use", subtitle="Eight cars", caption="Made up", x="Weight")
        + ggsize(700, 300)
        + facet_wrap("origin", ncol=2)
        + coord_flip()
    )


# PlotSpec


def test_plot_data_is_the_same_object() -> None:
    plot = full_plot()
    assert plot.data is CARS
    assert plot.get_plot_shared_data() is CARS
    assert ggplot().data is None


def test_plot_mapping_and_layers() -> None:
    plot = full_plot()
    assert plot.mapping == {"x": "weight", "y": "mpg"}
    assert ggplot().mapping == {}
    assert isinstance(plot.layers, tuple)
    assert [layer.geom for layer in plot.layers] == ["point", "line"]
    assert all(isinstance(layer, LayerSpec) for layer in plot.layers)
    point, line = plot.layers
    assert point.mapping == {"color": "origin"}
    assert point.params == {"size": 3, "alpha": 0.5}
    assert point.data is None
    assert line.data == {"weight": [2, 4], "mpg": [30, 16]}
    assert line.data is line.get_plot_layer_data()
    assert point.stat is None


def test_plot_mapping_after_adding_aes() -> None:
    plot = ggplot(CARS, aes("weight")) + aes(y="mpg")
    assert plot.mapping == {"x": "weight", "y": "mpg"}


def test_layer_stat_and_position_params() -> None:
    layer = geom_point(stat="sum", position="jitter")
    assert layer.stat == "sum"
    assert layer.params["position"] == "jitter"


def test_boxplot_layers_match_as_dict() -> None:
    plot = ggplot(CARS, aes("origin", "mpg")) + geom_boxplot()
    assert [layer.geom for layer in plot.layers] == [
        layer["geom"] for layer in plot.as_dict()["layers"]
    ]
    assert len(plot.layers) == 2


def test_plot_scales_theme_labels_size_facet_coord() -> None:
    plot = full_plot()
    assert plot.scales[0]["aesthetic"] == "color"
    assert plot.scales[0]["palette"] == "Set2"
    assert plot.theme == {"name": "bw", "legend_position": "bottom"}
    assert plot.labels == {
        "title": "Fuel use",
        "subtitle": "Eight cars",
        "caption": "Made up",
        "x": "Weight",
    }
    assert plot.size == (700, 300)
    assert plot.facet is not None
    assert plot.facet["facets"] == "origin"
    assert plot.facet["ncol"] == 2
    assert plot.coord == {"name": "flip", "flip": True}
    assert plot.guides == {"x": {"title": "Weight"}}
    empty = ggplot()
    assert (empty.scales, empty.theme, empty.labels, empty.size) == ((), {}, {}, None)
    assert (empty.facet, empty.coord, empty.guides) == (None, None, {})


def test_labels_keep_scale_group_keys() -> None:
    plot = (
        ggplot()
        + labs(color="First")
        + guides(fill="none")
        + new_scale_color()
        + labs(color="Second")
    )
    assert plot.labels == {"color": "First", "color:1": "Second"}


def test_accessors_return_fresh_containers() -> None:
    plot = full_plot()
    plot.mapping["x"] = "mpg"
    plot.theme["name"] = "grey"
    plot.labels["title"] = "Changed"
    plot.guides["x"] = "none"
    plot.layers[0].params["size"] = 10
    plot.layers[0].mapping["color"] = "weight"
    assert plot.as_dict() == full_plot().as_dict() | {"data": CARS}
    assert plot.mapping == {"x": "weight", "y": "mpg"}
    assert plot.labels["title"] == "Fuel use"
    assert plot.layers[0].params["size"] == 3


def test_adding_does_not_change_the_base_plot() -> None:
    base = ggplot(CARS, aes("weight", "mpg")) + geom_point()
    before = base.as_dict()
    bigger = base + geom_line() + theme_bw() + ggtitle("Title") + ggsize(300, 200)
    assert base.as_dict() == before
    assert len(base.layers) == 1
    assert len(bigger.layers) == 2
    assert (base.theme, base.labels, base.size) == ({}, {}, None)


def test_with_data_returns_a_new_plot() -> None:
    plot = full_plot()
    trucks = CARS.with_columns(pl.col("weight") * 2)
    truck_plot = plot.with_data(trucks)
    assert truck_plot is not plot
    assert truck_plot.data is trucks
    assert plot.data is CARS
    assert truck_plot.layers == plot.layers
    assert truck_plot.labels == plot.labels
    assert truck_plot.scales == plot.scales
    without = plot.with_data(None)
    assert without.data is None
    assert "data" not in without.as_dict()


def test_with_data_keeps_scale_groups() -> None:
    plot = ggplot() + new_scale_color() + scale_color_brewer()
    copied = plot.with_data(CARS) + geom_point(aes(color="origin"))
    assert copied.layers[0].params["scale_group"] == {"color": 1}


def test_accessors_copy_nested_lists() -> None:
    plot = (
        ggplot()
        + scale_x_continuous(breaks=[1, 2, 3])
        + geom_point(tooltips=layer_tooltips().line("@x"))
        + coord_cartesian(xlim=[0, 1])
        + facet_wrap("origin", order=[1, 2])
    )
    before = plot.as_dict()
    plot.scales[0]["breaks"].append(99)
    plot.layers[0].params["tooltips"]["lines"].append("z")
    plot.coord["xlim"].append(5)  # ty: ignore[not-subscriptable]
    plot.facet["order"].append(3)  # ty: ignore[not-subscriptable]
    assert plot.as_dict() == before
    grid = gggrid([ggplot(), ggplot()], widths=[1, 2])
    grid.layout["widths"].append(9)
    assert grid.layout["widths"] == [1, 2]


def test_with_data_refreshes_data_meta() -> None:
    dates = pl.DataFrame(
        {"day": [datetime.date(2020, month, 1) for month in range(1, 13)], "v": range(12)}
    )
    swapped = (ggplot(None, aes("day", "v")) + geom_point()).with_data(dates)
    direct = ggplot(dates, aes("day", "v")) + geom_point()
    assert swapped.as_dict() == direct.as_dict()
    assert swapped.build()["panels"] == direct.build()["panels"]


# layer_data and build


def histogram() -> PlotSpec:
    return ggplot(CARS, aes("mpg")) + geom_histogram(bins=4)


def test_histogram_layer_data() -> None:
    frame = histogram().layer_data(0)
    assert isinstance(frame, pl.DataFrame)
    assert frame.columns[:2] == ["panel", "group"]
    assert frame.height == 4
    assert frame["count"].sum() == CARS.height
    assert frame["count"].to_list() == frame["y"].to_list()
    # Density integrates to 1 over the bins.
    widths = frame["xmax"] - frame["xmin"]
    assert (frame["density"] * widths).sum() == pytest.approx(1.0)
    assert histogram().layer_data(-1).equals(frame)


def bar_rects(svg: str) -> list[tuple[float, float, float, float]]:
    """``(x, y, width, height)`` of every rect after the background and panel."""
    rects = re.findall(r"<rect[^>]*>", svg)[2:]
    found = []
    for rect in rects:
        attributes = dict(re.findall(r' (x|y|width|height)="([^"]*)"', rect))
        found.append(tuple(float(attributes[key]) for key in ("x", "y", "width", "height")))
    return sorted(found)


def test_histogram_layer_data_matches_rendered_bars() -> None:
    plot = histogram()
    frame = plot.layer_data()
    bars = bar_rects(cast_str(plot.to_svg()))
    replot = ggplot(frame) + geom_rect(aes(xmin="xmin", xmax="xmax", ymin="ymin", ymax="ymax"))
    replotted = bar_rects(cast_str(replot.to_svg()))
    assert len(bars) == frame.height
    # Same bins on x; y differs only by the histogram's zero-based axis.
    assert [bar[0::2] for bar in bars] == [bar[0::2] for bar in replotted]
    heights = [bar[3] for bar in bars]
    counts = frame.sort("xmin")["count"].to_list()
    ratios = [height / count for height, count in zip(heights, counts, strict=True)]
    assert max(ratios) == pytest.approx(min(ratios), rel=1e-3)


def cast_str(value: str | None) -> str:
    assert value is not None
    return value


def test_density_layer_data() -> None:
    frame = (ggplot(CARS, aes("mpg", fill="origin")) + geom_density()).layer_data()
    assert set(frame["group"].unique().to_list()) == {0, 1, 2}
    assert frame["fill"].dtype == pl.Categorical
    assert set(frame["fill"].unique().to_list()) == {"EU", "US", "JP"}
    for (origin,), group in frame.group_by("fill"):
        values = np.array(CARS.filter(pl.col("origin") == origin)["mpg"].to_list())
        # R's bw.nrd0 and a Gaussian kernel, evaluated where the engine evaluated.
        quartiles = np.quantile(values, [0.25, 0.75])
        spread = min(values.std(ddof=1), (quartiles[1] - quartiles[0]) / 1.34)
        bandwidth = 0.9 * spread * len(values) ** -0.2
        x = group["x"].to_numpy()
        kernel = np.exp(-0.5 * ((x[:, None] - values[None, :]) / bandwidth) ** 2)
        expected = kernel.sum(axis=1) / (len(values) * bandwidth * np.sqrt(2 * np.pi))
        assert group["density"].to_numpy() == pytest.approx(expected, rel=1e-6)


def test_stat_variables_copied_by_default_mapping_appear_once() -> None:
    columns = (ggplot(CARS, aes("origin", "mpg")) + geom_violin()).layer_data().columns
    assert "violinwidth" in columns
    assert not [name for name in columns if name.startswith("..")]


def test_layer_data_of_each_layer_and_facets() -> None:
    plot = (
        ggplot(CARS, aes("weight", "mpg"))
        + geom_point()
        + geom_line(data={"weight": [2, 4], "mpg": [30, 16]})
        + facet_wrap("origin")
    )
    built = plot.build()
    assert len(built["layers"]) == 2
    assert len(built["panels"]) == 3
    points = built["layers"][0]
    assert points.height == CARS.height
    assert sorted(points["panel"].unique().to_list()) == [0, 1, 2]
    assert plot.layer_data(1).height == 2 * 3  # the line data repeats in each panel


def test_build_discrete_and_log_scales() -> None:
    plot = ggplot(CARS, aes("origin", "weight")) + geom_point()
    panel = plot.build()["panels"][0]
    assert panel["x"]["discrete"] is True
    assert panel["x"]["labels"] == ["EU", "US", "JP"]
    assert panel["x"]["breaks"] == [0.0, 1.0, 2.0]
    assert set(plot.layer_data()["x"].to_list()) == {0.0, 1.0, 2.0}
    logged = ggplot(CARS, aes("weight", "mpg")) + geom_point() + scale_x_log10()
    x = logged.build()["panels"][0]["x"]
    assert x["discrete"] is False
    low, high = x["domain"]
    assert low < 0.5 < high < 0.7  # log10 of 2.2 .. 4.1
    assert len(x["breaks"]) == len(x["labels"])


def test_build_keeps_data_axes_under_coord_flip() -> None:
    plot = ggplot(CARS, aes("origin", "mpg")) + geom_point()
    flipped = plot + coord_flip()
    assert plot.build()["panels"][0]["x"] == flipped.build()["panels"][0]["x"]


def test_layer_data_index_errors() -> None:
    plot = histogram()
    with pytest.raises(PlotIndexError, match=r"layer index 1 is out of range.*1 layer;") as info:
        plot.layer_data(1)
    assert isinstance(info.value, IndexError)
    assert (info.value.index, info.value.length) == (1, 1)
    with pytest.raises(PlotIndexError, match="no layers"):
        ggplot(CARS).layer_data(0)
    with pytest.raises(TypeError, match="layer index must be an int"):
        plot.layer_data("count")  # ty: ignore[invalid-argument-type]
    with pytest.raises(TypeError, match="must be an int"):
        plot.layer_data(True)


def test_layer_data_reports_spec_errors() -> None:
    plot = ggplot(CARS, aes("mpgg")) + geom_histogram()
    with pytest.raises(ColumnNotFoundError, match="did you mean 'mpg'"):
        plot.layer_data()


# SupPlotsSpec


def points() -> PlotSpec:
    return ggplot(CARS, aes("weight", "mpg")) + geom_point()


def test_grid_figures_layout_and_cells() -> None:
    first, second, third = points(), histogram(), points()
    inner = gggrid([second, third])
    grid = gggrid([first, None, inner], ncol=2, guides="collect")
    assert grid.figures == (first, None, inner, None)
    assert (grid.ncol, grid.nrow) == (2, 2)
    assert grid.layout["guides"] == "collect"
    assert grid.layout["widths"] is None
    assert "name" not in grid.layout
    assert grid[0] is first
    assert grid[1] is None
    assert grid[1, 0] is inner
    assert grid[-1, -2] is inner
    assert grid[-4] is first
    assert list(grid) == list(grid.figures)


def test_grid_without_ncol_is_one_row() -> None:
    grid = gggrid([points(), points(), points()])
    assert (grid.ncol, grid.nrow) == (3, 1)


def test_grid_plots_flatten_nested_grids() -> None:
    first, second, third = points(), histogram(), points()
    grid = gggrid([first, None, gggrid([None, second, gggrid([third])])], ncol=2)
    assert grid.plots() == (first, second, third)
    assert list(grid.plot_paths().items()) == [
        ("figures[0]", first),
        ("figures[2].figures[1]", second),
        ("figures[2].figures[2].figures[0]", third),
    ]
    assert gggrid([None]).plots() == ()


def test_grid_size_theme_labels() -> None:
    plot = points() + theme_bw()
    grid = gggrid([plot]) + ggsize(800, 400) + theme(legend_position="top")
    grid = grid + labs(title="Dashboard", caption="Source", tag="A")
    assert grid.size == (800, 400)
    assert grid.theme == {"legend_position": "top"}
    assert grid.labels == {"title": "Dashboard", "caption": "Source", "tag": "A"}
    # Figures keep the theme they were built with.
    assert grid[0] is plot
    assert plot.theme == {"name": "bw"}
    bare = gggrid([plot])
    assert (bare.size, bare.theme, bare.labels) == (None, {}, {})


def test_grid_index_errors() -> None:
    grid = gggrid([points(), points(), points()], ncol=2)
    with pytest.raises(PlotIndexError, match=r"grid cell 4 is out of range.*4 cells \(2 x 2\)"):
        grid[4]
    with pytest.raises(PlotIndexError, match=r"\(2, 0\) is out of range.*2 rows and 2 columns"):
        grid[2, 0]
    with pytest.raises(PlotIndexError) as info:
        grid[0, 5]
    assert (info.value.index, info.value.length) == ((0, 5), 4)
    for cell in ["a", (0,), (0, 1, 2), (0, "1"), True, 1.0]:
        with pytest.raises(TypeError, match="a grid cell is an int or a"):
            grid[cell]  # ty: ignore[invalid-argument-type]


def test_with_figure() -> None:
    first, line = points(), ggplot() + geom_line()
    grid = gggrid([first, first, first], ncol=2) + ggtitle("Title")
    changed = grid.with_figure((1, 1), line)
    assert changed[3] is line
    assert grid[3] is None
    assert changed.labels == {"title": "Title"}
    emptied = changed.with_figure(0, None)
    assert emptied[0] is None
    assert changed[0] is first
    nested = grid.with_figure(-1, gggrid([line]))
    assert nested.plots() == (first, first, first, line)
    with pytest.raises(PlotIndexError):
        grid.with_figure(9, line)
    with pytest.raises(TypeError, match="figure must be a plot"):
        grid.with_figure(0, geom_point())  # ty: ignore[invalid-argument-type]


def test_setitem_changes_one_cell_in_place() -> None:
    first, line = points(), ggplot() + geom_line()
    inner = gggrid([first, first])
    grid = gggrid([first, first, inner], ncol=2) + ggtitle("Title")
    before = SupPlotsSpec.duplicate(grid)
    grid[0] += geom_line()  # ty: ignore[unsupported-operator]
    assert [layer.geom for layer in grid[0].layers] == ["point", "line"]  # ty: ignore[unresolved-attribute]
    assert first.layers[-1].geom == "point"  # the old figure is not changed
    assert before[0] is first  # earlier copies are not changed
    grid[1, 1] = line
    assert grid[3] is line
    grid[-1] = None
    assert grid[3] is None
    grid[2][1] += geom_line()  # nested grid, changed in place through its cell
    assert grid[2] is inner and [layer.geom for layer in inner[1].layers] == ["point", "line"]
    assert grid.labels == {"title": "Title"}
    assert grid.as_dict()["figures"][3] is None
    assert grid.layout == before.with_figure(3, None).layout
    with pytest.raises(PlotIndexError):
        grid[9] = line
    with pytest.raises(TypeError, match="grid\\[cell\\] = figure: figure must be a plot"):
        grid[0] = geom_point()
    with pytest.raises(TypeError, match="a grid cell is an int or a"):
        grid["a"] = line


def test_with_layout() -> None:
    first = points()
    grid = gggrid([first, first, first], ncol=2) + ggsize(500, 300) + ggtitle("T")
    wide = grid.with_layout(ncol=3, guides="collect")
    assert (wide.ncol, wide.nrow) == (3, 1)
    assert wide.figures == (first, first, first)
    assert wide.layout["guides"] == "collect"
    assert (wide.size, wide.labels) == ((500, 300), {"title": "T"})
    assert (grid.ncol, grid.layout["guides"]) == (2, None)
    tall = wide.with_layout(ncol=1)
    assert (tall.ncol, tall.nrow) == (1, 3)
    spaced = grid.with_layout(hspace=10, sharex=True)
    assert spaced.figures == grid.figures
    assert spaced.layout["sharex"] == "all"
    assert spaced.as_dict()["layout"]["hspace"] == 10
    with pytest.raises(TypeError, match=r"unexpected option 'ncols'.*did you mean 'ncol'"):
        grid.with_layout(ncols=3)
    with pytest.raises(TypeError, match="unexpected option 'nrow'"):
        grid.with_layout(nrow=3)
    for ncol in [0, -1, True, 1.5]:
        with pytest.raises(SpecError, match="ncol must be a positive int") as info:
            grid.with_layout(ncol=ncol)
        assert info.value.path == "layout.ncol"


def test_grid_renders_after_with_changes() -> None:
    grid = gggrid([points(), None], ncol=2).with_figure(1, histogram())
    svg = grid.with_layout(ncol=1).to_svg()
    assert svg is not None and svg.startswith("<svg")


def test_grid_build_is_per_plot() -> None:
    grid = gggrid([points(), histogram()])
    assert [plot.build()["layers"][0].height for plot in grid.plots()] == [8, 4]
    assert isinstance(grid, SupPlotsSpec)
    assert not hasattr(grid, "build")


MEMBERS = [
    (cls, name)
    for cls in (PlotSpec, LayerSpec, SupPlotsSpec)
    for name in [
        "data",
        "mapping",
        "layers",
        "scales",
        "theme",
        "labels",
        "size",
        "facet",
        "coord",
        "guides",
        "with_data",
        "layer_data",
        "build",
        "geom",
        "stat",
        "params",
        "figures",
        "layout",
        "ncol",
        "nrow",
        "plots",
        "plot_paths",
        "with_figure",
        "with_layout",
    ]
    if name in vars(cls)
]


@pytest.mark.parametrize(
    ("cls", "name"), MEMBERS, ids=lambda value: getattr(value, "__name__", value)
)
def test_member_docstring_examples(cls: type, name: str) -> None:
    """Every accessor documents an example, and it passes as a doctest."""
    member = vars(cls)[name]
    docstring = inspect.getdoc(member.fget if isinstance(member, property) else member)
    assert docstring and ">>>" in docstring, f"{cls.__name__}.{name} has no example"
    test = doctest.DocTestParser().get_doctest(docstring, {}, f"{cls.__name__}.{name}", None, 0)
    report: list[str] = []
    result = doctest.DocTestRunner().run(test, out=report.append)
    assert result.failed == 0, "".join(report)
