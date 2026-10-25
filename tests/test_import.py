import ggrust
from packaging.version import Version


def test_engine_version_matches_package_version():
    # maturin rewrites semver pre-releases to PEP 440 (0.2.0-alpha.1 -> 0.2.0a1).
    assert Version(ggrust.engine_version()) == Version(ggrust.__version__)


def test_public_names_are_exported():
    for name in ["ggplot", "aes", "geom_point", "ggsize", "labs", "LetsPlot", "engine_version"]:
        assert name in ggrust.__all__
        assert hasattr(ggrust, name)
