"""``layer_tooltips()``: hover tooltip content of a layer.

Ported from lets-plot's ``lets_plot/plot/tooltip.py`` (MIT, see THIRD_PARTY_NOTICES.md).
The spec is complete; drawing tooltips arrives with interactivity (Phase 4).
"""

import warnings

from ggrust.plot.core import FeatureSpec, _filter_none

__all__ = ["layer_tooltips"]


def layer_tooltips(variables=None) -> "LayerTooltipsSpec":
    """Configure hover tooltips.

    Parameters
    ----------
    variables : list of str, optional
        Columns shown as ``name: value`` lines.

    Returns
    -------
    LayerTooltipsSpec
        Chain ``.line()``, ``.format()``, ``.title()``, ``.anchor()``, ... on it.
    """
    return LayerTooltipsSpec(variables=variables)


class LayerTooltipsSpec(FeatureSpec):
    """Tooltip configuration built by ``layer_tooltips()``."""

    def __init__(self, variables=None):
        self._tooltip_formats = []
        self._tooltip_lines = None
        self._tooltip_anchor = None
        self._tooltip_min_width = None
        self._tooltip_color = None
        self._tooltip_variables = variables
        self._tooltip_title = None
        self._disable_splitting = None
        super().__init__("tooltips", name=None)

    def as_dict(self) -> dict:
        """Return the tooltip options as lets_plot does."""
        result = super().as_dict()
        result["formats"] = self._tooltip_formats
        result["lines"] = self._tooltip_lines
        result["tooltip_anchor"] = self._tooltip_anchor
        result["tooltip_min_width"] = self._tooltip_min_width
        result["tooltip_color"] = self._tooltip_color
        result["variables"] = self._tooltip_variables
        result["title"] = self._tooltip_title
        result["disable_splitting"] = self._disable_splitting
        return _filter_none(result)

    def format(self, field=None, format=None) -> "LayerTooltipsSpec":
        """Set the format of a field, e.g. ``format("@x", ".2f")``."""
        self._tooltip_formats.append({"field": field, "format": format})
        return self

    def line(self, value) -> "LayerTooltipsSpec":
        """Add a line template, e.g. ``"@cluster"`` or ``"count|@n"``."""
        if self._tooltip_lines is None:
            self._tooltip_lines = []
        self._tooltip_lines.append(value)
        return self

    def anchor(self, value) -> "LayerTooltipsSpec":
        """Fix the tooltip position, e.g. ``"top_right"``."""
        self._tooltip_anchor = value
        return self

    def min_width(self, value) -> "LayerTooltipsSpec":
        """Set the minimum tooltip width in pixels."""
        self._tooltip_min_width = value
        return self

    def color(self, value) -> "LayerTooltipsSpec":
        """Set the tooltip color (deprecated in lets_plot, not supported)."""
        warnings.warn("color() is deprecated and is no longer supported", stacklevel=2)
        self._tooltip_color = value
        return self

    def title(self, value) -> "LayerTooltipsSpec":
        """Set the tooltip title template."""
        self._tooltip_title = value
        return self

    def disable_splitting(self) -> "LayerTooltipsSpec":
        """Show all lines in one tooltip box."""
        self._disable_splitting = True
        return self
