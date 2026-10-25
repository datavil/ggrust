import ggrust


def test_engine_version_matches_package_version():
    assert ggrust.engine_version() == ggrust.__version__


def test_public_names_are_exported():
    assert set(ggrust.__all__) == {"__version__", "engine_version"}
