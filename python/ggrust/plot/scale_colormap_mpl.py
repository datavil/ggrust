"""``scale_color_cmapmpl`` and ``scale_fill_cmapmpl``: any matplotlib color map.

Ported from lets-plot's ``lets_plot/plot/scale_colormap_mpl.py`` (MIT, see
THIRD_PARTY_NOTICES.md). The map is sampled into a list of colors here and the scale is
an ordinary ``gradientn``. matplotlib is only imported when one of these is called.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Protocol

from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.scale_color import scale_color_gradientn, scale_fill_gradientn

if TYPE_CHECKING:
    from collections.abc import Sequence

    from ggrust._typing import Breaks, Color, Guide, Labels, Limits, Number, Transform
    from ggrust.plot.core import FeatureSpec

__all__ = ["scale_color_cmapmpl", "scale_fill_cmapmpl"]

_SAMPLES = 256


class Colormap(Protocol):
    """A matplotlib color map: maps a share in 0..1 to an RGBA tuple."""

    def __call__(self, value: float) -> Sequence[float]:
        """Return the color at ``value``."""
        ...


def _colormap_colors(cmap: str | Colormap) -> list[str]:
    """Return the hex colors of ``cmap``: a listed map's own, else 256 samples."""
    try:
        import matplotlib  # ty: ignore[unresolved-import]
        import matplotlib.colors  # ty: ignore[unresolved-import]
    except ImportError as error:
        message = (
            "scale_*_cmapmpl needs matplotlib to resolve `cmap`: install it "
            "(`pip install matplotlib`), or pass the colors to scale_*_gradientn(colors=[...])"
        )
        raise SpecError(message) from error
    resolved = cmap
    if isinstance(cmap, str):
        names = list(matplotlib.colormaps)
        if cmap not in names:
            hint = did_you_mean(cmap, names) or "; see matplotlib.colormaps for the names"
            raise SpecError(f"unknown colormap '{cmap}' at 'cmap'{hint}")
        resolved = matplotlib.colormaps[cmap]
    if not callable(resolved):
        message = (
            f"expected a colormap name or a matplotlib colormap at 'cmap', "
            f"got {type(cmap).__name__}"
        )
        raise SpecError(message)
    if isinstance(resolved, matplotlib.colors.ListedColormap):
        colors = resolved.colors
    else:
        colors = [resolved(index / (_SAMPLES - 1)) for index in range(_SAMPLES)]
    if len(colors) < 2:
        message = (
            f"the colormap at 'cmap' has {len(colors)} color, a gradient needs two or more; "
            "use a longer colormap, or scale_*_manual for a single color"
        )
        raise SpecError(message)
    return [matplotlib.colors.to_hex(color) for color in colors]


def scale_color_cmapmpl(
    cmap: str | Colormap,
    *,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``color`` through a matplotlib color map.

    Parameters
    ----------
    cmap : str or matplotlib colormap
        A name in ``matplotlib.colormaps``, or a colormap object.
    name : str, optional
        The legend title.
    breaks : list or dict, optional
        Legend breaks, or a mapping of labels to breaks.
    labels : list or dict, optional
        Legend labels.
    lablim : int, optional
        Longest label, in characters.
    limits : list, optional
        The range of the scale.
    na_value : str, optional
        Color of missing values.
    guide : str or FeatureSpec, optional
        ``guide_legend()`` / ``guide_colorbar()`` spec.
    trans : str, optional
        Transformation of the data: ``identity``, ``log10``, ``log2``, ``log``,
        ``log1p``, ``exp``, ``logit``, ``probit``, ``reciprocal``, ``pseudo_log``,
        ``symlog``, ``sqrt``, ``reverse``, or ``boxcox_trans(p)``.
    format : str, optional
        Label format.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If matplotlib is not installed or ``cmap`` is not a known color map.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3], "y": [1, 2, 3], "z": [1, 2, 3]}
    >>> plot = ggplot(data, aes("x", "y", color="z")) + geom_point()
    >>> plot = plot + scale_color_cmapmpl("plasma")  # doctest: +SKIP
    """
    return scale_color_gradientn(
        colors=_colormap_colors(cmap),
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
    )


def scale_fill_cmapmpl(
    cmap: str | Colormap,
    *,
    name: str | None = None,
    breaks: Breaks | None = None,
    labels: Labels | None = None,
    lablim: int | None = None,
    limits: Limits | None = None,
    na_value: Color | Number | None = None,
    guide: Guide | None = None,
    trans: Transform | None = None,
    format: str | None = None,
) -> FeatureSpec:
    """Map ``fill`` through a matplotlib color map.

    Parameters
    ----------
    cmap : str or matplotlib colormap
        A name in ``matplotlib.colormaps``, or a colormap object.
    name : str, optional
        The legend title.
    breaks : list or dict, optional
        Legend breaks, or a mapping of labels to breaks.
    labels : list or dict, optional
        Legend labels.
    lablim : int, optional
        Longest label, in characters.
    limits : list, optional
        The range of the scale.
    na_value : str, optional
        Color of missing values.
    guide : str or FeatureSpec, optional
        ``guide_legend()`` / ``guide_colorbar()`` spec.
    trans : str, optional
        Transformation of the data: ``identity``, ``log10``, ``log2``, ``log``,
        ``log1p``, ``exp``, ``logit``, ``probit``, ``reciprocal``, ``pseudo_log``,
        ``symlog``, ``sqrt``, ``reverse``, or ``boxcox_trans(p)``.
    format : str, optional
        Label format.

    Returns
    -------
    FeatureSpec
        The scale.

    Raises
    ------
    SpecError
        If matplotlib is not installed or ``cmap`` is not a known color map.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"x": [1, 2, 3], "y": [1, 1, 1], "z": [1, 2, 3]}
    >>> plot = ggplot(data, aes("x", "y", fill="z")) + geom_tile()
    >>> plot = plot + scale_fill_cmapmpl("Dark2")  # doctest: +SKIP
    """
    return scale_fill_gradientn(
        colors=_colormap_colors(cmap),
        name=name,
        breaks=breaks,
        labels=labels,
        lablim=lablim,
        limits=limits,
        na_value=na_value,
        guide=guide,
        trans=trans,
        format=format,
    )
