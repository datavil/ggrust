"""``SupPlotsSpec``: a figure made of several plots (``gggrid``).

Ported from lets-plot's ``lets_plot/plot/subplots.py`` (MIT, see THIRD_PARTY_NOTICES.md)
with identical ``as_dict()`` output and ``+`` semantics. The engine lays grids out
(``crates/ggrust-core/src/grid.rs``).
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from ggrust.plot.core import (
    DummySpec,
    FeatureSpec,
    FeatureSpecArray,
    _specs_to_dict,
    _theme_dicts_merge,
)

if TYPE_CHECKING:
    from typing import Any

    from ggrust._typing import Number, Target
    from ggrust.plot.core import PlotSpec


__all__ = ["SupPlotsSpec"]


class SupPlotsLayoutSpec:
    """Layout of a ``SupPlotsSpec`` (``name="grid"`` plus its options)."""

    def __init__(self, name: str, **kwargs: object) -> None:
        self.__props = dict(kwargs)
        self.__props["name"] = name

    def as_dict(self) -> dict[str, Any]:
        """Return the layout options without ``None`` values."""
        return _specs_to_dict(self.__props)


class SupPlotsSpec(FeatureSpec):
    """Several plots in one figure. Build it with ``gggrid()``."""

    @classmethod
    def duplicate(cls, other: SupPlotsSpec) -> SupPlotsSpec:
        """Return a shallow copy of ``other``.

        Parameters
        ----------
        other : SupPlotsSpec
            The spec to copy.
        """
        duplicate = SupPlotsSpec(figures=other.__figures, layout=other.__layout)
        duplicate.props().update(other.props())
        return duplicate

    def __init__(
        self, figures: list[PlotSpec | SupPlotsSpec | None], layout: SupPlotsLayoutSpec
    ) -> None:
        super().__init__("subplots", name=None)
        self.__figures = list(figures)
        self.__layout = layout

    def __add__(self, other: object) -> SupPlotsSpec:
        if isinstance(other, DummySpec):
            return self
        if isinstance(other, FeatureSpecArray):
            supplots = SupPlotsSpec.duplicate(self)
            for element in other.elements():
                supplots = supplots.__add__(element)
            return supplots
        if isinstance(other, FeatureSpec) and other.kind in [
            "ggsize",
            "theme",
            "ggtitle",
            "caption",
            "tag",
            "ggtoolbar",
        ]:
            supplots = SupPlotsSpec.duplicate(self)
            if other.kind == "theme":
                new_options = {
                    key: value for key, value in other.props().items() if value is not None
                }
                if "name" in new_options:
                    # A named theme replaces the theme, keeping the flavor.
                    flavor = supplots.props().get("theme", {}).get("flavor")
                    if flavor is not None:
                        new_options["flavor"] = flavor
                    supplots.props()["theme"] = new_options
                else:
                    old_options = supplots.props().get("theme", {})
                    supplots.props()["theme"] = _theme_dicts_merge(old_options, new_options)
                return supplots
            supplots.props()[other.kind] = other
            return supplots
        message = f"unsupported operand type(s) for +: {self.__class__} and {other.__class__}"
        raise TypeError(message)

    def as_dict(self) -> dict[str, Any]:
        """Return the figure as a lets_plot-compatible dictionary.

        Returns
        -------
        dict
            Keys as lets_plot's ``SupPlotsSpec.as_dict()``: ``kind``, ``layout``,
            ``figures`` (each ``as_dict()`` or None for an empty cell), ...
        """
        result = super().as_dict()
        result["kind"] = self.kind
        result["layout"] = self.__layout.as_dict()
        result["figures"] = [
            None if figure is None else figure.as_dict() for figure in self.__figures
        ]
        return result

    def _repr_html_(self) -> str:
        from ggrust._render import render_notebook_html

        return render_notebook_html(self)

    def show(self) -> None:
        """Draw the plot: inline in Jupyter/IPython, else in the default browser.

        Outside IPython the interactive HTML page is written to a temporary file
        and opened in the browser (lets_plot prints the spec dict there).

        Examples
        --------
        >>> from ggrust import ggplot, geom_point
        >>> (ggplot() + geom_point(x=0, y=0)).show()  # doctest: +SKIP
        """
        from ggrust._render import show

        show(self)

    def to_svg(
        self,
        path: Target | None = None,
        w: Number | None = None,
        h: Number | None = None,
        unit: str | None = None,
    ) -> str | None:
        """Render to SVG; parameters as ``PlotSpec.to_svg``.

        Parameters
        ----------
        path : str or file-like, optional
            Where to write: a file path or a binary file-like object.
        w : float, optional
            Width.
        h : float, optional
            Height.
        unit : str, optional
            Unit of ``w`` and ``h``: ``"in"``, ``"cm"``, ``"mm"`` or ``"px"``.
        """
        from ggrust._render import export_svg

        return export_svg(self, path, w=w, h=h, unit=unit)

    def to_png(
        self,
        path: Target,
        scale: Number | None = None,
        w: Number | None = None,
        h: Number | None = None,
        unit: str | None = None,
        dpi: Number | None = None,
    ) -> str | None:
        """Render to PNG; parameters as ``PlotSpec.to_png``.

        Parameters
        ----------
        path : str or file-like
            Where to write: a file path or a binary file-like object.
        scale : float, optional
            Multiplier of the pixel size.
        w : float, optional
            Width.
        h : float, optional
            Height.
        unit : str, optional
            Unit of ``w`` and ``h``: ``"in"``, ``"cm"``, ``"mm"`` or ``"px"``.
        dpi : float, optional
            Resolution in dots per inch.
        """
        from ggrust._render import export_png

        return export_png(self, path, scale=scale, w=w, h=h, unit=unit, dpi=dpi)

    def to_pdf(
        self,
        path: Target,
        scale: Number | None = None,
        w: Number | None = None,
        h: Number | None = None,
        unit: str | None = None,
        dpi: Number | None = None,
    ) -> str | None:
        """Render to PDF; parameters as ``PlotSpec.to_pdf``.

        Parameters
        ----------
        path : str or file-like
            Where to write: a file path or a binary file-like object.
        scale : float, optional
            Multiplier of the pixel size.
        w : float, optional
            Width.
        h : float, optional
            Height.
        unit : str, optional
            Unit of ``w`` and ``h``: ``"in"``, ``"cm"``, ``"mm"`` or ``"px"``.
        dpi : float, optional
            Resolution in dots per inch.
        """
        from ggrust._render import export_pdf

        return export_pdf(self, path, scale=scale, w=w, h=h, unit=unit, dpi=dpi)

    def to_html(self, path: Target | None = None, iframe: bool | None = None) -> str | None:
        """Render to standalone HTML; parameters as ``PlotSpec.to_html``.

        Parameters
        ----------
        path : str or file-like, optional
            Where to write: a file path or a binary file-like object.
        iframe : bool, optional
            Return an ``<iframe srcdoc>`` sized to the plot instead of a page.
        """
        from ggrust._render import export_html

        return export_html(self, path, iframe)
