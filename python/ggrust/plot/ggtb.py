"""``ggtb()``: the interactive zoom/pan toolbar.

Ported from lets-plot's ``lets_plot/plot/ggtb_.py`` (MIT, see THIRD_PARTY_NOTICES.md).
HTML output gets lets-plot's toolbar (pan, rubber band zoom, centerpoint zoom,
reset); static output (SVG, PNG) ignores it, as lets-plot's does. ``layers=`` adds
layer visibility toggles, a ggrust extension (lets-plot has none).
"""

import math
from collections.abc import Mapping
from typing import Literal

from ggrust.errors import SpecError, did_you_mean
from ggrust.plot.core import FeatureSpec

__all__ = ["ggtb"]

SIZE_BASES = ("x", "y", "min", "max")


def ggtb(
    size_basis: Literal["x", "y", "min", "max"] | None = None,
    size_zoomin: int | float | None = None,
    layers: bool | Mapping[str, int] | None = None,
) -> FeatureSpec:
    """Add the zoom/pan toolbar to an HTML plot.

    The toolbar sits above the plot. With a tool selected, dragging in a panel pans
    or zooms it and the mouse wheel zooms around the cursor; axes and grid lines
    are redrawn for the new range. Reset restores the original view.

    Parameters
    ----------
    size_basis : {"x", "y", "min", "max"}, default="max"
        Which axis' zoom factor point sizes follow (the smaller or larger of both
        for ``"min"`` / ``"max"``).
    size_zoomin : int or float, default=0
        How much points grow when zooming in: 0 keeps their size, -1 grows them
        without limit, a positive number caps the growth factor.
    layers : bool or dict of str to int, default=None
        The layers menu in the toolbar that hides and shows each layer (a
        ggrust extension, lets-plot has none). It is on by default (``None`` or
        ``True``); ``False`` removes it. Layers are labelled by their
        geom (``"imshow"``, ``"point"``, ``"point 2"``); a dict gives labels to
        layer indices, counted from 0 in the order the layers were added
        (``{"tissue": 0, "spots": 1}``); unlabelled layers keep their geom name.
        In a ``gggrid`` every subplot gets its own menu. Legends stay as drawn.

    Returns
    -------
    FeatureSpec
        The toolbar feature (kind ``"ggtoolbar"``).

    Raises
    ------
    ValueError
        If ``size_basis`` is not one of the accepted names, or ``size_zoomin`` is
        not a finite number >= 0 or -1, or ``layers`` is not a bool or a dict
        of string labels to distinct integer indices >= 0.

    Examples
    --------
    >>> from ggrust import *
    >>> plot = ggplot({"x": [1, 2, 3], "y": [3, 1, 2]}, aes("x", "y")) + geom_point()
    >>> html = (plot + ggtb(size_zoomin=-1)).to_html()

    Toggle the image under a scatter, with custom labels:

    >>> import numpy as np
    >>> image = np.zeros((3, 4, 3), dtype=np.uint8)
    >>> layered = (
    ...     ggplot({"x": [1, 2], "y": [1, 2]}, aes("x", "y"))
    ...     + geom_imshow(image_data=image)
    ...     + geom_point()
    ... )
    >>> html = (layered + ggtb(layers={"tissue": 0, "spots": 1})).to_html()
    """
    if size_basis is not None and size_basis not in SIZE_BASES:
        raise SpecError(
            f"ggtb(size_basis={size_basis!r}): unknown size_basis; "
            f"use one of 'x', 'y', 'min', 'max'{did_you_mean(str(size_basis), SIZE_BASES)}"
        )
    valid_number = (
        isinstance(size_zoomin, int | float)
        and not isinstance(size_zoomin, bool)
        and math.isfinite(size_zoomin)
        and (size_zoomin >= 0 or size_zoomin == -1)
    )
    if size_zoomin is not None and not valid_number:
        raise SpecError(
            f"ggtb(size_zoomin={size_zoomin!r}): use a finite number >= 0 (largest point growth, "
            "0 keeps sizes) or -1 (no limit)"
        )
    checked_layers = _checked_layers(layers)
    options: dict[str, object] = {"size_basis": size_basis, "size_zoomin": size_zoomin}
    if checked_layers is not None:
        options["layers"] = checked_layers
    return FeatureSpec("ggtoolbar", name=None, **options)


def _checked_layers(layers: bool | Mapping[str, int] | None) -> bool | dict[str, int] | None:
    """``layers`` as the spec stores it, or raise ``SpecError`` when it is malformed."""
    if layers is None or isinstance(layers, bool):
        return layers
    accepted = 'use True, False, or a dict of labels to layer indices such as {"spots": 1}'
    if not isinstance(layers, Mapping):
        raise SpecError(
            f"ggtb(layers={layers!r}): {type(layers).__name__} is not accepted; {accepted}"
        )
    labelled: dict[int, str] = {}
    for label, index in layers.items():
        if not isinstance(label, str):
            raise SpecError(f"ggtb(layers=...): label {label!r} is not a string; {accepted}")
        if isinstance(index, bool) or not isinstance(index, int) or index < 0:
            raise SpecError(
                f"ggtb(layers=...): {label!r} maps to {index!r}, not a layer index; "
                "use an integer >= 0 (layers count from 0 in the order they were added)"
            )
        if index in labelled:
            raise SpecError(
                f"ggtb(layers=...): layer {index} is labelled both {labelled[index]!r} and "
                f"{label!r}; give each layer one label"
            )
        labelled[index] = label
    return dict(layers)
