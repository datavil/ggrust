"""``ggsave`` to JPEG and TIFF: same pixels and sizes as PNG, resolution metadata,
JPEG quality, transparency, and clear errors for options that do not apply.
"""

from pathlib import Path

import ggrust as gg
import pytest
from ggrust.errors import ExportError
from PIL import Image, ImageChops, ImageStat

FRAME = {"x": [1.0, 2.0, 3.0, 4.0], "y": [1.0, 3.0, 2.0, 4.0], "g": ["a", "b", "a", "b"]}


def plot() -> gg.PlotSpec:
    return (
        gg.ggplot(FRAME, gg.aes("x", "y", color="g"))
        + gg.geom_point(size=6)
        + gg.geom_line()
        + gg.ggtitle("Raster formats")
    )


def save(tmp_path: Path, name: str, **options: object) -> Image.Image:
    written = gg.ggsave(plot(), name, path=str(tmp_path), **options)  # type: ignore[arg-type]
    assert written == str(tmp_path / name)
    image = Image.open(written)
    image.load()
    return image


OPTIONS: list[dict[str, object]] = [
    {},
    {"scale": 1},
    {"dpi": 150},
    {"w": 4, "h": 3, "unit": "in"},
    {"w": 10, "h": 5, "unit": "cm", "dpi": 200},
    {"w": 300, "h": 200, "unit": "px"},
]


@pytest.mark.parametrize("options", OPTIONS)
@pytest.mark.parametrize("extension", ["tif", "tiff", "jpg", "jpeg"])
def test_raster_matches_png_size_and_dpi(
    tmp_path: Path, extension: str, options: dict[str, object]
) -> None:
    png = save(tmp_path, "plot.png", **options)
    image = save(tmp_path, f"plot.{extension}", **options)
    assert image.format == ("TIFF" if extension.startswith("tif") else "JPEG")
    assert image.size == png.size
    # PNG stores dots per meter (rounded), TIFF and JPEG store dots per inch.
    expected = png.info["dpi"][0]
    assert image.info["dpi"][0] == pytest.approx(expected, abs=0.6)
    assert image.info["dpi"][1] == pytest.approx(expected, abs=0.6)


def test_tiff_pixels_equal_png(tmp_path: Path) -> None:
    png = save(tmp_path, "plot.png", dpi=150).convert("RGB")
    tiff = save(tmp_path, "plot.tiff", dpi=150)
    assert tiff.mode == "RGB"  # an opaque plot drops the alpha channel
    assert tiff.info["compression"] == "tiff_lzw"
    assert ImageChops.difference(png, tiff).getbbox() is None


def test_jpeg_is_close_to_png_and_quality_trades_size(tmp_path: Path) -> None:
    png = save(tmp_path, "plot.png").convert("RGB")
    low = save(tmp_path, "low.jpg", quality=20)
    default = save(tmp_path, "default.jpg")
    high = save(tmp_path, "high.jpg", quality=95)
    sizes = [(tmp_path / name).stat().st_size for name in ["low.jpg", "default.jpg", "high.jpg"]]
    assert sizes[0] < sizes[1] < sizes[2]
    errors = [
        sum(ImageStat.Stat(ImageChops.difference(png, image.convert("RGB"))).mean) / 3
        for image in [low, default, high]
    ]
    assert errors[2] < errors[1] < errors[0]
    assert errors[2] < 1.0


def test_transparent_background(tmp_path: Path) -> None:
    clear = plot() + gg.theme(plot_background=gg.element_blank())
    gg.ggsave(clear, "clear.tiff", path=str(tmp_path))
    gg.ggsave(clear, "clear.png", path=str(tmp_path))
    gg.ggsave(clear, "clear.jpg", path=str(tmp_path), quality=100)
    tiff = Image.open(tmp_path / "clear.tiff")
    png = Image.open(tmp_path / "clear.png").convert("RGBA")
    assert tiff.mode == "RGBA"
    assert tiff.getpixel((0, 0)) == (0, 0, 0, 0)
    assert ImageChops.difference(png, tiff).getbbox() is None
    # JPEG has no alpha: transparent pixels become white, as R's jpeg() device.
    jpeg = Image.open(tmp_path / "clear.jpg")
    assert all(channel >= 250 for channel in jpeg.getpixel((0, 0)))


def test_grid_saves_as_tiff(tmp_path: Path) -> None:
    grid = gg.gggrid([plot(), plot()])
    gg.ggsave(grid, "grid.tiff", path=str(tmp_path))
    gg.ggsave(grid, "grid.png", path=str(tmp_path))
    tiff = Image.open(tmp_path / "grid.tiff")
    assert tiff.size == Image.open(tmp_path / "grid.png").size


@pytest.mark.parametrize(
    ("name", "options", "message"),
    [
        ("plot.png", {"quality": 90}, r"'quality' applies to \.jpg / \.jpeg only, not \.png"),
        ("plot.tiff", {"quality": 90}, r"'quality' applies to \.jpg / \.jpeg only, not \.tiff"),
        ("plot.jpg", {"quality": 0}, "'quality' must be an integer from 1 to 100, got 0"),
        ("plot.jpg", {"quality": 101}, "got 101"),
        ("plot.jpg", {"quality": 80.5}, "got 80.5"),
        ("plot.jpg", {"quality": True}, "got True"),
        ("plot.tiff", {"dpi": 0}, "'dpi' must be a positive number, got 0"),
        ("plot.jpeg", {"scale": -1}, "'scale' must be a positive number"),
        ("plot.tiffs", {}, "did you mean 'tiff'"),
    ],
)
def test_option_errors(tmp_path: Path, name: str, options: dict[str, object], message: str) -> None:
    with pytest.raises(ExportError, match=message):
        gg.ggsave(plot(), name, path=str(tmp_path), **options)  # type: ignore[arg-type]
    assert not (tmp_path / name).exists()


def test_numpy_integer_quality(tmp_path: Path) -> None:
    import numpy as np

    image = save(tmp_path, "plot.jpg", quality=np.int64(90))
    assert image.format == "JPEG"
