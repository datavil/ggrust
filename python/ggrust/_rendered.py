"""Rendered plot outputs that Jupyter draws: ``plot.svg``, ``plot.png``, ``plot.html``.

Each is a ``str`` or ``bytes`` subclass, so it works as plain text or bytes (write it
to a file, slice it, compare it) and also shows as the picture when it ends a
notebook cell. They do not change the session's display mode (``ggrust.display``).
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from typing import Self

    from ggrust.plot.core import PlotSpec
    from ggrust.plot.subplots import SupPlotsSpec


class SVG(str):
    """A rendered SVG document that Jupyter draws as a static vector image.

    Returned by ``plot.svg``. It is a ``str`` holding the same text as
    ``plot.to_svg()``.

    Examples
    --------
    End a notebook cell with ``plot.svg`` to show the static image:

    >>> from ggrust import *
    >>> from ggrust.data import mpg
    >>> plot = ggplot(mpg, aes("displ", "hwy", color="drv")) + geom_point(size=3)
    >>> plot.svg  # doctest: +SKIP
    """

    __slots__ = ()

    def _repr_svg_(self) -> str:
        """Return the SVG text for Jupyter's ``image/svg+xml`` output."""
        return str(self)

    def _repr_pretty_(self, printer: Any, cycle: bool) -> None:  # noqa: ANN401 (IPython's protocol)
        """Print a short summary instead of the whole document in IPython."""
        printer.text(f"<SVG: {len(self):,} characters>")


class PNG(bytes):
    """A rendered PNG image that Jupyter draws inline.

    Returned by ``plot.png``. It is ``bytes`` holding the encoded PNG, the same
    image ``ggrust.display("png")`` shows.

    Examples
    --------
    End a notebook cell with ``plot.png`` to show the picture:

    >>> from ggrust import *
    >>> from ggrust.data import mpg
    >>> plot = ggplot(mpg, aes("displ", "hwy", color="drv")) + geom_point(size=3)
    >>> plot.png  # doctest: +SKIP
    """

    __slots__ = ()

    def _repr_png_(self) -> bytes:
        """Return the PNG bytes for Jupyter's ``image/png`` output."""
        return bytes(self)

    def _repr_pretty_(self, printer: Any, cycle: bool) -> None:  # noqa: ANN401 (IPython's protocol)
        """Print a short summary instead of the raw bytes in IPython."""
        printer.text(f"<PNG: {len(self):,} bytes>")


class HTML(str):
    """A standalone interactive HTML page that Jupyter shows as the interactive plot.

    Returned by ``plot.html``. It is a ``str`` holding the same page as
    ``plot.to_html()``; in a notebook it shows with tooltips and the zoom/pan
    toolbar, whatever ``ggrust.display`` is set to.

    Examples
    --------
    End a notebook cell with ``plot.html`` to show the interactive plot:

    >>> from ggrust import *
    >>> from ggrust.data import mpg
    >>> plot = ggplot(mpg, aes("displ", "hwy", color="drv")) + geom_point(size=3)
    >>> plot.html  # doctest: +SKIP
    """

    _plot: PlotSpec | SupPlotsSpec

    @classmethod
    def _from_plot(cls, page: str, plot: PlotSpec | SupPlotsSpec) -> Self:
        """Wrap ``page`` and keep ``plot`` to render the notebook view on display."""
        rendered = cls(page)
        rendered._plot = plot
        return rendered

    def _repr_html_(self) -> str:
        """Return the interactive notebook HTML (a fragment, not the full page)."""
        plot = getattr(self, "_plot", None)
        if plot is None:
            return str(self)
        from ggrust._render import render_interactive_html

        return render_interactive_html(plot)

    def _repr_pretty_(self, printer: Any, cycle: bool) -> None:  # noqa: ANN401 (IPython's protocol)
        """Print a short summary instead of the whole page in IPython."""
        printer.text(f"<HTML: {len(self):,} characters>")
