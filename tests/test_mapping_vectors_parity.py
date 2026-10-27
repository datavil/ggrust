"""Data vectors in a mapping (`aes(x=[1, 2, 3])`) must draw as in lets-plot.

lets-plot turns each list into a data column named after the aesthetic (`x1`,
`x2`, ... when a data column already has that name), so the axis titles, legends
and points are compared. `as_dict()` keeps the list, identical to lets_plot.
"""

import math
import warnings

import polars as pl
import pytest
from test_bar_parity import _texts
from test_jitter_parity import _ggrust_points, _lets_plot_points

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

CASES = {
    "function_list_x": lambda m: m.ggplot() + m.geom_function(m.aes(x=[1, 2, 3]), fun=math.sqrt),
    "function_tuple_x_points": lambda m: (
        m.ggplot() + m.geom_function(m.aes(x=(0.5, 1.5, 4.0)), fun=lambda x: x * x, geom="point")
    ),
    "layer_vectors": lambda m: m.ggplot() + m.geom_point(m.aes(x=[1, 2, 3], y=[3.0, 1.0, 2.0])),
    "plot_vector_with_data": lambda m: (
        m.ggplot({"a": [1.0, 2.0, 3.0]}, m.aes(x=[3, 1, 2], y="a")) + m.geom_point()
    ),
    "name_taken_by_data": lambda m: (
        m.ggplot(
            pl.DataFrame({"a": [1.0, 2.0, 3.0], "x": [9.0, 9.0, 9.0]}), m.aes(x=[1, 2, 3], y="a")
        )
        + m.geom_point()
    ),
    # Unused data columns still take names: lets-plot avoids all of them.
    "many_names_taken": lambda m: (
        m.ggplot(
            {"a": [1.0, 2.0, 3.0], **{f"x{i}" if i else "x": [0.0] * 3 for i in range(19)}},
            m.aes(x=[1, 2, 3], y="a"),
        )
        + m.geom_point()
    ),
    "discrete_color_vector": lambda m: (
        m.ggplot({"a": [1.0, 2.0, 3.0], "b": [2.0, 1.0, 3.0]}, m.aes("a", "b"))
        + m.geom_point(m.aes(color=["u", "v", "u"]), size=4)
    ),
    "text_vector_axis": lambda m: (
        m.ggplot() + m.geom_point(m.aes(x=["p", "q", "p"], y=[1, None, 3]))
    ),
}


@pytest.mark.parametrize("name", sorted(CASES))
def test_vectors_draw_as_in_lets_plot(name):
    build = CASES[name]
    svgs = [(build(m) + m.ggsize(400, 300)).to_svg() for m in (lp, gg)]
    expected, actual = _texts(svgs[0]), _texts(svgs[1])
    assert sorted(actual) == sorted(expected)
    no_legend = [(build(m) + m.theme(legend_position="none")).to_svg() for m in (lp, gg)]
    flat = [
        [value for point in points for value in point]
        for points in (_ggrust_points(no_legend[1]), _lets_plot_points(no_legend[0]))
    ]
    assert flat[0] == pytest.approx(flat[1], abs=2e-3)


def test_as_dict_keeps_the_list():
    for module in (gg, lp):
        assert module.geom_point(module.aes(x=[1, 2])).as_dict()["mapping"] == {"x": [1, 2]}


def test_length_mismatch_names_the_mapping():
    plot = gg.ggplot({"a": [1.0, 2.0]}, gg.aes(x=[1, 2, 3], y="a")) + gg.geom_point()
    with pytest.raises(Exception, match=r"mapping\.x.*3 values.*2 rows"):
        plot.to_svg()


def test_group_list_still_names_columns():
    frame = {"a": [1.0, 2.0, 3.0, 4.0], "b": [1.0, 2.0, 1.0, 2.0], "g": ["u", "u", "v", "v"]}
    svg = (gg.ggplot(frame, gg.aes("a", "b")) + gg.geom_line(gg.aes(group=["g"]))).to_svg()
    assert svg.count('stroke-width="1.65"') == 2


def test_unreadable_unused_columns_do_not_matter():
    """Only names are sent for unused columns, so an object column named `x` is fine."""
    import pandas as pd

    frame = pd.DataFrame({"a": [1.0, 2.0, 3.0], "x": [object()] * 3})
    svg = (gg.ggplot(frame, gg.aes(x=[1, 2, 3], y="a")) + gg.geom_point()).to_svg()
    assert ">x1<" in svg
