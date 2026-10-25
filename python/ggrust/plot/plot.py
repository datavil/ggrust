"""``ggplot()`` and ``ggsize()``.

Ported from lets-plot's ``lets_plot/plot/plot.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

import numbers

from ggrust.plot.core import FeatureSpec, PlotSpec
from ggrust.plot.util import as_annotated_data, key_int2str

__all__ = ["ggplot", "ggsize"]

# lets-plot's default limits for ggsize.
MAX_WIDTH = 10_000
MAX_HEIGHT = 10_000


def ggplot(data=None, mapping=None) -> PlotSpec:
    """Create a new plot.

    Parameters
    ----------
    data : DataFrame or dict, optional
        Data shared by all layers: polars or pandas DataFrame, pyarrow Table, any
        object exporting ``__arrow_c_stream__``, or a dict of columns.
    mapping : FeatureSpec, optional
        Default aesthetic mapping from ``aes()``.

    Returns
    -------
    PlotSpec
        The plot. Add layers, scales and features with ``+``.
    """
    if isinstance(data, FeatureSpec):
        message = f"Object {data.kind!r} is not acceptable as 'data' argument in ggplot()"
        raise ValueError(message)
    data = key_int2str(data)
    data, mapping, data_meta = as_annotated_data(data, mapping)
    return PlotSpec(data, mapping, scales=[], layers=[], **data_meta)


def ggsize(width, height) -> FeatureSpec:
    """Set the plot size in pixels.

    Parameters
    ----------
    width, height : float
        Size in pixels, at most 10,000 each.

    Returns
    -------
    FeatureSpec
        The size feature.
    """
    if not isinstance(width, numbers.Number) or not isinstance(height, numbers.Number):
        message = f"'width' and 'height' must be numeric, got {width!r} and {height!r}"
        raise TypeError(message)
    if width > MAX_WIDTH or height > MAX_HEIGHT:
        message = f"'width' and 'height' must be at most {MAX_WIDTH}, got {width} and {height}"
        raise ValueError(message)
    return FeatureSpec("ggsize", name=None, width=width, height=height)
