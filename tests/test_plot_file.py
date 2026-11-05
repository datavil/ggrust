"""Plot files (``.ggrs``): ``ggsave(plot, "x.ggrs")`` and ``ggload``.

Round trips must render the same picture; damaged or hostile files must fail with
``PlotFileError`` and never run code.
"""

from __future__ import annotations

import datetime
import hashlib
import io
import json
import warnings
import zipfile
from pathlib import Path

import ggrust as gg
import numpy as np
import polars as pl
import pytest
from ggrust import GgrustWarning
from ggrust.errors import ExportError, PlotFileError
from ggrust.plot.core import PlotSpec
from ggrust.plot.subplots import SupPlotsSpec
from ggrust.plot_file import dump_plot, read_plot

FONTS = Path(__file__).parent / "fonts"

FRAME = pl.DataFrame(
    {
        "weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1],
        "mpg": [31.0, 28.0, 24.0, 22.0, 19.0, 15.0],
        "origin": ["EU", "US", "EU", "JP", "US", "JP"],
    }
)


def _plot() -> PlotSpec:
    return (
        gg.ggplot(FRAME, gg.aes("weight", "mpg", color="origin"))
        + gg.geom_point(size=4)
        + gg.geom_smooth(gg.aes(group="origin"), method="lm")
        + gg.scale_color_manual(values=["#1b9e77", "#d95f02", "#7570b3"])
        + gg.theme_bw()
        + gg.ggtitle("Heavier cars use more fuel")
        + gg.ggtb()
        + gg.ggsize(500, 300)
    )


def _round_trip(plot: PlotSpec | SupPlotsSpec) -> PlotSpec | SupPlotsSpec:
    return read_plot(dump_plot(plot))


def _rewrite(content: bytes, change: dict[str, bytes | None], *, rehash: bool = False) -> bytes:
    """Return the archive with entries replaced (None removes), hashes updated if asked."""
    with zipfile.ZipFile(io.BytesIO(content)) as archive:
        entries = {name: archive.read(name) for name in archive.namelist()}
    for name, data in change.items():
        if data is None:
            entries.pop(name, None)
        else:
            entries[name] = data
    if rehash:
        manifest = json.loads(entries["manifest.json"])
        manifest["entries"] = {
            name: {"sha256": hashlib.sha256(data).hexdigest(), "size": len(data)}
            for name, data in entries.items()
            if name != "manifest.json"
        }
        entries["manifest.json"] = json.dumps(manifest).encode()
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        for name, data in entries.items():
            archive.writestr(name, data)
    return buffer.getvalue()


# Round trips --------------------------------------------------------------------------


def test_ggsave_and_ggload_render_the_same(tmp_path: Path) -> None:
    plot = _plot()
    path = gg.ggsave(plot, "fuel.ggrs", path=tmp_path)
    loaded = gg.ggload(path)
    assert isinstance(loaded, PlotSpec)
    assert loaded.to_svg() == plot.to_svg()
    plot.to_png(tmp_path / "saved.png")
    loaded.to_png(tmp_path / "loaded.png")
    assert (tmp_path / "loaded.png").read_bytes() == (tmp_path / "saved.png").read_bytes()
    assert loaded.as_dict().keys() == plot.as_dict().keys()
    assert loaded.data is not None
    assert loaded.data.equals(FRAME)


def test_loaded_plot_keeps_building() -> None:
    loaded = _round_trip(_plot())
    assert isinstance(loaded, PlotSpec)
    extended = loaded + gg.geom_line() + gg.labs(x="Weight")
    assert [layer.geom for layer in extended.layers] == ["point", "smooth", "line"]
    assert extended.labels["x"] == "Weight"


def test_grid_with_shared_frame_and_empty_cell() -> None:
    first = _plot()
    second = gg.ggplot(FRAME) + gg.geom_bar(gg.aes("origin"))
    grid = gg.gggrid([first, None, second], ncol=2, tag_levels="A")
    loaded = _round_trip(grid)
    assert isinstance(loaded, SupPlotsSpec)
    assert loaded.to_svg() == grid.to_svg()
    assert loaded.figures[1] is None
    figures = [figure for figure in loaded.figures if figure is not None]
    # The frame both plots share is stored once and comes back shared.
    assert figures[0].data is figures[1].data


def test_values_keep_their_types() -> None:
    plot = (
        gg.ggplot(FRAME, gg.aes("weight", "mpg"))
        + gg.geom_point(gg.aes(x=np.array([1.0, 2.0]), y=[3, 4]))
        + gg.scale_y_continuous(limits=(0, float("nan")), breaks=[10, 20])
        + gg.ggtb(layers={"points": 0})
        + gg.labs(title="When", caption=str(datetime.date(2026, 11, 5)))
        + gg.facet_wrap("origin")
        + gg.geom_text(gg.aes(label=gg.as_discrete("origin")))
    )
    loaded = _round_trip(plot)
    assert isinstance(loaded, PlotSpec)
    assert loaded.to_svg() == plot.to_svg()
    limits = loaded.scales[0]["limits"]
    assert isinstance(limits, tuple) and limits[0] == 0 and np.isnan(limits[1])
    mapping_x = loaded.layers[0].mapping["x"]
    assert isinstance(mapping_x, np.ndarray) and mapping_x.tolist() == [1.0, 2.0]


def test_pandas_and_pyarrow_data_come_back_as_their_kind() -> None:
    pandas = pytest.importorskip("pandas")
    pyarrow = pytest.importorskip("pyarrow")
    pandas_frame = FRAME.to_pandas()
    arrow_table = FRAME.to_arrow()
    plot = (
        gg.ggplot(pandas_frame, gg.aes("weight", "mpg"))
        + gg.geom_point()
        + gg.geom_line(data=arrow_table)
    )
    loaded = _round_trip(plot)
    assert isinstance(loaded, PlotSpec)
    assert isinstance(loaded.data, pandas.DataFrame)
    assert isinstance(loaded.layers[1].data, pyarrow.Table)
    assert loaded.to_svg() == plot.to_svg()


def test_registered_fonts_are_stored_and_registered_on_load() -> None:
    family = "Plot File Serif"
    gg.register_font(FONTS / "LiberationSerif-Regular.ttf", family=family)
    plot = _plot() + gg.theme(text=gg.element_text(family=family))
    content = dump_plot(plot)
    with zipfile.ZipFile(io.BytesIO(content)) as archive:
        manifest = json.loads(archive.read("manifest.json"))
        assert manifest["fonts"] == [{"entry": "fonts/0.font", "family": family}]
        assert archive.read("fonts/0.font") == (FONTS / "LiberationSerif-Regular.ttf").read_bytes()
    # Same font registered: loading registers nothing new and does not warn.
    with warnings.catch_warnings():
        warnings.simplefilter("error", GgrustWarning)
        loaded = read_plot(content)
    assert loaded.to_svg() == plot.to_svg()


def test_a_stored_font_added_to_a_session_family_warns() -> None:
    gg.register_font(FONTS / "LiberationSerif-Regular.ttf", family="Plot File Saved")
    content = dump_plot(_plot() + gg.theme(text=gg.element_text(family="Plot File Saved")))
    # In this session the family name holds another file (only the bold face).
    gg.register_font(FONTS / "LiberationSerif-Bold.ttf", family="Plot File Session")
    with zipfile.ZipFile(io.BytesIO(content)) as archive:
        manifest = json.loads(archive.read("manifest.json"))
        tree = archive.read("plot.json").replace(b"Plot File Saved", b"Plot File Session")
    manifest["fonts"][0]["family"] = "Plot File Session"
    content = _rewrite(content, {"plot.json": tree}, rehash=True)
    content = _rewrite(content, {"manifest.json": _with_fonts(content, manifest["fonts"])})
    with pytest.warns(GgrustWarning, match="'Plot File Session' is registered over"):
        read_plot(content)
    faces = dict(gg.registered_fonts())["Plot File Session"]
    assert sorted(faces) == ["bold", "plain"]


def test_pickle_round_trips() -> None:
    import pickle

    plot = _plot()
    assert pickle.loads(pickle.dumps(plot)).to_svg() == plot.to_svg()


# Saving errors ------------------------------------------------------------------------


def test_callables_cannot_be_saved() -> None:
    plot = (
        gg.ggplot(FRAME, gg.aes("weight", "mpg"))
        + gg.geom_point()
        + gg.scale_y_continuous(sec_axis=gg.sec_axis(lambda value: value * 0.425))
    )
    with pytest.raises(ExportError, match=r"scales\[0\].*sec_axis.*transform.*Python function"):
        dump_plot(plot)


def test_unknown_objects_cannot_be_saved() -> None:
    plot = gg.ggplot(FRAME, gg.aes("weight", "mpg")) + gg.geom_point(color=object())
    with pytest.raises(ExportError, match=r"layers\[0\].*builtins.object.*no safe stored form"):
        dump_plot(plot)


def test_ggsave_suggests_ggrs_for_a_typo(tmp_path: Path) -> None:
    with pytest.raises(ExportError, match="did you mean 'ggrs'"):
        gg.ggsave(_plot(), "fuel.ggr", path=tmp_path)


# Loading errors -----------------------------------------------------------------------


def test_not_a_zip() -> None:
    with pytest.raises(PlotFileError, match="not a zip archive"):
        read_plot(b"plain text")


def test_missing_manifest() -> None:
    content = _rewrite(dump_plot(_plot()), {"manifest.json": None})
    with pytest.raises(PlotFileError, match=r"no manifest\.json"):
        read_plot(content)


def test_edited_entry_fails_its_hash() -> None:
    content = dump_plot(_plot())
    with zipfile.ZipFile(io.BytesIO(content)) as archive:
        tree = archive.read("plot.json")
    edited = tree.replace(b"Heavier cars", b"Lighter cars")
    with pytest.raises(PlotFileError, match=r"'plot.json' does not match its SHA-256 hash"):
        read_plot(_rewrite(content, {"plot.json": edited}))


def test_extra_entry_is_rejected() -> None:
    content = _rewrite(dump_plot(_plot()), {"../escape.txt": b"x"})
    with pytest.raises(PlotFileError, match=r"not listed in manifest\.json"):
        read_plot(content)


def test_newer_format_version_asks_for_an_upgrade() -> None:
    content = dump_plot(_plot())
    with zipfile.ZipFile(io.BytesIO(content)) as archive:
        manifest = json.loads(archive.read("manifest.json"))
    manifest["format_version"] = 99
    content = _rewrite(content, {"manifest.json": json.dumps(manifest).encode()})
    with pytest.raises(PlotFileError, match=r"format version 99.*Upgrade ggrust"):
        read_plot(content)


def _with_fonts(content: bytes, fonts: list[dict[str, str]]) -> bytes:
    """Return ``manifest.json`` of ``content`` with another fonts list."""
    with zipfile.ZipFile(io.BytesIO(content)) as archive:
        manifest = json.loads(archive.read("manifest.json"))
    manifest["fonts"] = fonts
    return json.dumps(manifest).encode()


def _with_tree(tree: object) -> bytes:
    return _rewrite(dump_plot(_plot()), {"plot.json": json.dumps(tree).encode()}, rehash=True)


def test_classes_outside_ggrust_are_never_created() -> None:
    tree = {"$object": "os:system", "state": {}}
    with pytest.raises(PlotFileError, match="'os:system' is not a ggrust plot class"):
        read_plot(_with_tree(tree))


def test_out_of_range_frame_reference() -> None:
    tree = {"$frame": 7}
    with pytest.raises(PlotFileError, match="data frame reference 7 is out of range"):
        read_plot(_with_tree(tree))


def test_tree_that_is_not_a_plot() -> None:
    with pytest.raises(PlotFileError, match="holds a list, not a plot"):
        read_plot(_with_tree([1, 2]))


def test_deep_nesting_is_rejected() -> None:
    tree: object = 0
    for _ in range(300):
        tree = [tree]
    with pytest.raises(PlotFileError, match=r"nesting is deeper"):
        read_plot(_with_tree(tree))


def test_broken_arrow_entry() -> None:
    content = _rewrite(dump_plot(_plot()), {"data/0.arrow": b"not arrow"}, rehash=True)
    with pytest.raises(PlotFileError, match=r"'data/0.arrow' is not valid Arrow IPC"):
        read_plot(content)


def test_zip_bomb_entry_is_rejected(monkeypatch: pytest.MonkeyPatch) -> None:
    # Deflate expands at most ~1032 times, under the real limit: test the guard lower.
    import ggrust.plot_file

    monkeypatch.setattr(ggrust.plot_file, "MAXIMUM_EXPANSION", 100)
    monkeypatch.setattr(ggrust.plot_file, "EXPANSION_ALLOWANCE", 0)
    content = _rewrite(dump_plot(_plot()), {}, rehash=True)
    buffer = io.BytesIO()
    with zipfile.ZipFile(io.BytesIO(content)) as source, zipfile.ZipFile(buffer, "w") as target:
        for name in source.namelist():
            target.writestr(name, source.read(name))
        target.writestr("bomb", b"\0" * 50_000_000, zipfile.ZIP_DEFLATED)
    with pytest.raises(PlotFileError, match="zip bomb"):
        read_plot(buffer.getvalue())


def test_ggload_missing_file(tmp_path: Path) -> None:
    with pytest.raises(FileNotFoundError):
        gg.ggload(tmp_path / "missing.ggrs")


# Bug-hunter regressions ---------------------------------------------------------------


def test_numpy_non_finite_floats_round_trip() -> None:
    plot = (
        gg.ggplot(FRAME, gg.aes("weight", "mpg"))
        + gg.geom_point()
        + gg.xlim(np.float64(0), np.float64("inf"))
    )
    assert _round_trip(plot).to_svg() == plot.to_svg()


def test_failed_load_registers_no_fonts() -> None:
    gg.register_font(FONTS / "LiberationSerif-Regular.ttf", family="Plot File Failed")
    content = dump_plot(_plot() + gg.theme(text=gg.element_text(family="Plot File Failed")))
    with zipfile.ZipFile(io.BytesIO(content)) as archive:
        fonts = json.loads(archive.read("manifest.json"))["fonts"]
    fonts[0]["family"] = "Plot File Never"
    broken = _rewrite(content, {"plot.json": b'"junk"'}, rehash=True)
    broken = _rewrite(broken, {"manifest.json": _with_fonts(broken, fonts)})
    with pytest.raises(PlotFileError):
        read_plot(broken)
    assert "Plot File Never" not in dict(gg.registered_fonts())


def test_font_family_not_named_by_the_plot_is_rejected() -> None:
    gg.register_font(FONTS / "LiberationSerif-Regular.ttf", family="Plot File Named")
    content = dump_plot(_plot() + gg.theme(text=gg.element_text(family="Plot File Named")))
    with zipfile.ZipFile(io.BytesIO(content)) as archive:
        fonts = json.loads(archive.read("manifest.json"))["fonts"]
    fonts[0]["family"] = "sans"
    content = _rewrite(content, {"manifest.json": _with_fonts(content, fonts)})
    with pytest.raises(PlotFileError, match="'sans' is not used by the plot"):
        read_plot(content)


def test_repeated_frame_references_are_read_once() -> None:
    content = dump_plot(_plot())
    with zipfile.ZipFile(io.BytesIO(content)) as archive:
        manifest = json.loads(archive.read("manifest.json"))
    manifest["frames"] = manifest["frames"] * 10
    content = _rewrite(content, {"manifest.json": json.dumps(manifest).encode()})
    loaded = read_plot(content)
    assert loaded.to_svg() == _plot().to_svg()


def test_compressed_arrow_bomb_is_stopped() -> None:
    bomb = io.BytesIO()
    pl.DataFrame({"zero": np.zeros(20_000_000)}).write_ipc(bomb, compression="zstd")
    content = dump_plot(_plot())
    with zipfile.ZipFile(io.BytesIO(content)) as archive:
        manifest = json.loads(archive.read("manifest.json"))
    manifest["frames"].append({"entry": "data/9.arrow", "kind": "polars"})
    content = _rewrite(
        content,
        {"data/9.arrow": bomb.getvalue(), "manifest.json": json.dumps(manifest).encode()},
        rehash=True,
    )
    with pytest.raises(PlotFileError, match="compression bomb"):
        read_plot(content)


@pytest.mark.parametrize(
    ("damage", "message"),
    [
        ("deflate", "could not be read"),
        ("encrypted", "is encrypted"),
        ("method", "compression method 99"),
    ],
)
def test_damaged_archives_raise_plot_file_error(damage: str, message: str) -> None:
    content = bytearray(dump_plot(_plot()))
    with zipfile.ZipFile(io.BytesIO(bytes(content))) as archive:
        info = archive.getinfo("plot.json")
    if damage == "deflate":
        start = info.header_offset + 30 + len(info.filename) + len(info.extra)
        content[start : start + 8] = b"\xff" * 8
    else:
        # The central directory record of plot.json: name 46 bytes after its start.
        directory = bytes(content).index(b"PK\x01\x02")
        record = bytes(content).index(b"plot.json", directory) - 46
        if damage == "encrypted":
            content[record + 8] |= 0x1
        else:
            content[record + 10 : record + 12] = (99).to_bytes(2, "little")
    with pytest.raises(PlotFileError, match=message):
        read_plot(bytes(content))


def test_very_deep_json_raises_plot_file_error() -> None:
    content = _rewrite(dump_plot(_plot()), {"plot.json": b"[" * 100_000}, rehash=True)
    with pytest.raises(PlotFileError, match="not valid JSON"):
        read_plot(content)


def test_large_repetitive_column_data_round_trips() -> None:
    plot = (
        gg.ggplot({"x": [0] * 2_000_000, "y": [0] * 2_000_000}, gg.aes("x", "y")) + gg.geom_point()
    )
    loaded = _round_trip(plot)
    assert isinstance(loaded, PlotSpec)
    assert isinstance(loaded.data, dict)
    assert loaded.data["x"][:3] == [0, 0, 0]


def test_unordered_pandas_categoricals_keep_their_categories() -> None:
    pandas = pytest.importorskip("pandas")
    pytest.importorskip("pyarrow")
    frame = pandas.DataFrame(
        {
            "x": [1.0, 2.0, 3.0, 4.0],
            "y": [1.0, 2.0, 3.0, 4.0],
            "group": pandas.Categorical(["b", "a", "b", "c"], categories=["c", "b", "a", "z"]),
        }
    )
    plot = gg.ggplot(frame, gg.aes("x", "y", color="group")) + gg.geom_point()
    loaded = _round_trip(plot)
    assert isinstance(loaded, PlotSpec)
    assert list(loaded.data["group"].cat.categories) == ["c", "b", "a", "z"]
    assert loaded.to_svg() == plot.to_svg()


def test_pyarrow_record_batch_saves() -> None:
    pyarrow = pytest.importorskip("pyarrow")
    batch = FRAME.to_arrow().to_batches()[0]
    plot = gg.ggplot(batch, gg.aes("weight", "mpg")) + gg.geom_point()
    loaded = _round_trip(plot)
    assert isinstance(loaded, PlotSpec)
    assert isinstance(loaded.data, pyarrow.Table)
    assert loaded.to_svg() == plot.to_svg()


def test_family_spelled_in_another_case_is_stored_as_registered() -> None:
    gg.register_font(FONTS / "LiberationSerif-Regular.ttf", family="Plot File Case")
    plot = _plot() + gg.theme(text=gg.element_text(family="plot file case"))
    content = dump_plot(plot)
    with zipfile.ZipFile(io.BytesIO(content)) as archive:
        fonts = json.loads(archive.read("manifest.json"))["fonts"]
    assert fonts == [{"entry": "fonts/0.font", "family": "Plot File Case"}]


def test_bad_float_message_is_not_nested() -> None:
    tree = {"$object": "ggrust.plot.core:PlotSpec", "state": {"x": {"$float": "huge"}}}
    with pytest.raises(PlotFileError) as error:
        read_plot(_with_tree(tree))
    assert str(error.value).count("invalid") == 1
