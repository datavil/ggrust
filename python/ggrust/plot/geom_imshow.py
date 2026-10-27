"""``geom_imshow()``: draw an image array (not supported yet, plan.md step 5.2).

Exported so code written for lets_plot imports; calling it raises.
"""

__all__ = ["geom_imshow"]


def geom_imshow(
    image_data,
    cmap=None,
    *,
    norm=None,
    alpha=None,
    vmin=None,
    vmax=None,
    extent=None,
    compression=None,
    show_legend=True,
    color_by="paint_c",
    cguide=None,
    breaks=None,
    labels=None,
    lablim=None,
    format=None,
):
    """Display an image array. Not supported yet.

    Parameters
    ----------
    image_data : ndarray
        Image as in lets_plot (2D grayscale, or 3D RGB/RGBA).
    cmap, norm, alpha, vmin, vmax, extent : optional
        As in lets_plot, like the remaining keyword arguments.

    Raises
    ------
    NotImplementedError
        Always.
    """
    message = "geom_imshow is not supported by ggrust yet (plan.md step 5.2)"
    raise NotImplementedError(message)
