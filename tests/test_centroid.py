"""stat_centroid and stat_summary_xy (ggpp) against ggpp 0.6.1 output.

Reference: tests/fixtures/ggpp_centroid_reference.json, written by
tests/fixtures/generate_ggpp_centroid_reference.R.
"""

import json
from pathlib import Path
from typing import Any

import ggrust as gg
import polars as pl
import pytest
from ggrust.errors import SpecError

REFERENCE = json.loads(
    (Path(__file__).parent / "fixtures" / "ggpp_centroid_reference.json").read_text()
)
DATA = {name: REFERENCE[name] for name in ("x", "y", "cluster", "sample")}


def _rows(layer: gg.LayerSpec) -> pl.DataFrame:
    plot = gg.ggplot(DATA, gg.aes("x", "y", label="cluster")) + layer + gg.facet_wrap("sample")
    return plot.layer_data(0).sort("panel", "label")


@pytest.mark.parametrize("case", REFERENCE["cases"], ids=lambda case: json.dumps(case["options"]))
@pytest.mark.parametrize("form", ["geom", "function"])
def test_matches_ggpp_per_cluster_and_panel(case: dict[str, Any], form: str) -> None:
    if form == "geom":
        layer = gg.geom_text(stat=case["stat"], **case["options"])
    elif case["stat"] == "centroid":
        layer = gg.stat_centroid(geom="text", **case["options"])
    else:
        layer = gg.stat_summary_xy(geom="text", **case["options"])
    rows = _rows(layer)
    expected = case["rows"]
    assert rows["panel"].to_list() == expected["panel"]
    assert rows["label"].cast(pl.String).to_list() == expected["label"]
    assert rows["x"].to_list() == pytest.approx(expected["x"], abs=1e-12)
    assert rows["y"].to_list() == pytest.approx(expected["y"], abs=1e-12)


def test_label_geom_and_defaults() -> None:
    # Mean by default; a label column with one value per group keeps it.
    rows = _rows(gg.geom_label(stat="centroid"))
    assert rows.height == 6
    mean = next(case for case in REFERENCE["cases"] if case["options"] == {"fun": "mean"})
    assert rows["x"].to_list() == pytest.approx(mean["rows"]["x"], abs=1e-12)
    layer = gg.stat_centroid()
    assert layer.as_dict()["geom"] == "point"
    assert layer.as_dict()["show_legend"] is False


def test_missing_values_and_empty_groups() -> None:
    data = {"x": [1.0, 3.0, None, 7.0], "y": [2.0, 4.0, 5.0, None], "g": ["a", "a", "a", "b"]}
    plot = gg.ggplot(data, gg.aes("x", "y", label="g")) + gg.geom_text(stat="centroid", na_rm=True)
    rows = plot.layer_data(0)
    # Group b has no row with both x and y: no centroid.
    assert rows.select("x", "y").rows() == [(2.0, 3.0)]


def test_renders_text_at_the_centres() -> None:
    plot = (
        gg.ggplot(DATA, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.geom_text(gg.aes(label="cluster"), stat="centroid")
    )
    svg = plot.to_svg()
    for name in ("B cells", "T cells", "NK"):
        assert svg.count(f">{name}<") == 1


def test_bad_functions_name_the_parameter() -> None:
    base = gg.ggplot(DATA, gg.aes("x", "y", label="cluster"))
    with pytest.raises(SpecError, match=r"layers\[0\]\.fun\b.*did you mean 'median'"):
        (base + gg.stat_centroid(fun="medain")).to_svg()  # ty: ignore[invalid-argument-type]
    with pytest.raises(SpecError, match=r"layers\[0\]\.fun_x.*'mean' or 'median'"):
        (base + gg.stat_summary_xy(fun_x="mode")).to_svg()  # ty: ignore[invalid-argument-type]
