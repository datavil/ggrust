"""Python -> Rust bridge: frame references, deduplication and referenced columns."""

import json
import re

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
    assert spec["data"] == {"frame": 0}
    assert [layer.get("data") for layer in spec["layers"]] == [
        {"frame": 0},
        {"frame": 1},
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
    assert [figure["data"] for figure in spec["figures"][::2]] == [{"frame": 0}] * 2
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
    assert referenced_columns(to_json(spec)) == ["c", "cell type", "sample", "x", "y"]


def test_a_bad_frame_reference_names_the_path():
    spec = {"kind": "plot", "data": {"frame": 3}, "layers": [{"geom": "point"}]}
    with pytest.raises(ValueError, match=r"at 'data': frame 3 was referenced but only 0"):
        gg._ggrust.render(json.dumps(spec), [], "svg", 1.0)


def test_shared_frame_renders_like_separate_frames():
    shared = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point(data=FRAME, color="red")
    separate = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point(data=FRAME.clone(), color="red")

    # SVG ids still come from a global counter (plan 0.10.8): compare without them.
    def strip(svg):
        return re.sub(r"gg\d+-", "", svg)

    assert strip(shared.to_svg()) == strip(separate.to_svg())
