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
