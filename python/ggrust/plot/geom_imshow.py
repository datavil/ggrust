"""``geom_imshow()``: draw an image array as a ``geom_image`` layer.

Ported from lets-plot's ``lets_plot/plot/geom_imshow_.py`` (MIT, see THIRD_PARTY_NOTICES.md):
the array becomes an RGBA PNG data URL stretched over ``extent``; a greyscale image adds a
``paint_c`` color scale for its legend. Named color maps (``cmap="viridis"``) need the
optional ``palettable`` package, as in lets_plot. The PNG is written with ``zlib`` (no
``pypng`` needed).
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Literal

from ggrust.plot.core import FeatureSpec, LayerSpec, aes
from ggrust.plot.geom import _geom
from ggrust.plot.scale import _scale

if TYPE_CHECKING:
    import numpy

__all__ = ["geom_imshow"]


def _as_boolean(value: object, *, default: bool) -> bool:
    """lets_plot ``as_boolean``: None is ``default``, the string ``'False'`` is False."""
    if value is None:
        return default
    return bool(value) and value != "False"


def _parse_hex_color(hex_color: str, alpha: float | None = None) -> numpy.ndarray:
    """``#RGB`` or ``#RRGGBB`` as uint8 RGB, or RGBA when ``alpha`` (0..255) is given."""
    import numpy

    digits = hex_color.lstrip("#")
    if len(digits) == 3:
        digits = "".join(digit + digit for digit in digits)
    channels = [int(digits[index : index + 2], 16) for index in (0, 2, 4)]
    if alpha is not None:
        channels.append(int(alpha + 0.5))
    return numpy.array(channels, dtype=numpy.uint8)


def _parse_rgb_rgba_color(color: str, alpha: float | None = None) -> numpy.ndarray:
    """``rgb(r, g, b)`` or ``rgba(r, g, b, a)`` as uint8 channels; ``alpha`` scales a."""
    import numpy

    inner = color[5:-1] if color.startswith("rgba(") else color[4:-1]
    parts = [part.strip() for part in inner.split(",")]
    if len(parts) == 3:
        channels = [int(part) for part in parts]
        if alpha is not None:
            channels.append(int(alpha + 0.5))
    elif len(parts) == 4:
        red, green, blue, color_alpha = (int(part) for part in parts)
        channels = [red, green, blue]
        channels.append(int(color_alpha * alpha / 255 + 0.5) if alpha is not None else color_alpha)
    else:
        message = f"Invalid rgb/rgba format: {color}"
        raise ValueError(message)
    return numpy.array(channels, dtype=numpy.uint8)


def _parse_color(color: str, alpha: float | None = None) -> numpy.ndarray:
    """Parse a ``cmap`` list entry (hex, ``rgb()`` or ``rgba()``) into uint8 channels."""
    color = color.strip()
    if color.startswith("#"):
        return _parse_hex_color(color, alpha)
    if color.startswith("rgb"):
        return _parse_rgb_rgba_color(color, alpha)
    message = f"Unsupported color format: {color}. Expected hex (#RRGGBB), rgb(), or rgba()"
    raise ValueError(message)


def _normalize_2d(
    image_data: numpy.ndarray,
    norm: bool | None,
    vmin: float | None,
    vmax: float | None,
    min_lum: int,
) -> tuple[numpy.ndarray, float, float]:
    """Scale a greyscale image to luminance ``min_lum..255-min_lum``.

    Returns the new image and the data range (``vmin``, ``vmax``) the legend shows.
    """
    import numpy

    min_lum = max(0, min_lum)
    max_lum = 255 - min_lum
    vmin = float(vmin if vmin is not None else numpy.nanmin(image_data))
    vmax = float(vmax if vmax is not None else numpy.nanmax(image_data))
    if vmin > vmax:
        message = f"vmin value must be less than vmax value, was: {vmin} > {vmax}"
        raise ValueError(message)
    if _as_boolean(norm, default=True):
        if vmin == vmax:
            image_data = numpy.copy(image_data)
            image_data[numpy.isfinite(image_data)] = 127
        else:
            if image_data.dtype.kind == "f":
                image_data = numpy.copy(image_data)
            else:
                image_data = image_data.astype(numpy.float32)
            image_data.clip(vmin, vmax, out=image_data)
            image_data -= vmin
            image_data *= max_lum / (vmax - vmin)
            image_data += min_lum
    else:
        image_data = numpy.copy(image_data)
        image_data.clip(min_lum, max_lum, out=image_data)
        vmin = float(numpy.nanmin(image_data))
        vmax = float(numpy.nanmax(image_data))
    return image_data, vmin, vmax


def _palettable_map(name: str, size: int) -> list[str]:
    """Hex colors of matplotlib color map ``name`` sampled at ``size`` points (palettable)."""
    try:
        import palettable
    except ImportError as error:
        message = (
            "Can't process `cmap`: please install 'Palettable' "
            "(https://pypi.org/project/palettable/) to your Python environment."
        )
        raise ValueError(message) from error
    return palettable.matplotlib.get_map(f"{name}_{size}").hex_colors


def _png_data_url(rgba: numpy.ndarray, compression: int | None) -> str:
    """Encode an ``(height, width, 4)`` uint8 array as a ``data:image/png;base64,...`` URL.

    Encoded in Rust with the fastest deflate by default: the PNG differs byte-wise
    from lets-plot's (pure zlib) but decodes to the same pixels.
    """
    import numpy

    from ggrust import _ggrust

    height, width, _ = rgba.shape
    data = numpy.ascontiguousarray(rgba, dtype=numpy.uint8).tobytes()
    return _ggrust.png_data_url(data, width, height, compression)


def geom_imshow(
    image_data: numpy.ndarray,
    cmap: str | list[str] | None = None,
    *,
    norm: bool | None = None,
    alpha: float | None = None,
    vmin: float | None = None,
    vmax: float | None = None,
    extent: list[float] | tuple[float, float, float, float] | None = None,
    compression: int | None = None,
    show_legend: bool = True,
    color_by: Literal["paint_a", "paint_b", "paint_c"] = "paint_c",
    cguide: dict | None = None,
    breaks: list | dict | None = None,
    labels: list | dict | None = None,
    lablim: int | None = None,
    format: str | None = None,
) -> LayerSpec | FeatureSpec:
    """Display an image array, as lets_plot ``geom_imshow``.

    Parameters
    ----------
    image_data : ndarray
        2D greyscale image (``height x width``), or 3D RGB / RGBA (``height x width x 3|4``).
        Float RGB(A) values are in 0..1, integer ones in 0..255.
    cmap : str or list of str, optional
        Greyscale only: a matplotlib color map name (needs ``palettable``) or a list of
        colors (hex, ``rgb()`` or ``rgba()``). Default: shades of grey.
    norm : bool, default=True
        Greyscale only: stretch ``vmin..vmax`` to the full luminance range. False keeps the
        values as luminance (clipped to 0..255).
    alpha : float, optional
        Image opacity in 0..1.
    vmin, vmax : float, optional
        Greyscale only: the data range the color map covers; the image's min / max by default.
    extent : list of 4 float, optional
        ``[x0, x1, y0, y1]`` of the image box in data units. Default: pixel centers at
        integers (``[-0.5, width - 0.5, -0.5, height - 0.5]``). ``x0 > x1`` or ``y0 > y1``
        flips the image.
    compression : int, optional
        PNG compression level 0..9.
    show_legend : bool, default=True
        Greyscale only: show a colorbar.
    color_by : {"paint_a", "paint_b", "paint_c"}, default="paint_c"
        The aesthetic the colorbar uses.
    cguide, breaks, labels, lablim, format : optional
        Colorbar guide and labels, as in lets_plot.

    Returns
    -------
    LayerSpec or FeatureSpec
        The ``geom_image`` layer, plus its color scale for a greyscale image with a legend.

    Raises
    ------
    ValueError
        When ``image_data`` is not a 2D/3D ndarray, ``alpha`` or ``compression`` is out of
        range, ``vmin > vmax``, ``cmap`` is invalid, or ``extent`` is not 4 numbers.

    Examples
    --------
    >>> import numpy as np
    >>> from ggrust import ggplot, geom_imshow
    >>> image = np.random.default_rng(0).random((20, 30))
    >>> plot = ggplot() + geom_imshow(image, cmap=["#000000", "#ff8800"])
    """
    import numpy

    if not isinstance(image_data, numpy.ndarray):
        message = f"Invalid image_data: ndarray is expected but was {type(image_data)}"
        raise ValueError(message)
    if image_data.ndim not in (2, 3):
        message = (
            f"Invalid image_data: 2d or 3d array is expected but was {image_data.ndim}-dimensional"
        )
        raise ValueError(message)
    if image_data.shape[0] == 0 or image_data.shape[1] == 0:
        message = (
            f"Invalid image_data: expected at least 1 x 1 pixels but the shape was "
            f"{image_data.shape}"
        )
        raise ValueError(message)
    if alpha is not None and not 0 <= alpha <= 1:
        message = f"Invalid alpha: expected float in range [0..1] but was {alpha}"
        raise ValueError(message)
    if compression is not None and not 0 <= compression <= 9:
        message = f"Invalid compression: expected integer in range [0..9] but was {compression}"
        raise ValueError(message)

    greyscale = image_data.ndim == 2
    cmap_list = isinstance(cmap, list)
    cmap_palettable = isinstance(cmap, str)
    greyscale_data_min = greyscale_data_max = 0.0
    if greyscale:
        has_nan = bool(numpy.isnan(image_data.max()))
        min_lum = 1 if has_nan and cmap else 0  # palette index 0 is kept for NaN
        image_data, greyscale_data_min, greyscale_data_max = _normalize_2d(
            image_data, norm, vmin, vmax, min_lum
        )
        height, width = image_data.shape
        has_nan = bool(numpy.isnan(image_data.max()))
        if cmap and not cmap_list and not cmap_palettable:
            kind = type(cmap).__name__
            message = f"cmap must be a string (colormap name) or a list of colors, but was {kind}"
            raise ValueError(message)
        if cmap:
            alpha_channel = 255 if alpha is None else 255 * alpha
            size = 255 if has_nan else 256
            if cmap_palettable:
                palette = [
                    _parse_hex_color(color, alpha_channel) for color in _palettable_map(cmap, size)
                ]
            else:
                n_colors = len(cmap)
                if n_colors == 0:
                    message = "cmap list must contain at least one color"
                    raise ValueError(message)
                palette = [
                    _parse_color(cmap[min(index * n_colors // size, n_colors - 1)], alpha_channel)
                    for index in range(size)
                ]
            if has_nan:
                palette = [numpy.array([0, 0, 0, 0], dtype=numpy.uint8), *palette]
                numpy.nan_to_num(image_data, copy=False, nan=0)
            image_data = numpy.take(palette, numpy.round(image_data).astype(numpy.int32), axis=0)
        else:
            is_nan = numpy.isnan(image_data)
            alpha_channel = numpy.zeros(image_data.shape, dtype=image_data.dtype)
            alpha_channel[~is_nan] = 255 * (1 if alpha is None else alpha)
            image_data[is_nan] = 0
            image_data = numpy.repeat(image_data[:, :, numpy.newaxis], 3, axis=2)
            image_data = numpy.dstack((image_data, alpha_channel))
    else:
        image_data = numpy.copy(image_data)
        if image_data.dtype.kind == "f":
            image_data *= 255
        height, width, channel_count = image_data.shape
        if channel_count == 3:
            opacity = 255 * (1 if alpha is None else alpha)
            alpha_channel = numpy.full((height, width, 1), opacity, dtype=image_data.dtype)
            image_data = numpy.dstack((image_data, alpha_channel))
        elif channel_count == 4 and alpha is not None:
            if image_data.dtype.kind != "f":
                image_data = image_data.astype(numpy.float32)
            image_data[:, :, 3] *= alpha
        elif channel_count != 4:
            message = (
                f"Invalid image_data: 3 (RGB) or 4 (RGBA) channels expected but was {channel_count}"
            )
            raise ValueError(message)

    image_data.clip(0, 255, out=image_data)

    ext_x0, ext_x1, ext_y0, ext_y1 = -0.5, width - 0.5, -0.5, height - 0.5
    if extent:
        try:
            ext_x0, ext_x1, ext_y0, ext_y1 = (float(value) for value in extent)
        except (TypeError, ValueError) as error:
            message = f"Invalid `extent`: list of 4 numbers expected: {error}"
            raise ValueError(message) from error
    if ext_x0 > ext_x1:
        image_data = numpy.flip(image_data, axis=1)
        ext_x0, ext_x1 = ext_x1, ext_x0
    if ext_y0 > ext_y1:
        image_data = numpy.flip(image_data, axis=0)
        ext_y0, ext_y1 = ext_y1, ext_y0

    if image_data.dtype.kind == "f":
        image_data = image_data + 0.5
    rgba = image_data.astype(numpy.uint8)
    href = _png_data_url(rgba, compression)

    show_legend = _as_boolean(show_legend, default=True)
    normalize = _as_boolean(norm, default=True)
    color_scale = None
    color_scale_mapping = None
    if greyscale and show_legend:
        color_scale_mapping = aes(**{color_by: [greyscale_data_min, greyscale_data_max]})
        guide_options: dict[str, object] = {
            "name": "",
            "guide": cguide,
            "breaks": breaks,
            "labels": labels,
            "lablim": lablim,
            "format": format,
        }
        if cmap_palettable and normalize:
            colors = _palettable_map(cmap, 32)
            color_scale = _scale(
                color_by, colors=colors, scale_mapper_kind="color_gradientn", **guide_options
            )
        elif cmap_palettable:
            colors = _palettable_map(cmap, 256)
            start = max(0, round(greyscale_data_min))
            end = min(255, round(greyscale_data_max))
            colors = colors[start:end]
            if len(colors) > 32:
                colors = [
                    colors[index] for index in numpy.linspace(0, len(colors) - 1, 32, dtype=int)
                ]
            color_scale = _scale(
                color_by, colors=colors, scale_mapper_kind="color_gradientn", **guide_options
            )
        elif cmap_list:
            color_scale = _scale(color_by, values=cmap, **guide_options)
        else:
            start = 0 if normalize else greyscale_data_min / 255.0
            end = 1 if normalize else greyscale_data_max / 255.0
            color_scale = _scale(
                color_by, start=start, end=end, scale_mapper_kind="color_grey", **guide_options
            )

    layer = _geom(
        "image",
        mapping=color_scale_mapping,
        href=href,
        xmin=ext_x0,
        ymin=ext_y0,
        xmax=ext_x1,
        ymax=ext_y1,
        show_legend=show_legend,
        inherit_aes=False,
        color_by=color_by if show_legend and greyscale else None,
    )
    if color_scale is not None:
        return layer + color_scale
    return layer
