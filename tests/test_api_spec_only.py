"""Spec-only lets_plot API (geoms, positions, facets, gggrid, ggtb, ...) the engine does
not draw yet: identical signatures and as_dict(), clear errors when rendered."""

import inspect
import warnings

import polars as pl
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)  # lets_plot's own docstrings

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.plot.core import FeatureSpecArray, PlotSpec  # noqa: E402
from ggrust.plot.subplots import SupPlotsSpec  # noqa: E402

FRAME = pl.DataFrame({"x": [1.0, 2.0, 3.0], "y": [3.0, 1.0, 2.0], "c": ["a", "b", "a"]})

SPEC_ONLY_GEOMS = [
    "geom_area_ridges",
    "geom_bar",
    "geom_boxplot",
    "geom_bracket",
    "geom_histogram",
    "geom_jitter",
    "geom_label_repel",
    "geom_raster",
    "geom_rect",
    "geom_sina",
    "geom_text_repel",
    "geom_tile",
    "geom_violin",
]
NEW_NAMES = [
    *SPEC_ONLY_GEOMS,
    "geom_blank",
    "coord_flip",
    "facet_grid",
    "facet_wrap",
    "geom_function",
    "geom_imshow",
    "gggrid",
    "ggsave",
    "ggtb",
    "position_dodge",
    "position_fill",
    "position_jitter",
    "position_jitterdodge",
    "position_nudge",
    "position_stack",
]


def without_data(value):
    if isinstance(value, dict):
        return {key: without_data(item) for key, item in value.items() if key != "data"}
    if isinstance(value, list):
        return [without_data(item) for item in value]
    return value


@pytest.mark.parametrize("name", NEW_NAMES)
def test_signatures_match_lets_plot(name):
    ours = inspect.signature(getattr(gg, name))
    theirs = inspect.signature(getattr(lp, name))
    assert [(p.name, p.kind, p.default) for p in ours.parameters.values()] == [
        (p.name, p.kind, p.default) for p in theirs.parameters.values()
    ]
    assert name in gg.__all__


def every_option(name):
    """Give every keyword parameter a distinct value, so each must reach the spec."""
    options = {}
    for index, parameter in enumerate(inspect.signature(getattr(lp, name)).parameters.values()):
        if parameter.kind is inspect.Parameter.KEYWORD_ONLY and parameter.name not in (
            "mapping",
            "data",
            "map",
            "map_join",  # GIS (out of scope): lets_plot rewrites it when added to a plot
            "tooltips",
            "arrow",
        ):
            options[parameter.name] = f"v{index}"
    return options


@pytest.mark.parametrize("name", SPEC_ONLY_GEOMS)
def test_geom_specs_match_lets_plot(name):
    has_tooltips = "tooltips" in inspect.signature(getattr(lp, name)).parameters

    def build(module):
        return (
            module.ggplot(FRAME, module.aes("x", "y"))
            + getattr(module, name)(module.aes(color="c"), size=3, **every_option(name))
            + getattr(module, name)(**({"tooltips": ["c"]} if has_tooltips else {}))
        )

    assert without_data(build(gg).as_dict()) == without_data(build(lp).as_dict())


def test_boxplot_adds_the_outlier_layer_like_lets_plot():
    ours = gg.geom_boxplot(outlier_size=2, color="red", stroke=1)
    theirs = lp.geom_boxplot(outlier_size=2, color="red", stroke=1)
    assert isinstance(ours, FeatureSpecArray)
    assert ours.as_dict() == theirs.as_dict()
    assert gg.geom_boxplot(stat="identity").as_dict() == lp.geom_boxplot(stat="identity").as_dict()


def test_boxplot_coef_reaches_the_outlier_layer():
    """Deliberate difference: lets_plot drops `coef` from the outlier layer."""

    def layers(spec):
        return [item["layer"] for item in spec.as_dict()["feature-list"]]

    box, outliers = layers(gg.geom_boxplot(coef=0.5))
    assert box["coef"] == outliers["coef"] == 0.5
    assert "coef" not in layers(gg.geom_boxplot())[1]


def test_violin_rejects_draw_quantiles():
    with pytest.raises(ValueError, match="draw_quantiles"):
        gg.geom_violin(draw_quantiles=[0.5])


FEATURE_CASES = {
    "positions": lambda m: [
        m.position_dodge(0.5),
        m.position_jitter(0.1, 0.2, seed=1),
        m.position_nudge(1, 2, "px"),
        m.position_jitterdodge(0.7, 0.1, 0.2, 3),
        m.position_stack(0.5, "all"),
        m.position_fill(),
    ],
    "facets": lambda m: [
        m.facet_wrap("c", ncol=2, scales="free", order=-1, format="{}", drop=False),
        m.facet_wrap(["c", "x"]),
        m.facet_grid("c", "x", x_order=-1, y_format="{}"),
    ],
    "coords": lambda m: [m.coord_flip(), m.coord_flip(xlim=[0, 1], ylim=[None, 2])],
    "toolbar": lambda m: [m.ggtb(), m.ggtb(size_basis="x", size_zoomin=-1)],
}


@pytest.mark.parametrize("name", FEATURE_CASES)
def test_feature_specs_match_lets_plot(name):
    ours, theirs = FEATURE_CASES[name](gg), FEATURE_CASES[name](lp)
    assert [spec.as_dict() for spec in ours] == [spec.as_dict() for spec in theirs]


def test_features_in_a_plot_match_lets_plot():
    def build(module):
        return (
            module.ggplot(FRAME, module.aes("x", "y"))
            + module.geom_point(position=module.position_jitterdodge(seed=1))
            + module.facet_wrap("c")
            + module.coord_flip()
            + module.ggtb(size_zoomin=-1)
        )

    assert without_data(build(gg).as_dict()) == without_data(build(lp).as_dict())


def plots(module):
    base = module.ggplot(FRAME, module.aes("x", "y")) + module.geom_point()
    return [base, base + module.ggtitle("second"), None]


GRID_CASES = {
    "one row": lambda m: m.gggrid(plots(m)),
    "ncol pads": lambda m: m.gggrid(plots(m), 2),
    "options": lambda m: m.gggrid(
        plots(m),
        ncol=2,
        sharex=True,
        sharey="row",
        widths=[1, 2],
        heights=[1, 1],
        hspace=4,
        vspace=5,
        fit=False,
        align=True,
        guides="collect",
    ),
    "plus features": lambda m: (
        m.gggrid(plots(m), 2)
        + m.ggsize(800, 400)
        + m.theme_classic()
        + m.theme(legend_position="bottom")
        + m.ggtitle("grid")
        + m.ggtb(size_zoomin=-1)
        + (m.labs(caption="cap") + m.ggsize(500, 300))
    ),
    "named theme keeps flavor": lambda m: m.gggrid(plots(m)) + m.flavor_darcula() + m.theme_bw(),
    "nested": lambda m: m.gggrid([m.gggrid(plots(m)), plots(m)[0]]),
}


@pytest.mark.parametrize("name", GRID_CASES)
def test_gggrid_matches_lets_plot(name):
    ours, theirs = GRID_CASES[name](gg), GRID_CASES[name](lp)
    assert isinstance(ours, SupPlotsSpec)
    assert without_data(ours.as_dict()) == without_data(theirs.as_dict())


def test_gggrid_semantics():
    grid = gg.gggrid(plots(gg))
    assert grid.as_dict()["figures"][0]["data"] is FRAME
    with pytest.raises(ValueError, match="empty"):
        gg.gggrid([])
    # Adding returns a copy; layers and unknown objects are rejected as in lets_plot.
    _ = grid + gg.ggsize(1, 1)
    assert "ggsize" not in grid.as_dict()
    with pytest.raises(TypeError):
        _ = grid + gg.geom_point()
    with pytest.raises(TypeError):
        _ = grid + object()


def test_geom_function_matches_lets_plot():
    def build(module):
        return [
            module.geom_function(fun=lambda x: x * x, n=5, xlim=[-1, 1], color="red"),
            module.geom_function(module.aes(color="c"), fun=abs),
            module.geom_function(module.aes(x=[1, 2]), fun=None, geom="point"),
        ]

    ours, theirs = build(gg), build(lp)
    for our_layer, their_layer in zip(ours, theirs, strict=True):
        our_spec, their_spec = our_layer.as_dict(), their_layer.as_dict()
        assert our_spec.pop("data") == their_spec.pop("data")
        assert our_spec == their_spec


def test_geom_function_on_frames():
    frame = FRAME.select("x")
    ours = gg.geom_function(gg.aes("x"), data=frame, fun=lambda x: 2 * x)
    theirs = lp.geom_function(lp.aes("x"), data=frame, fun=lambda x: 2 * x)
    assert ours.as_dict()["data"].equals(theirs.as_dict()["data"])
    pandas_frame = frame.to_pandas()
    ours = gg.geom_function(gg.aes("x"), data=pandas_frame, fun=lambda x: 2 * x)
    assert ours.as_dict()["data"]["y"].tolist() == [2.0, 4.0, 6.0]
    svg = (gg.ggplot() + gg.geom_function(fun=lambda x: x, n=3)).to_svg()
    assert "<svg" in svg


@pytest.mark.parametrize(
    ("layer", "message"),
    [
        (lambda: gg.geom_violin(), "geom 'violin' is not supported by ggrust yet"),
        (lambda: gg.geom_tile(), "geom 'tile' is not supported by ggrust yet"),
        (
            lambda: gg.geom_point(position=gg.position_jitter()),
            "position 'jitter' is not supported by ggrust yet",
        ),
    ],
)
def test_rendering_spec_only_parts_says_so(layer, message):
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + layer()
    with pytest.raises(ValueError, match=message):
        plot.to_svg()


def test_rendering_ignored_features_warns():
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point()
    for feature, message in [
        (gg.ggtb(), "ggtb"),
        (gg.facet_wrap("c"), "facet"),
        (gg.coord_flip(), "coord_flip"),
    ]:
        with pytest.warns(gg.GgrustWarning, match=message) as caught:
            (plot + feature).to_svg()
        assert len(caught) == 1, [str(item.message) for item in caught]


def test_rendering_a_grid_says_so():
    with pytest.raises(ValueError, match="gggrid"):
        gg.gggrid(plots(gg)).to_svg()


def test_ggsave_writes_by_extension(tmp_path):
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point()
    for name in ["plot.svg", "plot.png", "plot.html", " plot.HTM "]:
        written = gg.ggsave(plot, name, path=str(tmp_path / "out"))
        assert written == str(tmp_path / "out" / name.strip())
    assert (tmp_path / "out" / "plot.svg").read_text().startswith("<svg")
    assert (tmp_path / "out" / "plot.png").read_bytes()[:4] == b"\x89PNG"
    for name, message in [
        ("plot", "Missing file extension"),
        ("  ", "Malformed filename"),
        ("plot.pdf", "Unsupported file extension: 'pdf'"),
    ]:
        with pytest.raises(ValueError, match=message):
            gg.ggsave(plot, name, path=str(tmp_path))
    with pytest.raises(ValueError, match="PlotSpec or SupPlotsSpec expected"):
        gg.ggsave(gg.geom_point(), "plot.svg", path=str(tmp_path))


def test_ggsave_default_directory_is_lets_plots(tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point()
    assert gg.ggsave(plot, "a.svg") == str(tmp_path / "lets-plot-images" / "a.svg")


def test_unsupported_names_raise_on_use():
    with pytest.raises(NotImplementedError, match="geom_imshow"):
        gg.geom_imshow([[0.0]])
    with pytest.raises(NotImplementedError, match="gggrid"):
        gg.GGBunch()
    assert issubclass(gg.GGBunch, gg.plot.core.FeatureSpec)


def test_cellestial_module_paths():
    from ggrust.plot.core import FeatureSpec, LayerSpec  # noqa: F401
    from ggrust.plot.subplots import SupPlotsSpec as Imported

    assert Imported is gg.SupPlotsSpec
    assert isinstance(gg.gggrid([gg.ggplot()]), Imported)
    assert not isinstance(gg.gggrid([gg.ggplot()]), PlotSpec)


def test_geom_blank_has_no_tooltips_like_lets_plot():
    ours = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point() + gg.geom_blank(gg.aes(y="x"))
    theirs = lp.ggplot(FRAME, lp.aes("x", "y")) + lp.geom_point() + lp.geom_blank(lp.aes(y="x"))
    assert without_data(ours.as_dict()) == without_data(theirs.as_dict())
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        ours.to_svg()
