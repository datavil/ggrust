"""Dendrogram leaves that span a run of levels (``scale_*_dendro(leaf_levels=)``).

A tree over clusters drawn beside a per-cell heatmap: each leaf stands for the
cells of its cluster. Checked on the drawn SVG: the axis is the runs end to end in
the tree's order, each run in its given order, every leaf sits at the centre of
its run (on the axis side, on the opposite side, reversed, under ``coord_flip``
and in an aligned ``gggrid``), and bad mappings raise typed errors.
"""

from __future__ import annotations

import itertools
import re

import ggrust as gg
import polars as pl
import pytest
from ggrust.errors import SpecError, UnsupportedValueError

PROFILES = {
    "T": [5.0, 1.0, 0.2],
    "NK": [4.0, 1.5, 0.3],
    "B": [0.2, 6.0, 0.1],
    "Mono": [0.1, 0.5, 7.0],
}
SIZES = {"T": 4, "NK": 2, "B": 3, "Mono": 2}
CLUSTERS = pl.DataFrame(
    {
        "cluster": list(PROFILES),
        **{
            gene: [profile[index] for profile in PROFILES.values()]
            for index, gene in enumerate(["CD3E", "MS4A1", "LYZ"])
        },
    }
)
TREE = gg.hclust(CLUSTERS, labels="cluster")
RUNS = {cluster: [f"{cluster}-{index}" for index in range(n)] for cluster, n in SIZES.items()}
CELLS = pl.DataFrame(
    [
        {"cell": cell, "gene": gene, "value": PROFILES[cluster][index] + 0.1 * position}
        for cluster, cells in RUNS.items()
        for position, cell in enumerate(cells)
        for index, gene in enumerate(["CD3E", "MS4A1", "LYZ"])
    ]
)


def _axis_group(svg: str, axis: str) -> str:
    match = re.search(rf'<g class="axis-{axis}">(.*?)</g>', svg, re.S)
    assert match is not None
    return match.group(1)


def _tree_points(svg: str, axis: str) -> list[tuple[float, float]]:
    """Every point of the ``dendrogram-{axis}`` group's paths."""
    match = re.search(rf'<g class="dendrogram-{axis}"[^>]*>(.*?)</g>', svg, re.S)
    assert match is not None, f"no dendrogram-{axis} group"
    points = []
    for data in re.findall(r'<path d="(M[-\d. L]+)"', match.group(1)):
        numbers = [float(value) for value in re.findall(r"-?\d+(?:\.\d+)?", data)]
        points.extend(zip(numbers[::2], numbers[1::2], strict=True))
    return points


def _y_labels(svg: str) -> dict[str, float]:
    """Y axis label -> text baseline."""
    labels = re.findall(r'y="([\d.]+)"[^>]*>([^<]+)</text>', _axis_group(svg, "y"))
    return {name: float(y) for y, name in labels}


def _x_labels(svg: str) -> dict[str, float]:
    """X axis label -> text x (labels are centred)."""
    labels = re.findall(r'<text x="([\d.]+)"[^>]*>([^<]+)</text>', _axis_group(svg, "x"))
    return {name: float(x) for x, name in labels}


def _leaf_ends(points: list[tuple[float, float]], along: int) -> list[float]:
    """Along-axis pixels of the leaf ends (the points nearest the leaves' baseline)."""
    across = 1 - along
    values = [point[across] for point in points]
    baseline = max(values, key=lambda value: values.count(value))
    return sorted({round(point[along], 1) for point in points if point[across] == baseline})


def _heatmap_y(**scale: object) -> gg.PlotSpec:
    return (
        gg.ggplot(CELLS, gg.aes("gene", "cell", fill="value"))
        + gg.geom_tile()
        + gg.scale_y_dendro(TREE, leaf_levels=RUNS, **scale)
    )


def _run_centres(positions: dict[str, float], runs: dict[str, list[str]]) -> list[float]:
    return sorted(
        round((positions[cells[0]] + positions[cells[-1]]) / 2, 1) for cells in runs.values()
    )


def test_axis_is_the_runs_in_tree_order_and_leaves_sit_at_run_centres() -> None:
    svg = _heatmap_y().to_svg()
    labels = _y_labels(svg)
    bottom_to_top = [name for name, _ in sorted(labels.items(), key=lambda item: -item[1])]
    expected = list(itertools.chain.from_iterable(RUNS[leaf] for leaf in TREE.ordered_labels))
    assert bottom_to_top == expected
    # Text baselines sit 4.5 px below the tick.
    ticks = {name: y - 4.5 for name, y in labels.items()}
    leaves = _leaf_ends(_tree_points(svg, "y"), along=1)
    assert leaves == pytest.approx(_run_centres(ticks, RUNS), abs=1.5)


def test_levels_keep_their_given_order_inside_a_run() -> None:
    runs = {**RUNS, "T": list(reversed(RUNS["T"]))}
    plot = (
        gg.ggplot(CELLS, gg.aes("gene", "cell", fill="value"))
        + gg.geom_tile()
        + gg.scale_y_dendro(TREE, leaf_levels=runs)
    )
    labels = _y_labels(plot.to_svg())
    t_bottom_to_top = [
        name for name, _ in sorted(labels.items(), key=lambda item: -item[1]) if name[0] == "T"
    ]
    assert t_bottom_to_top == runs["T"]


def test_opposite_band_on_x() -> None:
    plot = (
        gg.ggplot(CELLS, gg.aes("cell", "gene", fill="value"))
        + gg.geom_tile()
        + gg.scale_x_dendro(TREE, leaf_levels=RUNS, guide=gg.guide_axis_dendro(side="opposite"))
    )
    svg = plot.to_svg()
    points = _tree_points(svg, "x")
    leaves = _leaf_ends(points, along=0)
    assert leaves == pytest.approx(_run_centres(_x_labels(svg), RUNS), abs=0.5)
    # Above the panel: every point is above the top row's gene label.
    assert max(y for _, y in points) < min(_y_labels(svg).values()) - 20


def test_reverse_flips_the_runs_and_the_tree() -> None:
    svg = _heatmap_y(reverse=True).to_svg()
    labels = _y_labels(svg)
    top_to_bottom = [name for name, _ in sorted(labels.items(), key=lambda item: item[1])]
    expected = list(itertools.chain.from_iterable(RUNS[leaf] for leaf in TREE.ordered_labels))
    assert top_to_bottom == expected
    ticks = {name: y - 4.5 for name, y in labels.items()}
    leaves = _leaf_ends(_tree_points(svg, "y"), along=1)
    assert leaves == pytest.approx(_run_centres(ticks, RUNS), abs=1.5)


def test_coord_flip_draws_the_tree_beside_the_vertical_axis() -> None:
    plot = (
        gg.ggplot(CELLS, gg.aes("cell", "gene", fill="value"))
        + gg.geom_tile()
        + gg.scale_x_dendro(TREE, leaf_levels=RUNS)
        + gg.coord_flip()
    )
    svg = plot.to_svg()
    labels = _y_labels(svg)
    assert set(labels) == {cell for cells in RUNS.values() for cell in cells}
    ticks = {name: y - 4.5 for name, y in labels.items()}
    points = _tree_points(svg, "x") if 'class="dendrogram-x"' in svg else _tree_points(svg, "y")
    leaves = _leaf_ends(points, along=1)
    assert leaves == pytest.approx(_run_centres(ticks, RUNS), abs=1.5)


def test_aligned_grid_keeps_panels_in_line() -> None:
    border = gg.theme(panel_border=gg.element_rect(color="red"))
    heatmap = _heatmap_y() + gg.theme(axis_text_y=gg.element_blank()) + border
    bars = gg.ggplot(CELLS, gg.aes("gene", "value")) + gg.geom_bar(stat="identity") + border
    svg = gg.gggrid([heatmap, bars], ncol=1, align=True).to_svg()
    panels = re.findall(
        r'stroke="#ff0000"[^>]*>\s*<rect x="([\d.]+)" y="[\d.]+" width="([\d.]+)"', svg
    )
    assert len(panels) == 2
    # Both panels share left and right edges; the tree sits left of them.
    assert panels[0] == panels[1]
    assert max(x for x, _ in _tree_points(svg, "y")) < float(panels[0][0])


def test_run_levels_missing_from_the_data_keep_their_slot() -> None:
    runs = {**RUNS, "NK": [*RUNS["NK"], "NK-unseen"]}
    plot = (
        gg.ggplot(CELLS, gg.aes("gene", "cell", fill="value"))
        + gg.geom_tile()
        + gg.scale_y_dendro(TREE, leaf_levels=runs)
    )
    assert "NK-unseen" in _y_labels(plot.to_svg())


def test_data_level_in_no_run_raises() -> None:
    runs = {**RUNS, "B": RUNS["B"][:-1]}
    plot = (
        gg.ggplot(CELLS, gg.aes("gene", "cell", fill="value"))
        + gg.geom_tile()
        + gg.scale_y_dendro(TREE, leaf_levels=runs)
    )
    with pytest.raises(SpecError, match=r"'B-2' is on the y axis but in no run"):
        plot.to_svg()


@pytest.mark.parametrize(
    ("leaf_levels", "error", "message"),
    [
        ([("T", ["a"])], UnsupportedValueError, "expected a mapping {leaf: [levels]}, got list"),
        ({**RUNS, "Monno": ["x"]}, SpecError, "'Monno' is not a leaf of the tree"),
        ({**RUNS, "Monno": ["x"]}, SpecError, "did you mean 'Mono'?"),
        ({**RUNS, "T": "T-0"}, UnsupportedValueError, "must be a sequence of str"),
        ({**RUNS, "T": []}, SpecError, "leaf 'T' has no levels"),
        ({**RUNS, "T": [0]}, UnsupportedValueError, "levels are str, got int 0"),
        ({**RUNS, "NK": ["T-0"]}, SpecError, "level 'T-0' is in the runs of leaves 'T' and 'NK'"),
        ({**RUNS, "T": ["T-0", "T-0"]}, SpecError, "level 'T-0' is twice in the run of leaf 'T'"),
        ({"T": RUNS["T"]}, SpecError, "no levels for the leaves 'NK', 'B', 'Mono'"),
    ],
)
def test_bad_leaf_levels_fail_early(
    leaf_levels: object, error: type[Exception], message: str
) -> None:
    with pytest.raises(error, match=re.escape(message)) as caught:
        gg.scale_x_dendro(TREE, leaf_levels=leaf_levels)  # type: ignore[arg-type]
    assert caught.value.path.startswith("leaf_levels")  # type: ignore[attr-defined]


def test_runs_may_be_series() -> None:
    runs = {leaf: pl.Series(cells) for leaf, cells in RUNS.items()}
    spec = gg.scale_y_dendro(TREE, leaf_levels=runs).as_dict()
    assert spec["leaf_levels"] == RUNS


def test_a_later_tree_replaces_the_runs() -> None:
    # Runs belong to their tree: a later scale_*_dendro() without leaf_levels drops them.
    cell_tree = gg.hclust(CELLS.pivot(on="gene", index="cell", values="value"), labels="cell")
    plot = _heatmap_y() + gg.scale_y_dendro(cell_tree)
    labels = _y_labels(plot.to_svg())
    bottom_to_top = [name for name, _ in sorted(labels.items(), key=lambda item: -item[1])]
    assert bottom_to_top == list(cell_tree.ordered_labels)
