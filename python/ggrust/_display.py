"""Notebook display mode: what ``_repr_html_`` shows. Does not affect ``to_*`` or ``ggsave``."""

from __future__ import annotations

from typing import Literal, get_args

from ggrust.errors import SpecError, did_you_mean

DisplayMode = Literal["html", "svg", "png"]

_MODES: tuple[str, ...] = get_args(DisplayMode)
_current: list[DisplayMode] = ["html"]


def display(mode: DisplayMode) -> None:
    """Choose how plots show in notebooks, for the rest of the session.

    Parameters
    ----------
    mode : {"html", "svg", "png"}
        ``"html"`` (the default) is interactive: tooltips and the zoom/pan toolbar.
        ``"svg"`` is static vector output with no JavaScript, for viewers that block
        scripts (GitHub previews, some exporters). ``"png"`` is a static image.

    Raises
    ------
    SpecError
        If ``mode`` is not one of the accepted values.

    Examples
    --------
    >>> import ggrust
    >>> ggrust.display("svg")
    >>> ggrust.display("html")
    """
    if mode not in _MODES:
        accepted = ", ".join(repr(name) for name in _MODES)
        reason = (
            f"unknown display mode {mode!r}{did_you_mean(str(mode), _MODES)}; accepted: {accepted}"
        )
        raise SpecError(f"display: {reason}", path="display", reason=reason)
    _current[0] = mode


def current_display() -> DisplayMode:
    """Return the notebook display mode set by :func:`display`.

    Returns
    -------
    {"html", "svg", "png"}
        The active mode, ``"html"`` unless changed.
    """
    return _current[0]
