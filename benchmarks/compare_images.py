"""Stack lets-plot and ggrust PNGs of every scenario into one comparison figure.

uv run python benchmarks/compare_images.py benchmarks/results/scenarios-<stamp>-images
"""

import sys
from pathlib import Path

from PIL import Image, ImageDraw


def main(directory: Path) -> None:
    """Write ``comparison.png`` into ``directory`` (rows: scenarios, columns: libraries)."""
    names = sorted({p.name.rsplit("-", 1)[0] for p in directory.glob("*-ggrust.png")})
    rows = []
    for name in names:
        pair = [
            Image.open(directory / f"{name}-{lib}.png").convert("RGB")
            for lib in ("lets-plot", "ggrust")
        ]
        height = max(image.height for image in pair)
        width = sum(image.width for image in pair) + 30
        row = Image.new("RGB", (width, height + 40), "white")
        ImageDraw.Draw(row).text(
            (10, 10), f"{name}:  lets-plot (left)  |  ggrust (right)", fill="black"
        )
        row.paste(pair[0], (0, 40))
        row.paste(pair[1], (pair[0].width + 30, 40))
        rows.append(row)
    sheet = Image.new("RGB", (max(r.width for r in rows), sum(r.height for r in rows)), "white")
    top = 0
    for row in rows:
        sheet.paste(row, (0, top))
        top += row.height
    sheet.save(directory / "comparison.png")
    print(directory / "comparison.png", sheet.size)


if __name__ == "__main__":
    main(Path(sys.argv[1]))
