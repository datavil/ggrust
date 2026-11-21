"""Grid slices (plan step 14.1): ``grid[:] += feature`` through a ``GridCells`` view.

Decisions (user, 2026-11-21): nested grids recurse, figure-level features apply per
plot, design areas spanning cells count once and ``#`` cells are skipped, the view is
read-only iterable, and every slice ``slice.indices`` accepts is accepted.
"""

from __future__ import annotations

from typing import Any

import pytest
from ggrust import (
    GridCells,
    SupPlotsSpec,
    aes,
    geom_line,
    geom_point,
    geom_smooth,
    gggrid,
    gghighlight,
    ggplot,
    ggsize,
    ggtitle,
    labs,
    scale_color_viridis,
    theme,
)
from ggrust.errors import PlotIndexError
from ggrust.plot.core import PlotSpec

DATA = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}


def points() -> PlotSpec:
    return ggplot(DATA, aes("weight", "mpg")) + geom_point()


def geoms(figure: object) -> list[str]:
    assert isinstance(figure, PlotSpec)
    return [str(layer.geom) for layer in figure.layers]


def grid_of_four() -> SupPlotsSpec:
    return gggrid([points() for _ in range(4)], ncol=2)


def changed_positions(before: SupPlotsSpec, after: SupPlotsSpec) -> list[int]:
    """The figure indices whose ``as_dict()`` differs."""
    old: list[Any] = before.as_dict()["figures"]
    new: list[Any] = after.as_dict()["figures"]
    return [index for index, (a, b) in enumerate(zip(old, new, strict=True)) if a != b]


@pytest.mark.parametrize(
    ("key", "expected"),
    [
        (slice(None), [0, 1, 2, 3]),
        ((0, slice(None)), [0, 1]),
        ((-1, slice(None)), [2, 3]),
        ((slice(None), 1), [1, 3]),
        ((slice(None), slice(None)), [0, 1, 2, 3]),
        (slice(1, 3), [1, 2]),
        (slice(-2, None), [2, 3]),
        (slice(None, None, 2), [0, 2]),
        (slice(None, None, -1), [0, 1, 2, 3]),
        ((slice(None, None, -1), 0), [0, 2]),
        (slice(9, None), []),
    ],
)
def test_slice_adds_to_exactly_the_selected_plots(key: Any, expected: list[int]) -> None:
    grid = grid_of_four()
    before = SupPlotsSpec.duplicate(grid)
    grid[key] += geom_smooth()
    assert changed_positions(before, grid) == expected
    for index in expected:
        assert geoms(grid[index]) == ["point", "smooth"]


def test_int_and_pair_indexing_unchanged() -> None:
    grid = grid_of_four()
    first = grid.figures[0]
    assert grid[0] is first and grid[0, 0] is first and grid[-1] is grid.figures[3]
    grid[1] += geom_line()  # ty: ignore[unsupported-operator]
    assert geoms(grid[1]) == ["point", "line"]


def test_view_repr_len_iter() -> None:
    grid = grid_of_four()
    view = grid[0, :]
    assert isinstance(view, GridCells)
    assert repr(view) == "GridCells(grid[0, :]: figures [0, 1] of a 2 x 2 grid)"
    assert repr(grid[1:3]) == "GridCells(grid[1:3]: figures [1, 2] of a 2 x 2 grid)"
    assert repr(grid[::2]) == "GridCells(grid[::2]: figures [0, 2] of a 2 x 2 grid)"
    assert len(view) == 2
    assert list(view) == [grid[0], grid[1]]
    assert len(grid[5:]) == 0


def test_view_add_does_not_change_the_grid() -> None:
    grid = grid_of_four()
    before = SupPlotsSpec.duplicate(grid)
    view = grid[:] + geom_line()
    assert isinstance(view, GridCells)
    assert changed_positions(before, grid) == []
    assert all(geoms(figure) == ["point", "line"] for figure in view)


def test_empty_cells_stay_empty() -> None:
    grid = gggrid([points(), None, points()], ncol=2)
    grid[:] += geom_line()
    assert grid.figures[1] is None and grid.figures[3] is None
    assert [geoms(plot) for plot in grid.plots()] == [["point", "line"]] * 2


def test_shared_plot_objects_are_not_changed() -> None:
    shared = points()
    grid = gggrid([shared, shared], ncol=2)
    copied = SupPlotsSpec.duplicate(grid)
    grid[:] += geom_line()
    assert geoms(shared) == ["point"]
    assert copied[0] is shared and copied[1] is shared
    assert grid[0] is not grid[1]


def test_nested_grids_recurse() -> None:
    inner = gggrid([points(), None, points()], ncol=2)
    grid = gggrid([points(), inner], ncol=2)
    grid[:] += geom_line()
    assert [geoms(plot) for plot in grid.plots()] == [["point", "line"]] * 3
    assert geoms(inner[0]) == ["point"]  # the nested grid object itself is copied
    nested = grid[1]
    assert isinstance(nested, SupPlotsSpec) and nested is not inner
    assert nested.figures[1] is None and nested.layout == inner.layout


def test_figure_level_features_apply_per_plot() -> None:
    grid = grid_of_four() + ggtitle("Grid")
    grid[0, :] += theme(legend_position="none")
    grid[0, :] += ggtitle("Plot")
    assert grid.theme == {} and grid.labels == {"title": "Grid"}
    top = [grid[0], grid[1]]
    assert all(isinstance(plot, PlotSpec) for plot in top)
    assert [plot.theme for plot in top] == [{"legend_position": "none"}] * 2  # ty: ignore[unresolved-attribute]
    assert [plot.labels.get("title") for plot in top] == ["Plot"] * 2  # ty: ignore[unresolved-attribute]
    assert grid[2].labels == {} and grid[2].theme == {}  # ty: ignore[unresolved-attribute]


def test_feature_kinds_and_feature_lists() -> None:
    grid = grid_of_four()
    grid[:] += scale_color_viridis() + labs(x="weight (1000 lbs)")
    for plot in grid.plots():
        assert [scale["aesthetic"] for scale in plot.scales] == ["color"]
        assert plot.labels["x"] == "weight (1000 lbs)"
    grid[:] += ggsize(300, 200)
    assert grid.size is None and all(plot.size == (300, 200) for plot in grid.plots())


def test_design_areas_once_and_hash_cells_skipped() -> None:
    plots = [points(), points(), points()]
    grid = gggrid(plots, design="AB\nCC")
    assert repr(grid[1, :]) == "GridCells(grid[1, :]: figures [2] of a 2 x 2 grid)"
    grid[1, :] += geom_line()
    assert geoms(grid[2]) == ["point", "line"]
    grid[:, :] += geom_smooth()
    assert [geoms(plot) for plot in grid.plots()] == [
        ["point", "smooth"],
        ["point", "smooth"],
        ["point", "line", "smooth"],
    ]
    holes = gggrid([points(), points()], design="b#\nba")
    assert list(holes[0, :]) == [holes[1]]
    holes[0, :] += geom_line()
    assert geoms(holes[1]) == ["point", "line"] and geoms(holes[0]) == ["point"]
    with pytest.raises(PlotIndexError, match="empty cell"):
        holes[0, 1]


def test_rendered_grid_after_slice_matches_the_list_form() -> None:
    grid = grid_of_four()
    grid[:] += geom_smooth()
    by_hand = gggrid([points() + geom_smooth() for _ in range(4)], ncol=2)
    assert grid.as_dict() == by_hand.as_dict()
    assert grid.to_svg() == by_hand.to_svg()


# Errors


def test_assigning_anything_but_a_view_to_a_slice() -> None:
    grid = grid_of_four()
    with pytest.raises(TypeError) as info:
        grid[:] = points()  # ty: ignore[invalid-assignment]
    message = str(info.value)
    assert message.startswith("grid[:] = value: a slice of a grid takes the GridCells view")
    assert "got PlotSpec" in message and "grid[:] += feature" in message


def test_assigning_a_view_to_one_cell() -> None:
    grid = grid_of_four()
    with pytest.raises(
        TypeError, match=r"one cell takes a figure, got a GridCells view \(grid\[:\]\)"
    ):
        grid[0] = grid[:]  # ty: ignore[invalid-assignment]


def test_assigning_a_view_of_other_cells() -> None:
    grid = grid_of_four()
    with pytest.raises(TypeError) as info:
        grid[0, :] = grid[:] + geom_line()
    message = str(info.value)
    assert "grid[0, :] = view" in message
    assert "figures [0, 1, 2, 3] (grid[:]), not figures [0, 1]" in message
    assert all(geoms(plot) == ["point"] for plot in grid.plots())


def test_assigning_a_view_of_another_grid() -> None:
    grid, other = grid_of_four(), grid_of_four()
    with pytest.raises(TypeError, match=r"the view \(grid\[:\]\) was taken from another grid"):
        grid[:] = other[:]


def test_index_types_as_slice_bounds() -> None:
    np = pytest.importorskip("numpy")
    grid = grid_of_four()
    assert list(grid[np.int64(1) : 3]) == [grid[1], grid[2]]
    with pytest.raises(TypeError, match="slice bounds and step must be ints"):
        grid[True:]


def test_stale_view_is_refused() -> None:
    grid = grid_of_four()
    replacement = points()
    view = grid[:] + geom_line()
    grid[0] = replacement
    with pytest.raises(TypeError, match=r"grid\[:\] = view: figures \[0\] changed after"):
        grid[:] = view
    assert grid[0] is replacement and geoms(grid[1]) == ["point"]


def test_bad_slice_parts() -> None:
    grid = grid_of_four()
    with pytest.raises(TypeError, match=r"grid\['a':\]: slice bounds and step must be ints"):
        grid["a":]
    with pytest.raises(TypeError, match=r"slice bounds and step must be ints or None"):
        grid[0, 1.0:]
    with pytest.raises(ValueError, match=r"grid\[::0\]: slice step cannot be zero"):
        grid[::0]
    for key in [(slice(None), "1"), (slice(None), 0, 1), (slice(None),), (True, slice(None))]:
        with pytest.raises(TypeError, match="several cells are a slice"):
            grid[key]  # ty: ignore[invalid-argument-type]


def test_int_part_out_of_range() -> None:
    grid = grid_of_four()
    with pytest.raises(
        PlotIndexError, match=r"grid\[5, :\]: row 5 is out of range.*2 rows"
    ) as info:
        grid[5, :]
    assert (info.value.index, info.value.length) == (5, 2)
    with pytest.raises(PlotIndexError, match=r"grid\[:, -3\]: column -3 is out of range"):
        grid[:, -3]


def test_unsupported_operands() -> None:
    grid = grid_of_four()
    view = grid[:]
    for operand in [3, points(), grid]:
        with pytest.raises(TypeError, match="unsupported operand"):
            view + operand


def test_plot_error_names_the_cell_path() -> None:
    empty = ggplot()
    grid = gggrid([points(), gggrid([points(), empty])], ncol=2)
    with pytest.raises(ValueError, match="no layers and no data") as info:
        grid[:] += gghighlight("mpg > 20")
    assert "in grid[:] + gghighlight: adding to the plot at figures[1].figures[1]" in (
        info.value.__notes__
    )
    assert geoms(grid[0]) == ["point"]  # nothing was written back
