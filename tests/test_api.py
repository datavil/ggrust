"""lets_plot-compatible API: as_dict() parity with real lets_plot, + semantics, exports."""

import io
import re
import warnings

import numpy as np
import pandas as pd
import polars as pl
import pyarrow as pa
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)  # lets_plot's own docstrings

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.plot.core import FeatureSpec, FeatureSpecArray, LayerSpec, PlotSpec  # noqa: E402

FRAME = pl.DataFrame({"x": [1.0, 2.0, 3.0], "y": [3.0, 1.0, 2.0], "c": ["a", "b", "a"]})
OTHER = pl.DataFrame({"u": [1.0, 2.0], "v": [2.0, 1.0]})


def without_data(value):
    if isinstance(value, dict):
        return {key: without_data(item) for key, item in value.items() if key != "data"}
    if isinstance(value, list):
        return [without_data(item) for item in value]
    return value


def build_both(build):
    """Run the same builder against lets_plot and ggrust."""
    return build(lp), build(gg)


PARITY_CASES = {
    "scatter": lambda m: m.ggplot(FRAME, m.aes("x", "y")) + m.geom_point(),
    "constants": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y"))
        + m.geom_point(size=2, color="red", alpha=0.5, na_rm=True, tooltips="none")
    ),
    "layer data": lambda m: (
        m.ggplot(FRAME) + m.geom_point(m.aes("u", "v"), data=OTHER, inherit_aes=False)
    ),
    "labels": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y"))
        + m.geom_point()
        + m.ggsize(400, 300)
        + m.labs(title="T", subtitle="S", x="X", y="Y", color="C", caption="cap")
        + m.xlab("x2")
        + m.ggtitle("T2")
    ),
    "plus aes": lambda m: m.ggplot(FRAME) + m.geom_point(m.aes("x", "y")) + m.aes(color="c"),
    "as_discrete": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y", color=m.as_discrete("c", order=-1))) + m.geom_point()
    ),
    "as_discrete levels": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y", color=m.as_discrete("c", levels=["b", "a"])))
        + m.geom_point()
    ),
    "pandas categorical": lambda m: (
        m.ggplot(
            FRAME.to_pandas().assign(
                c=lambda f: pd.Categorical(f["c"], categories=["b", "a"], ordered=True)
            ),
            m.aes("x", "y", color="c"),
        )
        + m.geom_point()
    ),
    "polars enum": lambda m: (
        m.ggplot(
            FRAME.with_columns(pl.col("c").cast(pl.Enum(["b", "a"]))), m.aes("x", "y", color="c")
        )
        + m.geom_point()
    ),
    "dict data": lambda m: (
        m.ggplot({"x": [1, 2], "y": [1.5, None], "s": ["p", "q"]}, m.aes("x", "y")) + m.geom_point()
    ),
    "layer function": lambda m: m.ggplot(FRAME) + m.layer("point", mapping=m.aes("x", "y")),
    "guides": lambda m: m.ggplot(FRAME, m.aes("x", "y")) + m.geom_point() + m.guides(color="none"),
    "layer plus aes": lambda m: m.ggplot(FRAME) + (m.geom_point() + m.aes("x", "y")),
    "shape scales": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y", shape="c"))
        + m.geom_point(shape=21, fill="red", stroke=2)
        + m.scale_shape(solid=False, name="S")
        + m.scale_shape_manual(values={"a": 1, "b": 22}, na_value=3, guide="none")
    ),
    "axis labels": lambda m: (
        m.ggplot(FRAME, m.aes("x", "y"))
        + m.geom_point()
        + m.scale_x_continuous(breaks=[1, 2], labels=["a", "b"], format=".1f")
        + m.scale_y_discrete(labels={"p": "P"})
    ),
}


@pytest.mark.parametrize("name", PARITY_CASES)
def test_as_dict_matches_lets_plot(name):
    theirs, ours = build_both(PARITY_CASES[name])
    assert without_data(ours.as_dict()) == without_data(theirs.as_dict())


def test_as_dict_keeps_the_data_object():
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point(data=OTHER)
    spec = plot.as_dict()
    assert spec["data"] is FRAME
    assert spec["layers"][0]["data"] is OTHER


def test_feature_arrays_match_lets_plot():
    ours = gg.labs(title="t", x="a", y="b")
    theirs = lp.labs(title="t", x="a", y="b")
    assert isinstance(ours, FeatureSpecArray)
    assert ours.as_dict() == theirs.as_dict()


def test_plus_semantics():
    plot = gg.ggplot(FRAME, gg.aes("x", "y"))
    assert isinstance(plot + gg.geom_point(), PlotSpec)
    assert isinstance(gg.geom_point() + gg.ggsize(1, 1), FeatureSpecArray)
    # lets_plot stores plot + plot under "plot" instead of raising.
    assert (plot + plot).props()["plot"] is plot
    with pytest.raises(TypeError):
        _ = gg.geom_point() + plot
    # cellestial's deferred layers rely on TypeError for unknown right operands.
    with pytest.raises(TypeError):
        _ = plot + object()
    # Adding returns a copy: the original plot is unchanged.
    _ = plot + gg.geom_point()
    assert not plot.has_layers()


def test_layer_plus_own_feature_replaces_it():
    layer = gg.geom_point(gg.aes("x", "y")) + gg.aes("y", "x")
    assert isinstance(layer, LayerSpec)
    assert layer.as_dict()["mapping"] == {"x": "y", "y": "x"}


def test_module_paths_cellestial_imports():
    from ggrust.plot.core import DummySpec

    assert issubclass(PlotSpec, FeatureSpec) and issubclass(DummySpec, FeatureSpec)
    gg.LetsPlot.setup_html()  # no-op, must not raise


def scatter(data=FRAME):
    return gg.ggplot(data, gg.aes("x", "y")) + gg.geom_point()


@pytest.mark.parametrize(
    "data",
    [FRAME, FRAME.to_pandas(), FRAME.to_arrow(), FRAME.to_dict(as_series=False), FRAME.lazy()],
    ids=["polars", "pandas", "pyarrow", "dict", "lazy"],
)
def test_to_svg_draws_every_point(data):
    svg = scatter(data).to_svg()
    assert svg.startswith("<svg") and svg.count("<circle") == 3


def test_to_png_writes_files_and_file_likes(tmp_path):
    path = scatter().to_png(str(tmp_path / "nested" / "plot.png"))
    with open(path, "rb") as file:
        assert file.read(8) == b"\x89PNG\r\n\x1a\n"
    buffer = io.BytesIO()
    assert scatter().to_png(buffer, scale=1) is None
    assert buffer.getvalue().startswith(b"\x89PNG")


def test_svg_size_override_and_units():
    svg = scatter().to_svg(w=300, h=200)
    assert 'width="300" height="200"' in svg
    with pytest.raises(ValueError, match="unit 'in' is not supported yet"):
        scatter().to_svg(w=3, h=2, unit="in")
    with pytest.raises(ValueError, match="'dpi' is not supported yet"):
        scatter().to_png(io.BytesIO(), dpi=300)


def root_id(html: str) -> str:
    return re.search(r'<div id="([^"]+)" class="ggrust-plot"', html).group(1)


def test_html_outputs():
    fragment = scatter()._repr_html_()
    assert fragment.startswith('<div id="ggrust-') and "<svg" in fragment
    assert "<!DOCTYPE" not in fragment
    assert f'window.ggrust.mount("{root_id(fragment)}");' in fragment
    assert f'("{gg.engine_version()}");' in fragment
    page = scatter().to_html()
    assert page.startswith("<!DOCTYPE html>") and "<svg" in page and "window.ggrust" in page
    iframe = scatter().to_html(iframe=True)
    assert iframe.startswith("<iframe ") and 'width="600" height="400"' in iframe
    assert "&lt;!DOCTYPE html&gt;" in iframe and "<svg" not in iframe
    grid = gg.gggrid([scatter(), scatter()])
    assert "window.ggrust.mount" in grid._repr_html_() and grid.to_html().startswith("<!DOCTYPE")


def test_each_html_display_gets_its_own_ids():
    """One plot shown twice on a page must not share root or clip-path ids."""
    plot = scatter()
    first, second = plot._repr_html_(), plot._repr_html_()
    assert root_id(first) != root_id(second)
    first_ids = set(re.findall(r' id="([^"]+)"', first))
    second_ids = set(re.findall(r' id="([^"]+)"', second))
    assert any(id_.endswith("-clip0") for id_ in first_ids)
    assert all(id_.startswith(root_id(first)) for id_ in first_ids)
    assert not first_ids & second_ids
    # SVG stays deterministic (content-hash ids).
    assert plot.to_svg() == plot.to_svg()


def test_unsupported_options_warn():
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point(angle=45)
    with pytest.warns(gg.GgrustWarning, match="does not support option 'angle'"):
        plot.to_svg()


def test_errors_name_the_path():
    plot = gg.ggplot(FRAME, gg.aes("x", "missing")) + gg.geom_point()
    with pytest.raises(ValueError, match=r"at 'mapping.y': column 'missing' not found"):
        plot.to_svg()


def test_numpy_values_and_nan_options_are_sent_safely():
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point(
        size=np.float64(4), alpha=float("nan")
    )
    svg = plot.to_svg()
    assert 'r="4.4"' in svg  # size 4 x 2.2 / 2
    with pytest.raises(TypeError, match=r"at 'spec.layers\[0\].color'"):
        (gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point(color=object())).to_svg()


def test_pyarrow_and_large_data():
    rng = np.random.default_rng(0)
    table = pa.table({"x": rng.normal(size=200_000), "y": rng.normal(size=200_000)})
    png = io.BytesIO()
    (gg.ggplot(table, gg.aes("x", "y")) + gg.geom_point(size=0.5)).to_png(png, scale=1)
    assert len(png.getvalue()) > 1000
