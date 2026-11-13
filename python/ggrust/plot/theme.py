"""Themes: ``theme()``, ``element_*()``, named themes and flavors.

Ported from lets-plot's ``lets_plot/plot/theme_.py`` and ``theme_set.py`` (MIT, see
THIRD_PARTY_NOTICES.md). The engine draws the options documented on ``theme()`` and
warns about the rest.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, cast

from ggrust.errors import SpecError, UnsupportedValueError
from ggrust.plot.core import FeatureSpec

if TYPE_CHECKING:
    from collections.abc import Mapping, Sequence
    from typing import Any, Literal

    from ggrust._typing import Color, Margins, Number, ThemeElement

__all__ = [
    "element_blank",
    "element_geom",
    "element_line",
    "element_markdown",
    "element_rect",
    "element_text",
    "flavor_darcula",
    "flavor_high_contrast_dark",
    "flavor_high_contrast_light",
    "flavor_solarized_dark",
    "flavor_solarized_light",
    "flavor_standard",
    "margin",
    "theme",
    "theme_bw",
    "theme_classic",
    "theme_dark",
    "theme_gray",
    "theme_grey",
    "theme_light",
    "theme_linedraw",
    "theme_minimal",
    "theme_minimal2",
    "theme_none",
    "theme_void",
]


def theme(
    *,
    exponent_format: str | tuple[str, int | None, int | None] | None = None,
    line: ThemeElement | None = None,
    rect: ThemeElement | None = None,
    text: ThemeElement | None = None,
    title: ThemeElement | None = None,
    axis: ThemeElement | None = None,
    axis_ontop: bool | None = None,
    axis_ontop_x: bool | None = None,
    axis_ontop_y: bool | None = None,
    axis_title: ThemeElement | None = None,
    axis_title_x: ThemeElement | None = None,
    axis_title_y: ThemeElement | None = None,
    axis_text: ThemeElement | None = None,
    axis_text_x: ThemeElement | None = None,
    axis_text_y: ThemeElement | None = None,
    axis_text_spacing: Number | None = None,
    axis_text_spacing_x: Number | None = None,
    axis_text_spacing_y: Number | None = None,
    axis_ticks: ThemeElement | None = None,
    axis_ticks_x: ThemeElement | None = None,
    axis_ticks_y: ThemeElement | None = None,
    axis_minor_ticks: ThemeElement | None = None,
    axis_minor_ticks_x: ThemeElement | None = None,
    axis_minor_ticks_y: ThemeElement | None = None,
    axis_ticks_length: Number | None = None,
    axis_ticks_length_x: Number | None = None,
    axis_ticks_length_y: Number | None = None,
    axis_minor_ticks_length: Number | None = None,
    axis_minor_ticks_length_x: Number | None = None,
    axis_minor_ticks_length_y: Number | None = None,
    axis_line: ThemeElement | None = None,
    axis_line_x: ThemeElement | None = None,
    axis_line_y: ThemeElement | None = None,
    legend_background: ThemeElement | None = None,
    legend_text: ThemeElement | None = None,
    legend_title: ThemeElement | None = None,
    legend_position: str | Sequence[Number] | None = None,
    legend_position_inside: Sequence[Number] | None = None,
    legend_justification: str | Sequence[Number] | None = None,
    legend_direction: Literal["horizontal", "vertical"] | None = None,
    legend_margin: Margins | None = None,
    legend_spacing: Number | None = None,
    legend_spacing_x: Number | None = None,
    legend_spacing_y: Number | None = None,
    legend_key: ThemeElement | None = None,
    legend_key_size: Number | None = None,
    legend_key_width: Number | None = None,
    legend_key_height: Number | None = None,
    legend_key_spacing: Number | None = None,
    legend_key_spacing_x: Number | None = None,
    legend_key_spacing_y: Number | None = None,
    legend_box: Literal["horizontal", "vertical"] | None = None,
    legend_box_just: str | None = None,
    legend_box_spacing: Number | None = None,
    legend_ticks: ThemeElement | None = None,
    legend_ticks_length: Number | None = None,
    legend_interactive: bool | None = None,
    panel_background: ThemeElement | None = None,
    panel_border: ThemeElement | None = None,
    panel_border_ontop: bool | None = None,
    panel_grid: ThemeElement | None = None,
    panel_grid_ontop: bool | None = None,
    panel_grid_ontop_x: bool | None = None,
    panel_grid_ontop_y: bool | None = None,
    panel_grid_major: ThemeElement | None = None,
    panel_grid_minor: ThemeElement | None = None,
    panel_grid_major_x: ThemeElement | None = None,
    panel_grid_minor_x: ThemeElement | None = None,
    panel_grid_major_y: ThemeElement | None = None,
    panel_grid_minor_y: ThemeElement | None = None,
    panel_inset: Margins | None = None,
    plot_background: ThemeElement | None = None,
    plot_title: ThemeElement | None = None,
    plot_subtitle: ThemeElement | None = None,
    plot_caption: ThemeElement | None = None,
    plot_tag: ThemeElement | None = None,
    plot_message: ThemeElement | None = None,
    plot_margin: Margins | None = None,
    plot_inset: Margins | None = None,
    plot_title_position: Literal["panel", "plot"] | None = None,
    plot_caption_position: Literal["panel", "plot"] | None = None,
    plot_tag_position: str | Sequence[Number] | None = None,
    plot_tag_location: str | None = None,
    plot_tag_prefix: str | None = None,
    plot_tag_suffix: str | None = None,
    strip_background: ThemeElement | None = None,
    strip_background_x: ThemeElement | None = None,
    strip_background_y: ThemeElement | None = None,
    strip_text: ThemeElement | None = None,
    strip_text_x: ThemeElement | None = None,
    strip_text_y: ThemeElement | None = None,
    strip_spacing: Number | None = None,
    strip_spacing_x: Number | None = None,
    strip_spacing_y: Number | None = None,
    panel_spacing: Number | None = None,
    panel_spacing_x: Number | None = None,
    panel_spacing_y: Number | None = None,
    axis_tooltip: ThemeElement | None = None,
    axis_tooltip_x: ThemeElement | None = None,
    axis_tooltip_y: ThemeElement | None = None,
    axis_tooltip_text: ThemeElement | None = None,
    axis_tooltip_text_x: ThemeElement | None = None,
    axis_tooltip_text_y: ThemeElement | None = None,
    tooltip: ThemeElement | None = None,
    tooltip_text: ThemeElement | None = None,
    tooltip_title_text: ThemeElement | None = None,
    tooltip_merge: bool | None = None,
    tooltip_max_count: int | None = None,
    label_text: ThemeElement | None = None,
    geom: Mapping[str, Any] | None = None,
    ggside_panel_scale: Number | None = None,
    ggside_panel_scale_x: Number | None = None,
    ggside_panel_scale_y: Number | None = None,
    ggside_panel_spacing: Number | None = None,
    ggside_panel_spacing_x: Number | None = None,
    ggside_panel_spacing_y: Number | None = None,
    ggside_panel_background: ThemeElement | None = None,
    ggside_xside_panel_background: ThemeElement | None = None,
    ggside_yside_panel_background: ThemeElement | None = None,
    ggside_panel_border: ThemeElement | None = None,
    ggside_xside_panel_border: ThemeElement | None = None,
    ggside_yside_panel_border: ThemeElement | None = None,
    ggside_axis_text: ThemeElement | None = None,
    ggside_axis_text_x: ThemeElement | None = None,
    ggside_axis_text_x_top: ThemeElement | None = None,
    ggside_axis_text_x_bottom: ThemeElement | None = None,
    ggside_axis_text_y: ThemeElement | None = None,
    ggside_axis_text_y_left: ThemeElement | None = None,
    ggside_axis_text_y_right: ThemeElement | None = None,
    ggside_axis_ticks: ThemeElement | None = None,
    ggside_axis_ticks_x: ThemeElement | None = None,
    ggside_axis_ticks_x_top: ThemeElement | None = None,
    ggside_axis_ticks_x_bottom: ThemeElement | None = None,
    ggside_axis_ticks_y: ThemeElement | None = None,
    ggside_axis_ticks_y_left: ThemeElement | None = None,
    ggside_axis_ticks_y_right: ThemeElement | None = None,
    ggside_axis_ticks_length: Number | None = None,
    ggside_axis_ticks_length_x: Number | None = None,
    ggside_axis_ticks_length_x_top: Number | None = None,
    ggside_axis_ticks_length_x_bottom: Number | None = None,
    ggside_axis_ticks_length_y: Number | None = None,
    ggside_axis_ticks_length_y_left: Number | None = None,
    ggside_axis_ticks_length_y_right: Number | None = None,
    ggside_axis_minor_ticks: ThemeElement | None = None,
    ggside_axis_minor_ticks_x: ThemeElement | None = None,
    ggside_axis_minor_ticks_x_top: ThemeElement | None = None,
    ggside_axis_minor_ticks_x_bottom: ThemeElement | None = None,
    ggside_axis_minor_ticks_y: ThemeElement | None = None,
    ggside_axis_minor_ticks_y_left: ThemeElement | None = None,
    ggside_axis_minor_ticks_y_right: ThemeElement | None = None,
    ggside_axis_minor_ticks_length: Number | None = None,
    ggside_axis_minor_ticks_length_x: Number | None = None,
    ggside_axis_minor_ticks_length_x_top: Number | None = None,
    ggside_axis_minor_ticks_length_x_bottom: Number | None = None,
    ggside_axis_minor_ticks_length_y: Number | None = None,
    ggside_axis_minor_ticks_length_y_left: Number | None = None,
    ggside_axis_minor_ticks_length_y_right: Number | None = None,
    ggside_axis_line: ThemeElement | None = None,
    ggside_axis_line_x: ThemeElement | None = None,
    ggside_axis_line_x_top: ThemeElement | None = None,
    ggside_axis_line_x_bottom: ThemeElement | None = None,
    ggside_axis_line_y: ThemeElement | None = None,
    ggside_axis_line_y_left: ThemeElement | None = None,
    ggside_axis_line_y_right: ThemeElement | None = None,
    legendry_bracket: ThemeElement | None = None,
    legendry_bracket_size: Number | None = None,
    legendry_box: ThemeElement | None = None,
    legendry_guide_spacing: Number | None = None,
) -> FeatureSpec:
    """Set theme options: element_*() values, numbers or names per theme element.

    Every parameter is keyword-only, named and valued as in lets_plot ``theme()``
    (``axis_text_x=element_text(size=10)``, ``legend_position="bottom"``,
    ``panel_grid=element_blank()``, ...). Parameters left as None are not set.
    Element parameters take an ``element_*()`` value, or ``"blank"`` to hide the
    element. ``_x`` / ``_y`` variants override the shared one for one axis.

    Parameters
    ----------
    exponent_format : str or tuple, optional
        How every number label (axes, legends, text, tooltips) writes exponents:
        ``"e"`` (``1e+6``), ``"pow"`` (``10^6`` as a superscript, the default) or
        ``"pow_full"`` (``1·10^6``). A tuple ``(style, lower, upper)`` also sets the
        exponents at or beyond which numbers switch to scientific notation (``None``
        keeps the default bound, -7 or 6).
    line, rect, text, title : dict or str, optional
        Root elements every line, rectangle, text and title inherits from.
    axis : dict or str, optional
        Every part of both axes.
    axis_ontop, axis_ontop_x, axis_ontop_y : bool, optional
        True draws the axis over the plot layers.
    axis_title, axis_title_x, axis_title_y : dict or str, optional
        Axis titles (``element_text()``).
    axis_text, axis_text_x, axis_text_y : dict or str, optional
        Tick labels (``element_text()``).
    axis_text_spacing, axis_text_spacing_x, axis_text_spacing_y : float, optional
        Space between the ticks and their labels, in pixels.
    axis_ticks, axis_ticks_x, axis_ticks_y : dict or str, optional
        Major ticks (``element_line()``).
    axis_minor_ticks, axis_minor_ticks_x, axis_minor_ticks_y : dict or str, optional
        Minor ticks (``element_line()``).
    axis_ticks_length, axis_ticks_length_x, axis_ticks_length_y : float, optional
        Length of the major ticks, in pixels.
    axis_minor_ticks_length, axis_minor_ticks_length_x, axis_minor_ticks_length_y : float, optional
        Length of the minor ticks, in pixels.
    axis_line, axis_line_x, axis_line_y : dict or str, optional
        Axis lines (``element_line()``); they also take ``arrow`` and ``extent``
        for arrow-headed, truncated axes.
    legend_background, legend_text, legend_title : dict or str, optional
        Legend box (``element_rect()``), key labels and title (``element_text()``).
    legend_position : str or list of float, optional
        ``"right"``, ``"left"``, ``"top"``, ``"bottom"``, ``"none"``, or ``[x, y]``
        inside the plot (0 to 1). ``"inside"`` puts legends in the panel at
        ``legend_position_inside`` (ggplot2 3.5).
    legend_position_inside : list of float, optional
        ``[x, y]`` (0 to 1, from the panel's bottom-left) where inside legends go:
        ``legend_position="inside"`` and ``guide_*(position="inside")``. None uses
        ``legend_justification`` (ggplot2 3.5).
    legend_justification : str or list of float, optional
        Anchor of the legend box: a name such as ``"center"``, or ``[x, y]``.
    legend_direction : {"horizontal", "vertical"}, optional
        Layout of the legend keys.
    legend_margin : float or list of float, optional
        Margins around the legend, in pixels (one number, or 2 to 4 as in CSS).
    legend_spacing, legend_spacing_x, legend_spacing_y : float, optional
        Space between legends, in pixels.
    legend_key : dict or str, optional
        Background of the legend keys (``element_rect()``).
    legend_key_size, legend_key_width, legend_key_height : float, optional
        Size of the legend keys, in pixels.
    legend_key_spacing, legend_key_spacing_x, legend_key_spacing_y : float, optional
        Space between legend keys, in pixels.
    legend_box : {"horizontal", "vertical"}, optional
        Arrangement of several legends.
    ggside_panel_scale, ggside_panel_scale_x, ggside_panel_scale_y : float, optional
        Size of the ggside side panels (``geom_xside*()``, ``geom_yside*()``) as a
        share of the main panel: ``_x`` the x side panel's height, ``_y`` the y
        side panel's width. Default 0.1, as ggside.
    ggside_panel_spacing, ggside_panel_spacing_x, ggside_panel_spacing_y : float, optional
        Gap between the main panel and a side panel, in pixels: ``_x`` beside a y
        side panel, ``_y`` beside an x side panel. Default 2 pt (about 2.7 px).
    ggside_panel_background, ggside_*side_panel_background : dict or str, optional
        Background of the ggside side panels (``element_rect()``), as R ggside's
        ``ggside.panel.background``: ``ggside_xside_panel_background`` styles the x
        side panels (above or below), ``ggside_yside_panel_background`` the y side
        panels (left or right). Unset ones fall back
        to ``ggside_panel_background``, then to ``panel_background``, so by default
        a side panel looks like the main one. The background's border color and
        size frame the side panel the way they frame the main panel:
        ``ggside_panel_background=element_rect(size=0)`` keeps the fill and drops
        the frame, ``element_blank()`` drops both. ``ggmarginal()`` margins draw
        neither.
    ggside_panel_border, ggside_*side_panel_border : dict or str, optional
        Border of the ggside side panels, drawn over their layers, as R ggside's
        ``ggside.panel.border``, with the same suffix and fallback rules (last
        ``panel_border``). ``ggside_panel_border=element_blank()`` keeps the main
        panel's border and drops the side panels' ones. Under ``coord_flip()`` the
        ``_xside_`` and ``_yside_`` elements follow their aesthetic, as the
        ``ggside_axis_*`` ones do.
    ggside_axis_text, ggside_axis_text_* : dict or str, optional
        Tick labels of a side panel's own axis, as R ggside's ``ggside.axis.text*``.
        Suffixes: ``_x``, ``_x_top``, ``_x_bottom``, ``_y``, ``_y_left``,
        ``_y_right`` (the same for every ``ggside_axis_*`` element). ``_y``
        styles only the x side panel's (y) axis and ``_x`` only the y side
        panel's (x) axis; ``_top``, ``_bottom``, ``_left``, ``_right`` pick the
        axis drawn on that side. The most specific one set wins, and anything
        unset comes from the main ``axis_text*`` elements, so the main axes never
        change. ``ggside_axis_text_y=element_blank()`` hides the x side panel's
        track names and frees their room. Under ``coord_flip()`` the elements
        follow their aesthetic, as the main ``axis_*`` ones do.
    ggside_axis_ticks, ggside_axis_ticks_* : dict or str, optional
        Ticks of a side panel's own axis (``element_line()``), with the same
        suffix rules; unset ones come from ``axis_ticks*``.
    ggside_axis_ticks_length, ggside_axis_ticks_length_* : float, optional
        Tick length of a side panel's own axis, in pixels; unset ones come from
        ``axis_ticks_length*``.
    ggside_axis_minor_ticks, ggside_axis_minor_ticks_* : dict or str, optional
        Minor ticks of a side panel's own axis; unset ones come from
        ``axis_minor_ticks*``.
    ggside_axis_minor_ticks_length, ggside_axis_minor_ticks_length_* : float, optional
        Minor tick length of a side panel's own axis, in pixels; unset ones come
        from ``axis_minor_ticks_length*``.
    ggside_axis_line, ggside_axis_line_* : dict or str, optional
        Axis line of a side panel's own axis (``element_line()``, with ``arrow``
        and ``extent`` as on ``axis_line``); unset ones come from ``axis_line*``.
    legendry_bracket : dict or str, optional
        Bracket line of ``guide_axis_nested()`` (``element_line()``), as R
        legendry's ``legendry.bracket``; unset, it comes from ``line``.
    legendry_bracket_size : float, optional
        Room a bracket takes across its axis, in pixels (legendry's 2 mm: 7.56).
    legendry_box : dict or str, optional
        Box of ``guide_axis_nested(type="box")`` (``element_rect()``), as
        legendry's ``legendry.box``; unset, it comes from ``rect`` (legendry
        inherits ``strip.background``, blank in the lets-plot themes).
    legendry_guide_spacing : float, optional
        Gap between an axis's labels and its nested guide, in pixels (0, as in
        legendry's ``guide_axis_nested()``).
    legend_box_just : str, optional
        Justification of the legends within their box.
    legend_box_spacing : float, optional
        Space between the plot panel and the legend box, in pixels.
    legend_ticks : dict or str, optional
        Ticks of the color bar (``element_line()``).
    legend_ticks_length : float, optional
        Length of the color bar ticks, in pixels.
    legend_interactive : bool, optional
        ggrust extension, HTML output only. Clickable discrete legend keys:
        click hides or shows a key's marks, double-click isolates the key.
        ``False`` turns it off and leaves the key data out of the HTML.
        On by default.
    panel_background, panel_border : dict or str, optional
        Panel background and border (``element_rect()``).
    panel_border_ontop : bool, optional
        True draws the border over the plot layers.
    panel_grid, panel_grid_major, panel_grid_minor : dict or str, optional
        Grid lines (``element_line()``).
    panel_grid_major_x, panel_grid_minor_x : dict or str, optional
        Grid lines of the x axis.
    panel_grid_major_y, panel_grid_minor_y : dict or str, optional
        Grid lines of the y axis.
    panel_grid_ontop, panel_grid_ontop_x, panel_grid_ontop_y : bool, optional
        True draws the grid over the plot layers.
    panel_inset : float or list of float, optional
        Inner margins of the panel, in pixels.
    plot_background : dict or str, optional
        Background of the whole plot (``element_rect()``).
    plot_title, plot_subtitle, plot_caption, plot_tag : dict or str, optional
        Title, subtitle, caption and tag texts (``element_text()``).
    plot_message : dict or str, optional
        Messages shown on the plot (lets_plot); ``"blank"`` hides them.
    plot_margin, plot_inset : float or list of float, optional
        Outer and inner margins of the plot, in pixels.
    plot_title_position, plot_caption_position : {"panel", "plot"}, optional
        Align the title or caption with the panel or with the whole plot.
    plot_tag_position : str or list of float, optional
        Where the tag goes: a side name such as ``"topleft"``, or ``[x, y]``.
    plot_tag_location : str, optional
        What the tag position is relative to: ``"panel"``, ``"plot"`` or ``"margin"``.
    plot_tag_prefix, plot_tag_suffix : str, optional
        Text added before and after the tag.
    strip_background, strip_background_x, strip_background_y : dict or str, optional
        Facet strip backgrounds (``element_rect()``).
    strip_text, strip_text_x, strip_text_y : dict or str, optional
        Facet strip texts (``element_text()``).
    strip_spacing, strip_spacing_x, strip_spacing_y : float, optional
        Space between the strips and the panels, in pixels.
    panel_spacing, panel_spacing_x, panel_spacing_y : float, optional
        Space between facet panels, in pixels.
    axis_tooltip, axis_tooltip_x, axis_tooltip_y : dict or str, optional
        Axis tooltips of HTML output (``element_rect()``).
    axis_tooltip_text, axis_tooltip_text_x, axis_tooltip_text_y : dict or str, optional
        Text of the axis tooltips (``element_text()``).
    tooltip, tooltip_text, tooltip_title_text : dict or str, optional
        Tooltip box, text and title text of HTML output.
    tooltip_merge : bool, optional
        lets_plot's tooltip merging option.
    tooltip_max_count : int, optional
        Maximum number of tooltips shown at once.
    label_text : dict or str, optional
        Text of ``layer_labels()`` annotations (``element_text()``). ggrust does not
        draw annotations yet, so it is reported as unsupported.
    geom : dict, optional
        Default geom colors, from ``element_geom()``.

    Returns
    -------
    FeatureSpec
        The theme feature.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme(
    ...     legend_position="bottom",
    ...     axis_title=element_text(size=15, face="bold"),
    ...     panel_grid_minor=element_blank(),
    ... )
    """
    return FeatureSpec("theme", name=None, **_filter_none(locals()))


def _filter_none(original: dict[str, Any]) -> dict[str, Any]:
    def filter_value(value: object) -> object:
        return _filter_none(cast("dict[str, Any]", value)) if isinstance(value, dict) else value

    return {key: filter_value(value) for key, value in original.items() if value is not None}


def element_blank() -> dict[str, Any]:
    """Hide a theme element.

    Returns
    -------
    dict
        The element.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme(panel_grid=element_blank(), axis_ticks=element_blank())
    """
    return dict(blank=True)


def element_rect(
    fill: Color | None = None,
    color: Color | None = None,
    size: Number | None = None,
    linetype: int | str | None = None,
    blank: bool = False,
) -> dict[str, Any]:
    """Style a rectangle (backgrounds, borders).

    Parameters
    ----------
    fill : str, optional
        Fill color.
    color : str, optional
        Color.
    size : float, optional
        Size (line width, or text size in points).
    linetype : int or str, optional
        Line type, by number or name.
    blank : bool, default=False
        True hides the element.

    Returns
    -------
    dict
        The element.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme(panel_background=element_rect(fill="ivory", color="tan", size=2))
    """
    return locals()


def element_line(
    color: Color | None = None,
    size: Number | None = None,
    linetype: int | str | None = None,
    blank: bool = False,
    arrow: FeatureSpec | None = None,
    extent: Number | None = None,
) -> dict[str, Any]:
    """Style a line (axis lines, ticks, grid).

    Parameters
    ----------
    color : str, optional
        Color.
    size : float, optional
        Size (line width, or text size in points).
    linetype : int or str, optional
        Line type, by number or name.
    blank : bool, default=False
        True hides the element.
    arrow : FeatureSpec, optional
        Arrow heads from ``arrow()``, sized in pixels. Axis lines only
        (``axis_line``, ``axis_line_x``, ``axis_line_y``), as ggplot2's
        ``element_line(arrow=)``; lets-plot has no such option. With the default
        ``ends="last"`` the x line points right and the y line points up. The
        default theme blanks ``axis_line_y``, which wins over ``axis_line``: set
        ``axis_line_x`` and ``axis_line_y`` there, or use ``theme_classic()``.
    extent : float, optional
        Fraction of the panel an axis line covers from the bottom-left corner,
        ``0 < extent <= 1`` (default 1, the whole axis). Axis lines only; ggrust's
        take on ggh4x ``guide_axis_truncated``.

    Returns
    -------
    dict
        The element.

    Raises
    ------
    UnsupportedValueError
        If ``arrow`` is not an ``arrow()`` spec.
    SpecError
        If ``extent`` is not a number in ``(0, 1]``.

    Examples
    --------
    Corner arrows with titles next to them, as on UMAP plots:

    >>> from ggrust import *
    >>> corner = theme(
    ...     axis_line=element_line(arrow=arrow(length=8, type="closed"), extent=0.25),
    ...     axis_text=element_blank(),
    ...     axis_ticks=element_blank(),
    ...     axis_title_x=element_text(hjust=0),
    ...     axis_title_y=element_text(hjust=0),
    ... )
    """
    if arrow is not None and not (isinstance(arrow, FeatureSpec) and arrow.kind == "arrow"):
        path = "element_line.arrow"
        reason = f"expected arrow(...), got {type(arrow).__name__}; e.g. arrow(length=8)"
        raise UnsupportedValueError(f"at '{path}': {reason}", path=path, reason=reason)
    if extent is not None and (
        isinstance(extent, bool) or not isinstance(extent, (int, float)) or not 0 < extent <= 1
    ):
        path = "element_line.extent"
        reason = (
            f"expected a fraction of the panel, 0 < extent <= 1, got {extent!r}; "
            "e.g. extent=0.25 draws the first quarter of the axis"
        )
        raise SpecError(f"at '{path}': {reason}", path=path, reason=reason)
    return dict(color=color, size=size, linetype=linetype, blank=blank, arrow=arrow, extent=extent)


def element_text(
    color: Color | None = None,
    family: str | None = None,
    face: str | None = None,
    size: Number | None = None,
    angle: Number | None = None,
    hjust: Number | None = None,
    vjust: Number | None = None,
    margin: Margins | None = None,
    blank: bool = False,
) -> dict[str, Any]:
    """Style text.

    Parameters
    ----------
    color : str, optional
        Color.
    family : str, optional
        Font family.
    face : str, optional
        Font face, e.g. ``"bold"`` or ``"italic"``.
    size : float, optional
        Size (line width, or text size in points).
    angle : float, optional
        Text rotation in degrees.
    hjust : float, optional
        Horizontal justification, 0 (left) to 1 (right).
    vjust : float, optional
        Vertical justification, 0 (bottom) to 1 (top).
    margin : float or list of float, optional
        Margins in pixels: one number, or 2 to 4 numbers as in CSS.
    blank : bool, default=False
        True hides the element.

    Returns
    -------
    dict
        The element.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme(axis_text=element_text(size=14, color="darkred", angle=30))
    """
    return locals()


def element_markdown(
    color: Color | None = None,
    family: str | None = None,
    face: str | None = None,
    size: Number | None = None,
    angle: Number | None = None,
    hjust: Number | None = None,
    vjust: Number | None = None,
    margin: Margins | None = None,
    blank: bool = False,
) -> dict[str, Any]:
    """Style text written in Markdown (drawn as plain text by ggrust).

    Parameters
    ----------
    color : str, optional
        Color.
    family : str, optional
        Font family.
    face : str, optional
        Font face, e.g. ``"bold"`` or ``"italic"``.
    size : float, optional
        Size (line width, or text size in points).
    angle : float, optional
        Text rotation in degrees.
    hjust : float, optional
        Horizontal justification, 0 (left) to 1 (right).
    vjust : float, optional
        Vertical justification, 0 (bottom) to 1 (top).
    margin : float or list of float, optional
        Margins in pixels: one number, or 2 to 4 numbers as in CSS.
    blank : bool, default=False
        True hides the element.

    Returns
    -------
    dict
        The element.

    Examples
    --------
    >>> from ggrust import *
    >>> title = ggtitle("Fuel use: **heavier** cars go *less* far")
    >>> style = theme(plot_title=element_markdown())  # not drawn yet: warns
    """
    return {"markdown": True, **locals()}


def margin(
    t: Number | None = None,
    r: Number | None = None,
    b: Number | None = None,
    l: Number | None = None,  # noqa: E741 (lets_plot's name)
) -> list[Number | None]:
    """Return margins ``[top, right, bottom, left]`` (deprecated in lets_plot).

    Parameters
    ----------
    t : float, optional
        Top margin.
    r : float, optional
        Right margin.
    b : float, optional
        Bottom margin.
    l : float, optional
        Left margin.

    Returns
    -------
    list
        The margins.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme(
    ...     plot_margin=margin(t=30, r=60, b=30, l=60), plot_background=element_rect(fill="linen")
    ... )
    """
    return [t, r, b, l]


def element_geom(
    pen: Color | None = None, brush: Color | None = None, paper: Color | None = None
) -> dict[str, Any]:
    """Set default geom colors.

    Parameters
    ----------
    pen : str, optional
        Color of lines and point outlines.
    brush : str, optional
        Color of fills.
    paper : str, optional
        Background color.

    Returns
    -------
    dict
        The element.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"fruit": ["apple", "banana", "apple", "cherry"], "kg": [3, 5, 2, 4]}
    >>> plot = (
    ...     ggplot(data, aes("fruit", "kg"))
    ...     + geom_bar(stat="identity")
    ...     + geom_point(size=6)
    ...     + theme(geom=element_geom(pen="navy", brush="orange", paper="ivory"))
    ... )
    """
    return locals()


def _named(name: str) -> FeatureSpec:
    return FeatureSpec("theme", name=name)


def theme_grey() -> FeatureSpec:
    """Use the grey background theme.

    Returns
    -------
    FeatureSpec
        The theme.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme_grey()
    """
    return _named("grey")


def theme_gray() -> FeatureSpec:
    """Use the grey background theme (alias of ``theme_grey``).

    Returns
    -------
    FeatureSpec
        The theme.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme_gray()
    """
    return _named("gray")


def theme_light() -> FeatureSpec:
    """Use the light theme.

    Returns
    -------
    FeatureSpec
        The theme.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme_light()
    """
    return _named("light")


def theme_dark() -> FeatureSpec:
    """Use the dark theme: a mid-grey panel with dark grid lines.

    ggplot2's ``theme_dark``. The page stays light; only the panel and strips are
    dark, so bright colors stand out. For a fully dark plot, add a dark flavor such as
    ``flavor_darcula()``.

    Returns
    -------
    FeatureSpec
        The theme.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme_dark()
    """
    return _named("dark")


def theme_linedraw() -> FeatureSpec:
    """Use the line-drawing theme: a black frame and thin black grid lines.

    ggplot2's ``theme_linedraw``: ``theme_bw`` drawn in pure ink, with black strips.

    Returns
    -------
    FeatureSpec
        The theme.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme_linedraw()
    """
    return _named("linedraw")


def theme_classic() -> FeatureSpec:
    """Use the classic theme: axis lines, no grid.

    Returns
    -------
    FeatureSpec
        The theme.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme_classic()
    """
    return _named("classic")


def theme_minimal() -> FeatureSpec:
    """Use the minimal theme.

    Returns
    -------
    FeatureSpec
        The theme.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme_minimal()
    """
    return _named("minimal")


def theme_minimal2() -> FeatureSpec:
    """Use lets-plot's default theme.

    Returns
    -------
    FeatureSpec
        The theme.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme_minimal2()
    """
    return _named("minimal2")


def theme_none() -> FeatureSpec:
    """Use the unstyled theme.

    Returns
    -------
    FeatureSpec
        The theme.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme_none()
    """
    return _named("none")


def theme_bw() -> FeatureSpec:
    """Use the black-and-white theme.

    Returns
    -------
    FeatureSpec
        The theme.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme_bw()
    """
    return _named("bw")


def theme_void() -> FeatureSpec:
    """Use the classic theme without lines and axes.

    Returns
    -------
    FeatureSpec
        The theme.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme_void()
    """
    return theme_classic() + FeatureSpec("theme", name=None, line="blank", axis="blank")


def _flavor(name: str) -> FeatureSpec:
    return FeatureSpec("theme", name=None, flavor=name)


def flavor_darcula() -> FeatureSpec:
    """Use the Darcula color flavor.

    Returns
    -------
    FeatureSpec
        The flavor.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + flavor_darcula()
    """
    return _flavor("darcula")


def flavor_solarized_light() -> FeatureSpec:
    """Use the Solarized light color flavor.

    Returns
    -------
    FeatureSpec
        The flavor.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + flavor_solarized_light()
    """
    return _flavor("solarized_light")


def flavor_solarized_dark() -> FeatureSpec:
    """Use the Solarized dark color flavor.

    Returns
    -------
    FeatureSpec
        The flavor.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + flavor_solarized_dark()
    """
    return _flavor("solarized_dark")


def flavor_high_contrast_light() -> FeatureSpec:
    """Use the high-contrast light color flavor.

    Returns
    -------
    FeatureSpec
        The flavor.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + flavor_high_contrast_light()
    """
    return _flavor("high_contrast_light")


def flavor_high_contrast_dark() -> FeatureSpec:
    """Use the high-contrast dark color flavor.

    Returns
    -------
    FeatureSpec
        The flavor.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + flavor_high_contrast_dark()
    """
    return _flavor("high_contrast_dark")


def flavor_standard() -> FeatureSpec:
    """Use the standard color flavor of the theme.

    Returns
    -------
    FeatureSpec
        The flavor.

    Examples
    --------
    >>> from ggrust import *
    >>> data = {"age": [1, 3, 5, 7, 2, 4, 6, 8, 3, 9],
    ...         "weight": [4, 6, 6, 7, 8, 12, 14, 15, 2, 3],
    ...         "pet": ["cat"] * 4 + ["dog"] * 4 + ["rabbit"] * 2}
    >>> plot = ggplot(data, aes("age", "weight", color="pet")) + geom_point(size=5)
    >>> plot = plot + theme_bw() + flavor_standard()
    """
    return _flavor("standard")
