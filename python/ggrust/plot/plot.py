"""``ggplot()``, ``ggsize()`` and the deprecated ``GGBunch``.

Ported from lets-plot's ``lets_plot/plot/plot.py`` (MIT, see THIRD_PARTY_NOTICES.md).
"""

from __future__ import annotations

import numbers
from typing import TYPE_CHECKING

from ggrust.errors import SpecError
from ggrust.plot.core import FeatureSpec, PlotSpec
from ggrust.plot.util import as_annotated_data, key_int2str

if TYPE_CHECKING:
    from ggrust._typing import Number, PlotData

__all__ = ["GGBunch", "ggplot", "ggsize"]

# lets-plot's default limits for ggsize.
MAX_WIDTH = 10_000
MAX_HEIGHT = 10_000


def ggplot(data: PlotData | None = None, mapping: FeatureSpec | None = None) -> PlotSpec:
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

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes(x="weight", y="mpg")) + geom_point()
    """
    if isinstance(data, FeatureSpec):
        message = (
            f"ggplot(): 'data' got a {data.kind!r} spec; pass a DataFrame or dict as data and "
            "add specs with + (e.g. ggplot(frame, aes('x', 'y')) + geom_point())"
        )
        raise SpecError(message, path="data", reason=message)
    data = key_int2str(data)
    data, mapping, data_meta = as_annotated_data(data, mapping)
    return PlotSpec(data, mapping, scales=[], layers=[], **data_meta)


def ggsize(width: Number, height: Number) -> FeatureSpec:
    """Set the plot size in pixels.

    Parameters
    ----------
    width, height : float
        Size in pixels, at most 10,000 each (NaN is rejected, as in lets_plot).

    Returns
    -------
    FeatureSpec
        The size feature.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point() + ggsize(400, 250)
    """
    # Same checks and exception type (AssertionError) as lets_plot.
    if not isinstance(width, numbers.Number):
        raise AssertionError("'width' must be numeric")
    if not isinstance(height, numbers.Number):
        raise AssertionError("'height' must be numeric")
    if not width <= MAX_WIDTH:
        raise AssertionError(f"'width' must be less than or equal to {MAX_WIDTH}")
    if not height <= MAX_HEIGHT:
        raise AssertionError(f"'height' must be less than or equal to {MAX_HEIGHT}")
    return FeatureSpec("ggsize", name=None, width=width, height=height)


class GGBunch(FeatureSpec):
    """A free-layout bunch of plots. Not supported yet; use ``gggrid`` for a grid.

    Exported so code that names it (type hints, ``isinstance``) keeps importing.

    Examples
    --------
    >>> from ggrust import *
    >>> try:
    ...     GGBunch()
    ... except NotImplementedError:
    ...     print("use gggrid() instead")
    use gggrid() instead
    """

    def __init__(self) -> None:
        message = "GGBunch is not supported by ggrust yet; use gggrid() to lay out plots in a grid"
        raise NotImplementedError(message)
