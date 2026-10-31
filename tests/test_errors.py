"""Errors are part of the API: typed classes, structured attributes, helpful messages."""

import datetime
import io
import re

import ggrust as gg
import polars as pl
import pytest
from ggrust.errors import (
    ColumnNotFoundError,
    DataError,
    ExportError,
    GgrustError,
    RenderError,
    SpecError,
    UnsupportedColumnTypeError,
    UnsupportedDataError,
    UnsupportedValueError,
    did_you_mean,
)

FRAME = pl.DataFrame(
    {
        "sepal_length": [5.1, 4.9, 6.3],
        "sepal_width": [3.5, 3.0, 3.3],
        "species": ["setosa", "setosa", "virginica"],
    }
)


def scatter(**point: object) -> gg.PlotSpec:
    return gg.ggplot(FRAME, gg.aes("sepal_length", "sepal_width")) + gg.geom_point(**point)


def test_every_class_is_a_ggrust_error_and_the_matching_builtin() -> None:
    for error_class in [SpecError, DataError, ColumnNotFoundError, RenderError, ExportError]:
        assert issubclass(error_class, GgrustError)
        assert issubclass(error_class, ValueError)
    for error_class in [UnsupportedColumnTypeError, UnsupportedDataError, UnsupportedValueError]:
        assert issubclass(error_class, GgrustError)
        assert issubclass(error_class, TypeError)
        # lets_plot raised ValueError for these: `except ValueError` keeps working.
        assert issubclass(error_class, ValueError)


@pytest.mark.parametrize(
    ("build", "path", "message"),
    [
        (
            lambda: (
                gg.ggplot(FRAME, gg.aes("sepal_length", "sepal_width")) + gg.layer(geom="piont")
            ),
            "layers[0].geom",
            "unknown geom 'piont'; supported: point, .*; did you mean 'point'\\?$",
        ),
        (
            lambda: gg.ggplot(FRAME, gg.aes("species")) + gg.geom_bar(stat="cuont"),
            "layers[0].stat",
            "unknown stat 'cuont'; supported: .*; did you mean 'count'\\?$",
        ),
        (
            lambda: scatter(position="jiter"),
            "layers[0].position",
            "unknown position 'jiter'; supported: .*; did you mean 'jitter'\\?$",
        ),
        (
            lambda: scatter(color="rde"),
            "layers[0].color",
            "invalid color 'rde'; expected a color name .*; did you mean 'red'\\?$",
        ),
        (
            lambda: scatter(size="big"),
            "layers[0].size",
            'expected a finite number, got "big"',
        ),
        (
            lambda: scatter() + gg.scale_x_continuous(trans="log100"),
            "scales[0].trans",
            "unknown transform 'log100'; supported: .*; did you mean 'log10'\\?$",
        ),
        (
            lambda: (
                gg.ggplot(FRAME, gg.aes("sepal_length", "sepal_width", color="species"))
                + gg.geom_point()
                + gg.scale_color_brewer(palette="Set11")
            ),
            "scales[0].palette",
            "brewer palette 'Set11' was not found.*; did you mean 'Set1'\\?$",
        ),
        (
            lambda: (
                gg.ggplot(FRAME, gg.aes("sepal_length", "sepal_width", color="sepal_length"))
                + gg.geom_point()
                + gg.scale_color_viridis(option="magam")
            ),
            "scales[0].option",
            "unknown color map \"magam\"; use .*; did you mean 'magma'\\?$",
        ),
        (
            lambda: scatter() + gg.theme(legend_position="botom"),
            "theme.legend_position",
            "expected left, right, top, bottom, none or two numbers, got 'botom'; "
            "did you mean 'bottom'\\?$",
        ),
    ],
)
def test_spec_errors_name_the_path_and_suggest_a_fix(build, path: str, message: str) -> None:
    with pytest.raises(SpecError) as caught:
        build().to_svg()
    error = caught.value
    assert error.path == path
    assert str(error).startswith(f"invalid plot specification at '{path}': ")
    assert error.reason in str(error)
    assert re.search(message, error.reason), error.reason


def test_nested_figures_join_their_path() -> None:
    grid = gg.gggrid([scatter(), scatter(size="big")])
    with pytest.raises(SpecError) as caught:
        grid.to_svg()
    assert caught.value.path == "figures[1].layers[0].size"


def test_missing_column_lists_every_column_and_suggests_one() -> None:
    plot = gg.ggplot(FRAME, gg.aes("sepal_lenght", "sepal_width")) + gg.geom_point()
    with pytest.raises(ColumnNotFoundError) as caught:
        plot.to_svg()
    error = caught.value
    assert error.column == "sepal_lenght"
    assert error.path == "mapping.x"
    # Every column of the source data, not only the ones sent to the engine.
    assert error.available == ["sepal_length", "sepal_width", "species"]
    assert str(error) == (
        "at 'mapping.x': column 'sepal_lenght' not found in data; available columns: "
        "sepal_length, sepal_width, species; did you mean 'sepal_length'?"
    )


def test_missing_layer_column_names_the_layer_mapping() -> None:
    plot = gg.ggplot(FRAME) + gg.geom_point(gg.aes("sepal_length", "speceis"))
    with pytest.raises(ColumnNotFoundError, match="did you mean 'species'") as caught:
        plot.to_svg()
    assert caught.value.path == "layers[0].mapping.y"


def test_unsupported_column_type_says_how_to_fix_it() -> None:
    frame = FRAME.with_columns(day=pl.lit(datetime.date(2020, 1, 1)))
    plot = gg.ggplot(frame, gg.aes("day", "sepal_width")) + gg.geom_point()
    with pytest.raises(UnsupportedColumnTypeError) as caught:
        plot.to_svg()
    error = caught.value
    assert (error.column, error.data_type, error.path) == ("day", "Date32", "mapping.x")
    assert "convert the column first" in str(error)


def test_unsendable_values_name_their_spec_path() -> None:
    with pytest.raises(UnsupportedValueError, match="cannot send a value of type object") as caught:
        scatter(size=object()).to_svg()
    assert caught.value.path == "layers[0].size"


def test_series_data_is_rejected_with_a_fix() -> None:
    with pytest.raises(UnsupportedDataError, match=r"series\.to_frame\(\)"):
        gg.ggplot(FRAME["sepal_length"], gg.aes("sepal_length", "sepal_length")).to_svg()


def test_export_options_raise_export_errors() -> None:
    with pytest.raises(ExportError, match="'dpi' must be a positive number, got -1"):
        scatter().to_png(io.BytesIO(), dpi=-1)
    with pytest.raises(ExportError, match=r"unsupported unit 'mmm'.*did you mean 'mm'"):
        scatter().to_png(io.BytesIO(), w=3, h=2, unit="mmm")


def test_python_side_spec_errors() -> None:
    with pytest.raises(SpecError, match="'mapping' must be an aes\\(\\) mapping"):
        gg.geom_point(mapping="x")
    with pytest.raises(SpecError, match="'plots' is empty"):
        gg.gggrid([])
    with pytest.raises(SpecError, match="did you mean 'min'"):
        gg.ggtb(size_basis="mni")


def test_did_you_mean_follows_the_engine_rule() -> None:
    assert did_you_mean("sgv", ["svg", "png"]) == "; did you mean 'svg'?"
    assert did_you_mean("PNG", ["svg", "png"]) == "; did you mean 'png'?"
    assert did_you_mean("x", ["y"]) == ""
    assert did_you_mean("histogram", ["point", "line"]) == ""
    # Arbitrary guesses are left out: ties, and a prefix of several names.
    assert did_you_mean("genee", ["gene0", "gene1"]) == ""
    assert did_you_mean("log", ["log10", "log2"]) == ""


def test_missing_column_on_wide_data_stays_readable() -> None:
    frame = pl.DataFrame({f"gene{index}": [1.0] for index in range(1000)})
    plot = gg.ggplot(frame, gg.aes("gene1", "genee")) + gg.geom_point()
    with pytest.raises(ColumnNotFoundError) as caught:
        plot.to_svg()
    assert len(caught.value.available) == 1000
    assert str(caught.value).endswith("gene29, ... (970 more)")


def test_ggsave_accepts_paths(tmp_path) -> None:
    written = gg.ggsave(scatter(), tmp_path / "plot.svg", path=tmp_path)
    assert written == str(tmp_path / "plot.svg")
