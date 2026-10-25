"""ggrust: grammar-of-graphics plotting in Rust with a lets_plot-compatible API."""

from importlib.metadata import version as _distribution_version

from ggrust._ggrust import engine_version

__version__ = _distribution_version("ggrust")

__all__ = ["__version__", "engine_version"]
