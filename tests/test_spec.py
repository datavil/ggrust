import importlib.util
import json
from pathlib import Path

import pytest
from ggrust._ggrust import validate_spec

REPOSITORY = Path(__file__).parents[1]
FIXTURE_DIRECTORY = REPOSITORY / "crates/ggrust-core/tests/fixtures/specs"
FIXTURE_PATHS = sorted(FIXTURE_DIRECTORY.glob("*.json"))


def load_generator():
    path = REPOSITORY / "tests/fixtures/generate_spec_fixtures.py"
    module_spec = importlib.util.spec_from_file_location("generate_spec_fixtures", path)
    module = importlib.util.module_from_spec(module_spec)
    module_spec.loader.exec_module(module)
    return module


def test_fixtures_exist():
    assert len(FIXTURE_PATHS) >= 17


@pytest.mark.parametrize("path", FIXTURE_PATHS, ids=lambda path: path.stem)
def test_fixture_is_valid_and_fully_supported(path):
    assert validate_spec(path.read_text()) == []


@pytest.mark.parametrize("build", load_generator().FIXTURES, ids=lambda build: build.__name__)
def test_fixtures_match_installed_lets_plot(build):
    # Fails when lets_plot changes its as_dict() output: regenerate and review the diff.
    generator = load_generator()
    live = json.loads(json.dumps(generator.strip_data(build().as_dict()), sort_keys=True))
    committed = json.loads((FIXTURE_DIRECTORY / f"{build.__name__}.json").read_text())
    assert live == committed


def test_invalid_spec_raises_with_path():
    spec = {"kind": "plot", "layers": [{"geom": "point", "mapping": {"colr": "a"}}]}
    with pytest.raises(ValueError, match=r"at 'layers\[0\]\.mapping.*unknown aesthetic 'colr'"):
        validate_spec(json.dumps(spec))


def test_unsupported_options_are_listed():
    spec = {"kind": "plot", "layers": [{"geom": "point", "sampling": {"name": "random"}}]}
    assert validate_spec(json.dumps(spec)) == ["layers[0].sampling"]


def test_not_json_raises():
    with pytest.raises(ValueError, match="not valid JSON"):
        validate_spec("{")


def lets_plot_json(spec):
    generator = load_generator()
    return json.dumps(generator.strip_data(spec.as_dict()))


def test_real_lets_plot_specs_found_by_bug_hunt():
    import numpy as np
    import polars as pl
    from lets_plot import (
        aes,
        as_discrete,
        facet_wrap,
        geom_imshow,
        geom_point,
        geom_text,
        ggplot,
        guide_legend,
        guides,
    )

    frame = pl.DataFrame({"x": [1.0, 2.0], "y": [3.0, 4.0], "c": ["a", "b"]})
    categorical = frame.with_columns(pl.col("c").cast(pl.Categorical))
    base = ggplot(frame, aes("x", "y"))
    supported = [
        base + geom_text(aes(label="c", hjust="c", vjust="c")),
        base + geom_point(aes(group=["c", "x"])),
        ggplot() + geom_imshow(np.random.default_rng(0).random((3, 3))),
        base + geom_point(aes(color="c")) + guides(color=guide_legend(ncol=2.0)),
        base + geom_point() + facet_wrap("c", ncol=2.0),
        base + geom_point(sampling="none"),
        ggplot(categorical, aes(x=as_discrete("c", levels=["b", "a"]), y="y")) + geom_point(),
    ]
    for spec in supported:
        assert validate_spec(lets_plot_json(spec)) == []


def test_real_lets_plot_theme_error_names_element():
    from lets_plot import element_text, geom_point, ggplot, theme

    spec = ggplot() + geom_point() + theme(text=element_text(size="12"))
    with pytest.raises(ValueError, match=r"at 'theme\.text\.size'"):
        validate_spec(lets_plot_json(spec))


def test_real_lets_plot_font_metainfo_is_reported():
    from lets_plot import font_family_info, geom_point, ggplot

    spec = ggplot() + geom_point() + font_family_info("Arial", mono=False)
    assert validate_spec(lets_plot_json(spec)) == ["metainfo_list[0]"]
