"""Type stubs for the native extension module."""

__all__ = ["engine_version", "validate_spec"]

def engine_version() -> str:
    """Return the version of the Rust engine."""

def validate_spec(spec_json: str) -> list[str]:
    """Parse a plot spec and return the paths of unsupported options.

    Parameters
    ----------
    spec_json
        JSON of a lets_plot-style `as_dict()` spec, without inline data.

    Returns
    -------
    list[str]
        Dotted paths of options ggrust does not support, e.g. ``"layers[0].sampling"``.

    Raises
    ------
    ValueError
        If the spec is invalid; the message names the offending path.
    """
