"""Themes: ``theme()``, ``element_*()``, named themes and flavors.

Ported from lets-plot's ``lets_plot/plot/theme_.py`` and ``theme_set.py`` (MIT, see
THIRD_PARTY_NOTICES.md). The engine draws the options listed in plan.md (Phase 1.4) and
warns about the rest.
"""

from ggrust.plot.core import FeatureSpec

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
    "theme_gray",
    "theme_grey",
    "theme_light",
    "theme_minimal",
    "theme_minimal2",
    "theme_none",
    "theme_void",
]


def theme(
    *,
    exponent_format=None,
    line=None,
    rect=None,
    text=None,
    title=None,
    axis=None,
    axis_ontop=None,
    axis_ontop_x=None,
    axis_ontop_y=None,
    axis_title=None,
    axis_title_x=None,
    axis_title_y=None,
    axis_text=None,
    axis_text_x=None,
    axis_text_y=None,
    axis_text_spacing=None,
    axis_text_spacing_x=None,
    axis_text_spacing_y=None,
    axis_ticks=None,
    axis_ticks_x=None,
    axis_ticks_y=None,
    axis_minor_ticks=None,
    axis_minor_ticks_x=None,
    axis_minor_ticks_y=None,
    axis_ticks_length=None,
    axis_ticks_length_x=None,
    axis_ticks_length_y=None,
    axis_minor_ticks_length=None,
    axis_minor_ticks_length_x=None,
    axis_minor_ticks_length_y=None,
    axis_line=None,
    axis_line_x=None,
    axis_line_y=None,
    legend_background=None,
    legend_text=None,
    legend_title=None,
    legend_position=None,
    legend_justification=None,
    legend_direction=None,
    legend_margin=None,
    legend_spacing=None,
    legend_spacing_x=None,
    legend_spacing_y=None,
    legend_key=None,
    legend_key_size=None,
    legend_key_width=None,
    legend_key_height=None,
    legend_key_spacing=None,
    legend_key_spacing_x=None,
    legend_key_spacing_y=None,
    legend_box=None,
    legend_box_just=None,
    legend_box_spacing=None,
    legend_ticks=None,
    legend_ticks_length=None,
    panel_background=None,
    panel_border=None,
    panel_border_ontop=None,
    panel_grid=None,
    panel_grid_ontop=None,
    panel_grid_ontop_x=None,
    panel_grid_ontop_y=None,
    panel_grid_major=None,
    panel_grid_minor=None,
    panel_grid_major_x=None,
    panel_grid_minor_x=None,
    panel_grid_major_y=None,
    panel_grid_minor_y=None,
    panel_inset=None,
    plot_background=None,
    plot_title=None,
    plot_subtitle=None,
    plot_caption=None,
    plot_tag=None,
    plot_message=None,
    plot_margin=None,
    plot_inset=None,
    plot_title_position=None,
    plot_caption_position=None,
    plot_tag_position=None,
    plot_tag_location=None,
    plot_tag_prefix=None,
    plot_tag_suffix=None,
    strip_background=None,
    strip_background_x=None,
    strip_background_y=None,
    strip_text=None,
    strip_text_x=None,
    strip_text_y=None,
    strip_spacing=None,
    strip_spacing_x=None,
    strip_spacing_y=None,
    panel_spacing=None,
    panel_spacing_x=None,
    panel_spacing_y=None,
    axis_tooltip=None,
    axis_tooltip_x=None,
    axis_tooltip_y=None,
    axis_tooltip_text=None,
    axis_tooltip_text_x=None,
    axis_tooltip_text_y=None,
    tooltip=None,
    tooltip_text=None,
    tooltip_title_text=None,
    tooltip_merge=None,
    tooltip_max_count=None,
    label_text=None,
    geom=None,
) -> FeatureSpec:
    """Set theme options: element_*() values, numbers or names per theme element.

    Parameters
    ----------
    **options
        As in lets_plot ``theme()`` (``axis_text_x=element_text(size=10)``,
        ``legend_position="bottom"``, ``panel_grid=element_blank()``, ...).

    Returns
    -------
    FeatureSpec
        The theme feature.
    """
    return FeatureSpec("theme", name=None, **_filter_none(locals()))


def _filter_none(original: dict) -> dict:
    def filter_value(value):
        return _filter_none(value) if isinstance(value, dict) else value

    return {key: filter_value(value) for key, value in original.items() if value is not None}


def element_blank() -> dict:
    """Hide a theme element.

    Returns
    -------
    dict
        The element.
    """
    return dict(blank=True)


def element_rect(fill=None, color=None, size=None, linetype=None, blank=False) -> dict:
    """Style a rectangle (backgrounds, borders).

    Returns
    -------
    dict
        The element.
    """
    return locals()


def element_line(color=None, size=None, linetype=None, blank=False) -> dict:
    """Style a line (axis lines, ticks, grid).

    Returns
    -------
    dict
        The element.
    """
    return locals()


def element_text(
    color=None,
    family=None,
    face=None,
    size=None,
    angle=None,
    hjust=None,
    vjust=None,
    margin=None,
    blank=False,
) -> dict:
    """Style text.

    Returns
    -------
    dict
        The element.
    """
    return locals()


def element_markdown(
    color=None,
    family=None,
    face=None,
    size=None,
    angle=None,
    hjust=None,
    vjust=None,
    margin=None,
    blank=False,
) -> dict:
    """Style text written in Markdown (drawn as plain text by ggrust).

    Returns
    -------
    dict
        The element.
    """
    return {"markdown": True, **locals()}


def margin(t=None, r=None, b=None, l=None):  # noqa: E741 (lets_plot's name)
    """Return margins ``[top, right, bottom, left]`` (deprecated in lets_plot).

    Returns
    -------
    list
        The margins.
    """
    return [t, r, b, l]


def element_geom(pen=None, brush=None, paper=None) -> dict:
    """Set default geom colors.

    Returns
    -------
    dict
        The element.
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
    """
    return _named("grey")


def theme_gray() -> FeatureSpec:
    """Use the grey background theme (alias of ``theme_grey``).

    Returns
    -------
    FeatureSpec
        The theme.
    """
    return _named("gray")


def theme_light() -> FeatureSpec:
    """Use the light theme.

    Returns
    -------
    FeatureSpec
        The theme.
    """
    return _named("light")


def theme_classic() -> FeatureSpec:
    """Use the classic theme: axis lines, no grid.

    Returns
    -------
    FeatureSpec
        The theme.
    """
    return _named("classic")


def theme_minimal() -> FeatureSpec:
    """Use the minimal theme.

    Returns
    -------
    FeatureSpec
        The theme.
    """
    return _named("minimal")


def theme_minimal2() -> FeatureSpec:
    """Use lets-plot's default theme.

    Returns
    -------
    FeatureSpec
        The theme.
    """
    return _named("minimal2")


def theme_none() -> FeatureSpec:
    """Use the unstyled theme.

    Returns
    -------
    FeatureSpec
        The theme.
    """
    return _named("none")


def theme_bw() -> FeatureSpec:
    """Use the black-and-white theme.

    Returns
    -------
    FeatureSpec
        The theme.
    """
    return _named("bw")


def theme_void() -> FeatureSpec:
    """Use the classic theme without lines and axes.

    Returns
    -------
    FeatureSpec
        The theme.
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
    """
    return _flavor("darcula")


def flavor_solarized_light() -> FeatureSpec:
    """Use the Solarized light color flavor.

    Returns
    -------
    FeatureSpec
        The flavor.
    """
    return _flavor("solarized_light")


def flavor_solarized_dark() -> FeatureSpec:
    """Use the Solarized dark color flavor.

    Returns
    -------
    FeatureSpec
        The flavor.
    """
    return _flavor("solarized_dark")


def flavor_high_contrast_light() -> FeatureSpec:
    """Use the high-contrast light color flavor.

    Returns
    -------
    FeatureSpec
        The flavor.
    """
    return _flavor("high_contrast_light")


def flavor_high_contrast_dark() -> FeatureSpec:
    """Use the high-contrast dark color flavor.

    Returns
    -------
    FeatureSpec
        The flavor.
    """
    return _flavor("high_contrast_dark")


def flavor_standard() -> FeatureSpec:
    """Use the standard color flavor of the theme.

    Returns
    -------
    FeatureSpec
        The flavor.
    """
    return _flavor("standard")
