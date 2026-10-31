"""Smoke test for a freshly installed ggrust wheel: render SVG, PNG and HTML.

Run from outside the source tree so the installed wheel is imported, not `python/`.
"""

import io

import ggrust as gg
import polars as pl

frame = pl.DataFrame(
    {
        "city": ["Oslo", "Lima", "Pune", "Oslo", "Lima", "Pune"],
        "month": [1, 1, 1, 2, 2, 2],
        "temperature": [-4.3, 22.1, 21.0, -3.9, 22.6, 23.4],
    }
)
plot = (
    gg.ggplot(frame, gg.aes("month", "temperature", color="city"))
    + gg.geom_line()
    + gg.geom_point()
    + gg.ggsize(400, 300)
)

svg = plot.to_svg()
assert svg.lstrip().startswith("<svg"), svg[:80]

png = io.BytesIO()
plot.to_png(png)
assert png.getvalue()[:8] == b"\x89PNG\r\n\x1a\n"

html = plot.to_html()
assert "<svg" in html

print(f"ggrust {gg.__version__}: svg {len(svg)} B, png {len(png.getvalue())} B, html {len(html)} B")
