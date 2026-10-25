"""Type stubs for the native extension module."""

__all__ = ["engine_version"]

def engine_version() -> str:
    """Return the version of the Rust engine."""
