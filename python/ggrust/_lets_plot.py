"""``LetsPlot`` compatibility shim: settings calls that have no effect in ggrust."""


class LetsPlot:
    """lets_plot's global settings object. ggrust needs no setup; calls are accepted."""

    @classmethod
    def setup_html(cls, *args, **kwargs) -> None:
        """No-op: every ggrust HTML output embeds its own small runtime; nothing to load."""
