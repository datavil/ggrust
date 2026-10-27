"""Python -> Rust bridge: frame references, deduplication and referenced columns."""

import json

import ggrust as gg
import polars as pl
import pytest
from ggrust._ggrust import referenced_columns
from ggrust._render import split_data, to_json

FRAME = pl.DataFrame({"x": [1.0, 2.0, 3.0], "y": [3.0, 1.0, 2.0], "c": ["a", "b", "a"]})


def test_data_becomes_frame_references_deduplicated_by_identity():
    other = FRAME.clone()
    plot = (
        gg.ggplot(FRAME, gg.aes("x", "y"))
        + gg.geom_point(data=FRAME)
        + gg.geom_point(data=other)
        + gg.geom_point()
    )
    spec, frames = split_data(plot.as_dict())
    names = ["x", "y", "c"]
    assert spec["data"] == {"frame": 0, "columns": names}
    assert [layer.get("data") for layer in spec["layers"]] == [
        {"frame": 0, "columns": names},
        {"frame": 1, "columns": names},
        None,
    ]
    assert [path for _, path in frames] == ["data", "layers[1].data"]
    assert frames[0][0] is FRAME
    assert frames[1][0] is other


def test_nested_figures_share_frames():
    cell = {"kind": "plot", "data": FRAME, "layers": [{"geom": "point", "data": FRAME}]}
    grid = {"kind": "subplots", "figures": [cell, None, cell]}
    spec, frames = split_data(grid)
    assert len(frames) == 1
    assert spec["figures"][1] is None
    assert [figure["data"]["frame"] for figure in spec["figures"][::2]] == [0, 0]
    assert frames[0][1] == "figures[0].data"


def test_referenced_columns_come_from_rust():
    spec = {
        "kind": "plot",
        "data": {"frame": 0},
        "mapping": {"x": "x", "y": "y"},
        "facet": {"name": "wrap", "facets": "sample"},
        "layers": [
            {"geom": "point", "mapping": {"color": "c"}, "tooltips": {"lines": ["@{cell type}"]}}
        ],
    }
    # Facets and tooltips are not built yet: their columns are not exported.
    assert referenced_columns(to_json(spec)) == ["c", "x", "y"]


def test_unconvertible_column_used_only_by_tooltips_does_not_fail():
    pandas = pytest.importorskip("pandas")
    data = pandas.DataFrame({"x": [1.0, 2.0], "y": [2.0, 1.0], "mixed": [1, "a"]})
    plot = gg.ggplot(data, gg.aes("x", "y")) + gg.geom_point(
        tooltips=gg.layer_tooltips().line("@mixed")
    )
    assert plot.to_svg().count("<circle") == 2


def test_a_bad_frame_reference_names_the_path():
    spec = {"kind": "plot", "data": {"frame": 3}, "layers": [{"geom": "point"}]}
    with pytest.raises(ValueError, match=r"at 'data': frame 3 was referenced but only 0"):
        gg._ggrust.render(json.dumps(spec), [], "svg", 1.0)


def test_shared_frame_renders_like_separate_frames():
    shared = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point(data=FRAME, color="red")
    separate = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point(data=FRAME.clone(), color="red")

    # SVG ids are a hash of the content, so equal plots give equal bytes.
    assert shared.to_svg() == separate.to_svg()


def test_continuous_color_scales_take_expand_like_lets_plot():
    import lets_plot as lp

    for aesthetic in ("color", "fill"):
        ours = getattr(gg, f"scale_{aesthetic}_continuous")(expand=[0, 0], low="red")
        theirs = getattr(lp, f"scale_{aesthetic}_continuous")(expand=[0, 0], low="red")
        assert ours.as_dict() == theirs.as_dict()
