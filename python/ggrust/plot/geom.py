"""Geom layer functions.

Ported from lets-plot's ``lets_plot/plot/geom.py`` (MIT, see THIRD_PARTY_NOTICES.md).
Geoms the engine does not draw yet still build lets_plot-identical specs, so code
written for lets_plot imports and composes; rendering them reports the geom.
"""

from ggrust.plot.core import FeatureSpec, FeatureSpecArray, LayerSpec
from ggrust.plot.tooltip import layer_tooltips
from ggrust.plot.util import as_annotated_data, key_int2str

__all__ = [
    "geom_area_ridges",
    "geom_bar",
    "geom_blank",
    "geom_boxplot",
    "geom_bracket",
    "geom_histogram",
    "geom_hline",
    "geom_jitter",
    "geom_label",
    "geom_label_repel",
    "geom_line",
    "geom_path",
    "geom_point",
    "geom_polygon",
    "geom_raster",
    "geom_rect",
    "geom_segment",
    "geom_sina",
    "geom_text",
    "geom_text_repel",
    "geom_tile",
    "geom_violin",
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
    map=None,
    map_join=None,
    use_crs=None,
    color_by=None,
    fill_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Draw nothing; the mapped data still trains the scales. No tooltips (as lets_plot).

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
        tooltips="none",
        map=map,
        map_join=map_join,
        use_crs=use_crs,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


# Geoms below build lets_plot-identical specs; the engine does not draw them yet and
# rendering reports which one (plan.md names the step that adds each).


def geom_bar(
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
    labels=None,
    orientation=None,
    color_by=None,
    fill_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Bars, by default the count of rows per x (``stat="count"``), stacked.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "bar",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        labels=labels,
        orientation=orientation,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_histogram(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    manual_key=None,
    sampling=None,
    threshold=None,
    tooltips=None,
    labels=None,
    orientation=None,
    bins=None,
    binwidth=None,
    center=None,
    boundary=None,
    breaks=None,
    color_by=None,
    fill_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Histogram of binned x values (``stat="bin"``), stacked.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "histogram",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        threshold=threshold,
        tooltips=tooltips,
        labels=labels,
        orientation=orientation,
        bins=bins,
        binwidth=binwidth,
        center=center,
        boundary=boundary,
        breaks=breaks,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_boxplot(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    manual_key=None,
    tooltips=None,
    orientation=None,
    fatten=None,
    outlier_alpha=None,
    outlier_color=None,
    outlier_fill=None,
    outlier_shape=None,
    outlier_size=None,
    outlier_stroke=None,
    varwidth=None,
    whisker_width=None,
    width_unit=None,
    color_by=None,
    fill_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec | FeatureSpecArray:
    """Box and whiskers plot. Not drawn yet.

    With the default stat, a second point layer (``stat="boxplot_outlier"``) draws
    the outliers, as in lets_plot.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    outlier_alpha, outlier_color, outlier_fill : optional
        Style of the outlier points (also ``outlier_shape``, ``outlier_size``,
        ``outlier_stroke``); ``color``, ``fill``, ``shape``, ``size`` and
        ``stroke`` in ``other_args`` are the fallback.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec or FeatureSpecArray
        The box layer, plus the outlier layer with the default stat.
    """
    boxplot_layer = _geom(
        "boxplot",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=None,
        tooltips=tooltips,
        orientation=orientation,
        fatten=fatten,
        varwidth=varwidth,
        whisker_width=whisker_width,
        width_unit=width_unit,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )
    if stat is None or stat == "boxplot":

        def outlier_parameter(name, value):
            return value if value is not None else other_args.get(name)

        outlier_fatten = 4
        size = outlier_parameter("size", outlier_size)
        if size is not None:
            size *= outlier_fatten
        boxplot_layer += _geom(
            "point",
            mapping=mapping,
            data=data,
            stat="boxplot_outlier",
            position=position,
            show_legend=False,
            inherit_aes=inherit_aes,
            sampling=None,
            orientation=orientation,
            alpha=outlier_alpha,
            color=outlier_parameter("color", outlier_color),
            fill=outlier_parameter("fill", outlier_fill),
            shape=outlier_parameter("shape", outlier_shape),
            size=size,
            stroke=outlier_parameter("stroke", outlier_stroke),
            color_by=color_by,
            fill_by=fill_by,
            na_rm=True,
        )
    return boxplot_layer


def geom_violin(
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
    orientation=None,
    show_half=None,
    quantiles=None,
    quantile_lines=None,
    scale=None,
    trim=None,
    tails_cutoff=None,
    kernel=None,
    bw=None,
    adjust=None,
    n=None,
    fs_max=None,
    color_by=None,
    fill_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Violin plot: mirrored kernel density per x (``stat="ydensity"``). Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.
        ``draw_quantiles`` is rejected, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    if "draw_quantiles" in other_args:
        message = (
            "The parameter 'draw_quantiles' is no longer supported.\n"
            "Use parameters 'quantile_lines', 'quantiles' as needed."
        )
        raise ValueError(message)
    return _geom(
        "violin",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        orientation=orientation,
        show_half=show_half,
        quantiles=quantiles,
        quantile_lines=quantile_lines,
        scale=scale,
        trim=trim,
        tails_cutoff=tails_cutoff,
        kernel=kernel,
        bw=bw,
        adjust=adjust,
        n=n,
        fs_max=fs_max,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_sina(
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
    orientation=None,
    seed=None,
    show_half=None,
    quantiles=None,
    scale=None,
    trim=None,
    tails_cutoff=None,
    kernel=None,
    bw=None,
    adjust=None,
    n=None,
    fs_max=None,
    color_by=None,
    fill_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Sina plot: points spread within the violin outline. Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "sina",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        orientation=orientation,
        seed=seed,
        show_half=show_half,
        quantiles=quantiles,
        scale=scale,
        trim=trim,
        tails_cutoff=tails_cutoff,
        kernel=kernel,
        bw=bw,
        adjust=adjust,
        n=n,
        fs_max=fs_max,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_area_ridges(
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
    trim=None,
    tails_cutoff=None,
    kernel=None,
    adjust=None,
    bw=None,
    n=None,
    fs_max=None,
    min_height=None,
    scale=None,
    quantiles=None,
    quantile_lines=None,
    color_by=None,
    fill_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Ridgeline plot: one density curve per y level. Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "area_ridges",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        trim=trim,
        tails_cutoff=tails_cutoff,
        kernel=kernel,
        adjust=adjust,
        bw=bw,
        n=n,
        fs_max=fs_max,
        min_height=min_height,
        scale=scale,
        quantiles=quantiles,
        quantile_lines=quantile_lines,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_jitter(
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
    width=None,
    height=None,
    color_by=None,
    fill_by=None,
    seed=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Points with random noise added to their positions. Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "jitter",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        width=width,
        height=height,
        color_by=color_by,
        fill_by=fill_by,
        seed=seed,
        na_rm=na_rm,
        **other_args,
    )


def geom_tile(
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
    width_unit=None,
    height_unit=None,
    color_by=None,
    fill_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Rectangles centred on (x, y), e.g. heatmap cells. Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "tile",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        width_unit=width_unit,
        height_unit=height_unit,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_raster(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    inherit_aes=None,
    manual_key=None,
    sampling=None,
    fill_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Equal-sized tiles on a regular grid. Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "raster",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_rect(
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
    map=None,
    map_join=None,
    use_crs=None,
    color_by=None,
    fill_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Rectangles from ``xmin``, ``xmax``, ``ymin``, ``ymax``. Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    map, map_join, use_crs : optional
        GIS options of lets_plot; ggrust does not support maps.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "rect",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        map=map,
        map_join=map_join,
        use_crs=use_crs,
        color_by=color_by,
        fill_by=fill_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_bracket(
    mapping=None,
    *,
    data=None,
    stat=None,
    position=None,
    show_legend=None,
    manual_key=None,
    sampling=None,
    orientation=None,
    label_format=None,
    na_text=None,
    nudge_x=None,
    nudge_y=None,
    nudge_unit=None,
    size_unit=None,
    bracket_shorten=None,
    tiplength_unit=None,
    color_by=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Labelled brackets between two x positions (significance marks). Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``: ``xmin``, ``xmax``, ``y``, ``label``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "bracket",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=None,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=None,
        orientation=orientation,
        label_format=label_format,
        na_text=na_text,
        nudge_x=nudge_x,
        nudge_y=nudge_y,
        nudge_unit=nudge_unit,
        size_unit=size_unit,
        bracket_shorten=bracket_shorten,
        tiplength_unit=tiplength_unit,
        color_by=color_by,
        na_rm=na_rm,
        **other_args,
    )


def geom_text_repel(
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
    map=None,
    map_join=None,
    use_crs=None,
    label_format=None,
    na_text=None,
    halo_width=None,
    halo_color=None,
    nudge_x=None,
    nudge_y=None,
    size_unit=None,
    nudge_unit=None,
    check_overlap=None,
    color_by=None,
    seed=None,
    max_iter=None,
    max_time=None,
    direction=None,
    point_padding=None,
    box_padding=None,
    max_overlaps=None,
    min_segment_length=None,
    arrow=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Text labels pushed apart so they do not overlap. Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "text_repel",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        map=map,
        map_join=map_join,
        use_crs=use_crs,
        label_format=label_format,
        na_text=na_text,
        halo_width=halo_width,
        halo_color=halo_color,
        nudge_x=nudge_x,
        nudge_y=nudge_y,
        size_unit=size_unit,
        nudge_unit=nudge_unit,
        check_overlap=check_overlap,
        color_by=color_by,
        max_iter=max_iter,
        max_time=max_time,
        seed=seed,
        direction=direction,
        point_padding=point_padding,
        box_padding=box_padding,
        max_overlaps=max_overlaps,
        min_segment_length=min_segment_length,
        arrow=arrow,
        na_rm=na_rm,
        **other_args,
    )


def geom_label_repel(
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
    map=None,
    map_join=None,
    use_crs=None,
    label_format=None,
    na_text=None,
    nudge_x=None,
    nudge_y=None,
    label_padding=None,
    label_r=None,
    label_size=None,
    alpha_stroke=None,
    size_unit=None,
    nudge_unit=None,
    check_overlap=None,
    color_by=None,
    fill_by=None,
    seed=None,
    max_iter=None,
    max_time=None,
    direction=None,
    point_padding=None,
    box_padding=None,
    max_overlaps=None,
    min_segment_length=None,
    arrow=None,
    na_rm=None,
    **other_args,
) -> LayerSpec:
    """Boxed text labels pushed apart so they do not overlap. Not drawn yet.

    Parameters
    ----------
    mapping : FeatureSpec, optional
        Aesthetic mapping from ``aes()``.
    data : DataFrame or dict, optional
        Layer data; inherits the plot data when None.
    **other_args
        Constant aesthetics and the geom's options, as in lets_plot.

    Returns
    -------
    LayerSpec
        The layer.
    """
    return _geom(
        "label_repel",
        mapping=mapping,
        data=data,
        stat=stat,
        position=position,
        show_legend=show_legend,
        inherit_aes=inherit_aes,
        manual_key=manual_key,
        sampling=sampling,
        tooltips=tooltips,
        map=map,
        map_join=map_join,
        use_crs=use_crs,
        label_format=label_format,
        na_text=na_text,
        nudge_x=nudge_x,
        nudge_y=nudge_y,
        label_padding=label_padding,
        label_r=label_r,
        label_size=label_size,
        alpha_stroke=alpha_stroke,
        size_unit=size_unit,
        nudge_unit=nudge_unit,
        check_overlap=check_overlap,
        color_by=color_by,
        fill_by=fill_by,
        max_iter=max_iter,
        max_time=max_time,
        seed=seed,
        direction=direction,
        point_padding=point_padding,
        box_padding=box_padding,
        max_overlaps=max_overlaps,
        min_segment_length=min_segment_length,
        arrow=arrow,
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
