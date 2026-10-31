"""Render the images used by the docs pages into ``docs/images/``.

Run after changing an example: ``uv run python docs/render_images.py``.
"""

from pathlib import Path

import ggrust as gg
import polars as pl
from ggrust.plot.core import PlotSpec

IMAGES = Path(__file__).parent / "images"


# Approximate monthly mean temperatures, January to June.
OSLO = [-4.3, -4.0, -0.2, 4.5, 10.8, 15.2]
LIMA = [22.1, 22.6, 22.1, 20.4, 18.6, 17.3]
PUNE = [21.0, 23.4, 27.5, 30.4, 30.6, 27.1]


def weather_plot() -> PlotSpec:
    """Return the getting-started example shown on the home page."""
    frame = pl.DataFrame(
        {
            "city": ["Oslo"] * 6 + ["Lima"] * 6 + ["Pune"] * 6,
            "month": list(range(1, 7)) * 3,
            "temperature": OSLO + LIMA + PUNE,
        }
    )
    return (
        gg.ggplot(frame, gg.aes("month", "temperature", color="city"))
        + gg.geom_line(size=1)
        + gg.geom_point(size=3)
        + gg.scale_color_brewer(palette="Set1")
        + gg.labs(x="Month", y="Mean temperature (°C)", color="City")
        + gg.theme_classic()
        + gg.ggsize(560, 340)
    )


if __name__ == "__main__":
    IMAGES.mkdir(exist_ok=True)
    gg.ggsave(weather_plot(), "weather.png", path=str(IMAGES))
