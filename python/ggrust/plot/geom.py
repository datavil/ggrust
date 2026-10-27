"""Geom layer functions.

Ported from lets-plot's ``lets_plot/plot/geom.py`` (MIT, see THIRD_PARTY_NOTICES.md).
Only geoms the engine draws are exported; more are added phase by phase (plan.md).
"""

from ggrust.plot.core import FeatureSpec, LayerSpec
from ggrust.plot.tooltip import layer_tooltips
from ggrust.plot.util import as_annotated_data, key_int2str

__all__ = [
    "geom_blank",
    "geom_hline",
    "geom_label",
    "geom_line",
    "geom_path",
    "geom_point",
    "geom_polygon",
    "geom_segment",
    "geom_text",
    "geom_vline",
]


def geom_point(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    manual_key=None,
    sampling=None,
    tooltips=None,
    size_unit=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Scatter points.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``x``, ``y`` (required), ``color``, ``size``,
        ``alpha``, ... Mapping data to styles arrives in Phase 1.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    stat : str, optional
        Stat name (``"identity"``).
    position : str or FeatureSpec, optional
        Position adjustment (``"identity"``).
    show_legend, inherit_aes, manual_key, sampling, tooltips, size_unit, na_rm : optional
        As in lets_plot. ``inherit_aes`` and ``na_rm`` are used; the others are
        reported as not supported yet.
    **other_args
        Constant aesthetics such as ``color="red"``, ``size=2``, ``alpha=0.5``, or
        constant positions ``x=1, y=2``.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "point",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        size_unit=size_unit,
        na_rm=na_rm,
        **other_args,
    )


def geom_path(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    manual_key=None,
    sampling=None,
    tooltips=None,
    color_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Connect observations in data order.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "path",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_line(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    manual_key=None,
    sampling=None,
    tooltips=None,
    color_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Connect observations sorted by x.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "line",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_polygon(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    manual_key=None,
    sampling=None,
    tooltips=None,
    color_by=None,
    fill_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Draw filled polygons, one per group.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "polygon",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_hline(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    manual_key=None,
    sampling=None,
    tooltips=None,
    yintercept=None,
    color_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Horizontal reference lines at ``yintercept``.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "hline",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        yintercept=yintercept,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_vline(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    manual_key=None,
    sampling=None,
    tooltips=None,
    xintercept=None,
    color_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Vertical reference lines at ``xintercept``.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "vline",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        xintercept=xintercept,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_segment(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    manual_key=None,
    sampling=None,
    tooltips=None,
    arrow=None,
    color_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Line segments from (x, y) to (xend, yend), optionally with arrows.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "segment",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        arrow=arrow,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_text(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    manual_key=None,
    sampling=None,
    tooltips=None,
    label_format=None,
    na_text=None,
    nudge_x=None,
    nudge_y=None,
    check_overlap=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Text labels at (x, y); the text comes from the ``label`` aesthetic.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "text",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        label_format=label_format,
        na_text=na_text,
        nudge_x=nudge_x,
        nudge_y=nudge_y,
        check_overlap=check_overlap,
        na_rm=na_rm,
        **other_args,
    )


def geom_label(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    manual_key=None,
    sampling=None,
    tooltips=None,
    label_format=None,
    na_text=None,
    nudge_x=None,
    nudge_y=None,
    label_padding=None,
    label_r=None,
    label_size=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Text labels in a rounded box at (x, y).

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "label",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        label_format=label_format,
        na_text=na_text,
        nudge_x=nudge_x,
        nudge_y=nudge_y,
        label_padding=label_padding,
        label_r=label_r,
        label_size=label_size,
        na_rm=na_rm,
        **other_args,
    )


def geom_blank(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=False,
    manual_key=None,
    sampling=None,
    tooltips=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Draw nothing; the mapped data still trains the scales.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics such as ``color="red"`` or ``size=2``, and the geom's
        options. As in lets_plot; options ggrust does not read yet are reported as
        not supported.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "blank",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        na_rm=na_rm,
        **other_args,
    )


def _geom(
    name,
    *,
    mapping=None,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    na_rm=None,
    manual_key=None,
    sampling=None,
    tooltips=None,
    **kwargs,
) -> LayerSpec:
    if mapping and not (isinstance(mapping, FeatureSpec) and mapping.kind == "mapping"):
        message = "Unexpected value for argument 'mapping'. Hint: try to use function aes()"
        raise ValueError(message)
    data = key_int2str(data)
    data, mapping, data_meta = as_annotated_data(data, mapping)
    if isinstance(tooltips, list | tuple):
        tooltips = layer_tooltips(tooltips)
    return LayerSpec(
        geom=name,
        stat=stat,
        data=data,
        mapping=mapping,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        na_rm=na_rm,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        **data_meta,
        **kwargs,
    )
