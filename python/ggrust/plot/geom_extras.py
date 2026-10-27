"""Helpers for geoms, ported from lets-plot's ``lets_plot/plot/geom_extras.py``."""

from ggrust.plot.core import FeatureSpec

__all__ = ["arrow"]


def arrow(angle=None, length=None, ends=None, type=None) -> FeatureSpec:
    """Describe the arrow heads of a line (``geom_segment``, ``geom_path``).

    Parameters
    ----------
    angle : float, optional
        Angle of the arrow head in degrees (default 30).
    length : float, optional
        Length of the arrow head in pixels (default 10).
    ends : {"last", "first", "both"}, optional
        Which ends of the line get a head (default ``"last"``).
    type : {"open", "closed"}, optional
        Whether the head is a closed triangle (default ``"open"``).

    Returns
    -------
    FeatureSpec
        The arrow specification.
    """
    return FeatureSpec("arrow", "arrow", angle=angle, length=length, ends=ends, type=type)
