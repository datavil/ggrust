"""Regressions found by the Phase 0.7 bug hunt (Python API vs lets_plot)."""

import datetime
import io
import json
import warnings

import numpy as np
import pandas as pd
import polars as pl
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust._render import to_json  # noqa: E402


def scatter(data, module=gg, **options):
    return module.ggplot(data, module.aes("x", "y")) + module.geom_point(**options)


@pytest.mark.parametrize(
    "x",
    [[1, 2.5, 3], [1, float("nan"), 3], [1, np.nan, 3], [1, None, 3.5]],
    ids=["int-float", "nan", "numpy-nan", "none"],
)
def test_dicts_with_mixed_numbers_render(x):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", gg.GgrustWarning)
        assert scatter({"x": x, "y": [1, 2, 3]}).to_svg().count("<circle") >= 2


def test_unmapped_columns_of_any_type_are_not_exported():
    frame = pd.DataFrame(
        {
            "x": [1.0, 2.0, 3.0],
            "y": [3.0, 2.0, 1.0],
            "objects": [{"a": 1}, [1], None],
            "complex": [1 + 2j, 3j, 0j],
            "sparse": pd.arrays.SparseArray([0, 0, 1]),
        }
    )
    assert scatter(frame).to_svg().count("<circle") == 3
    wide = pl.DataFrame({"x": [1.0], "y": [2.0], "big": pl.Series([1], dtype=pl.Int128)})
    assert scatter(wide).to_svg().count("<circle") == 1
    assert scatter({"x": [1], "y": [2], "z": [1, "a", None]}).to_svg().count("<circle") == 1


def test_bad_mapped_column_is_a_clear_value_error():
    frame = pd.DataFrame({"x": [1, "a", 3], "y": [1.0, 2.0, 3.0]})
    with pytest.raises(ValueError, match=r"at 'data': could not read the data columns"):
        scatter(frame).to_svg()


@pytest.mark.parametrize("ordered", [False, True])
def test_pandas_as_discrete_levels_match_lets_plot(ordered):
    def build(module):
        frame = pd.DataFrame({"c": pd.Categorical(["a", "b"], ordered=ordered), "y": [1, 2]})
        mapping = module.aes(x=module.as_discrete("c", levels=["b", "a"], order=1), y="y")
        return module.ggplot(frame, mapping)

    assert build(gg).as_dict()["data_meta"] == build(lp).as_dict()["data_meta"]


def test_tooltip_lists_become_layer_tooltips():
    ours = gg.geom_point(tooltips=["x"]).as_dict()["tooltips"]
    theirs = lp.geom_point(tooltips=["x"]).as_dict()["tooltips"]
    assert ours == theirs == {"formats": [], "variables": ["x"]}
    chained = gg.layer_tooltips(["a"]).line("@b").format("@b", ".2f").title("t").anchor("top_left")
    reference = (
        lp.layer_tooltips(["a"]).line("@b").format("@b", ".2f").title("t").anchor("top_left")
    )
    assert chained.as_dict() == reference.as_dict()


def test_largest_ggsize_renders_at_default_scale():
    buffer = io.BytesIO()
    scatter({"x": [1.0], "y": [1.0]}).__add__(gg.ggsize(9000, 100)).to_png(buffer)
    assert buffer.getvalue().startswith(b"\x89PNG")


@pytest.mark.parametrize("width", [float("nan"), "wide", 20_000])
def test_ggsize_validation_matches_lets_plot(width):
    with pytest.raises(AssertionError) as ours:
        gg.ggsize(width, 10)
    with pytest.raises(AssertionError) as theirs:
        lp.ggsize(width, 10)
    assert str(ours.value) == str(theirs.value)


def test_datetimes_are_sent_as_epoch_milliseconds():
    spec = {
        "a": np.datetime64("2020-01-01T00:00:00.000000000"),
        "b": np.datetime64("2020-01-01"),
        "c": datetime.datetime(2020, 1, 1, tzinfo=datetime.UTC),
        "d": datetime.date(2020, 1, 1),
        "e": np.timedelta64(2, "s"),
        "f": pd.Timestamp("2020-01-01"),
    }
    values = json.loads(to_json(spec))
    assert values == {key: 1577836800000.0 for key in "abcdf"} | {"e": 2000.0}


def test_colliding_and_deep_values_are_type_errors():
    with pytest.raises(TypeError, match="collide"):
        to_json({"k": {1: "a", "1": "b"}})
    nested = []
    for _ in range(200):
        nested = [nested]
    with pytest.raises(TypeError, match="nested more than"):
        to_json({"deep": nested})


def test_errors_carry_the_warnings_that_explain_them():
    plot = gg.ggplot({"y": [1.0, 2.0]}, gg.aes(x=[1.0, 2.0], y="y")) + gg.geom_point()
    with pytest.raises(ValueError, match=r"missing: x(.|\n)*note: .*mapping 'x' to the value"):
        plot.to_svg()


def test_duplicate_aesthetic_aliases_are_an_error():
    plot = gg.ggplot({"x": [1.0], "y": [1.0], "c": ["a"]}, gg.aes("x", "y", color="c", colour="c"))
    with pytest.raises(ValueError, match="aesthetic 'color' is mapped twice"):
        (plot + gg.geom_point()).to_svg()


def test_str_matches_lets_plot_layout():
    ours = str(gg.ggplot() + gg.geom_point()).splitlines()
    theirs = str(lp.ggplot() + lp.geom_point()).splitlines()
    assert ours[0] == theirs[0] == "plot"
    assert [line for line in ours if "[" in line] == [line for line in theirs if "[" in line]


@pytest.mark.parametrize("kind", ["dict", "polars", "pandas"])
def test_layer_data_without_referenced_columns_keeps_its_rows(kind):
    # Bug hunt 0.10.2: the export kept no column, so the layer data had 0 rows and
    # could not merge with the plot data (lets-plot draws 3 points).
    layer = {"z": [1.0, 2.0, 3.0]}
    layer = {"dict": layer, "polars": pl.DataFrame(layer), "pandas": pd.DataFrame(layer)}[kind]
    plot = gg.ggplot({"a": [1.0, 2.0, 3.0], "b": [1.0, 2.0, 3.0]}, gg.aes("a", "b"))
    assert (plot + gg.geom_point(data=layer)).to_svg().count("<circle") == 3
