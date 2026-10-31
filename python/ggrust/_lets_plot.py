"""``LetsPlot`` compatibility shim: settings calls that have no effect in ggrust."""

from __future__ import annotations


class LetsPlot:
    """lets_plot's global settings object. ggrust needs no setup; calls are accepted."""

    @classmethod
    def setup_html(cls, *args: object, **kwargs: object) -> None:
        """No-op: every ggrust HTML output embeds its own small runtime; nothing to load.

        Parameters
        ----------
        *args, **kwargs
            lets_plot's options (``offline``, ``no_js``, ...), accepted and ignored.
        """
