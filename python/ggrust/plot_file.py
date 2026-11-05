"""Plot files (``.ggrs``): save a plot and open it again in another Python session.

A ``.ggrs`` file is a zip archive:

- ``manifest.json``: format name and version, the ggrust version that wrote it, and
  the SHA-256 hash and size of every other entry.
- ``plot.json``: the plot objects (``PlotSpec``, layers, scales, ``gggrid``, ...) as
  tagged JSON. Classes are named and resolved against a fixed list of ggrust classes.
- ``data/<n>.arrow``: every data frame once, as Arrow IPC (deflated by the zip).
- ``fonts/<n>.font``: the font files of registered families the plot names.

Opening a file never runs code from it: no pickle, no imports by name, no callables.
Entry names, counts and sizes are checked before anything is read, and every entry
must match its hash. Plots holding Python callables (``sec_axis(transform=...)``) or
objects with no safe form (polars expressions) cannot be saved; the error names the
place. Hashes detect corruption, not deliberate edits: whoever edits a file can update
them. What this module does not do: signing, or reading other formats.
"""

from __future__ import annotations

import base64
import datetime
import hashlib
import io
import json
import math
import os
import sys
import warnings
import zipfile
from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, Literal, NoReturn, cast

import polars as pl

from ggrust import _ggrust
from ggrust._render import GgrustWarning
from ggrust.errors import ExportError, PlotFileError

if TYPE_CHECKING:
    from ggrust.plot.core import PlotSpec
    from ggrust.plot.subplots import SupPlotsSpec

__all__ = ["ggload"]

FORMAT_NAME = "ggrs"
FORMAT_VERSION = 1

MAXIMUM_ENTRIES = 4096
"""More entries than this is not a file ggrust wrote."""
MAXIMUM_JSON_BYTES = 256 * 1024 * 1024
"""Largest ``manifest.json`` or ``plot.json`` accepted, uncompressed."""
MAXIMUM_EXPANSION = 2000
"""Largest ratio of declared or decoded data size to file size (zip bomb guard).

ggrust writes every entry stored or deflated (deflate expands at most about 1032
times), so its own files always stay below this."""
EXPANSION_ALLOWANCE = 64 * 1024 * 1024
"""Bytes allowed beyond ``MAXIMUM_EXPANSION`` times the file size (small files)."""
MAXIMUM_DEPTH = 200
"""Deepest nesting accepted in ``plot.json``."""

DataKind = Literal["polars", "pandas", "pyarrow", "columns"]
_DATA_KINDS: tuple[DataKind, ...] = ("polars", "pandas", "pyarrow", "columns")


# Saving ------------------------------------------------------------------------------


def write_plot_file(plot: PlotSpec | SupPlotsSpec, path: str) -> None:
    """Write ``plot`` to the ``.ggrs`` file ``path`` (``ggsave`` checks the arguments).

    Raises
    ------
    ExportError
        If the plot holds a value with no safe stored form.
    """
    content = dump_plot(plot)
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    with open(path, "wb") as file:
        file.write(content)


def dump_plot(plot: PlotSpec | SupPlotsSpec) -> bytes:
    """Return the ``.ggrs`` file content of ``plot`` (see ``ggsave``).

    Parameters
    ----------
    plot : PlotSpec or SupPlotsSpec
        The figure.

    Returns
    -------
    bytes
        The zip archive.

    Raises
    ------
    ExportError
        If the plot holds a value with no safe stored form.
    """
    encoder = _Encoder()
    tree = encoder.encode(plot, "plot")
    entries: dict[str, bytes] = {}
    frames = []
    for index, (data, kind) in enumerate(encoder.frames):
        name = f"data/{index}.arrow"
        entries[name] = _write_frame(data, kind)
        frames.append({"entry": name, "kind": kind})
    fonts = []
    registered = {name.lower(): name for name, _ in _ggrust.registered_fonts()}
    families = {registered[name.lower()] for name in encoder.families if name.lower() in registered}
    for family in sorted(families, key=str.lower):
        for data, embeddable in _ggrust.registered_font_files(family):
            if not embeddable:
                warnings.warn(
                    f"ggsave(): font family {family!r} is not stored in the file: its "
                    "license does not allow embedding; register the font again after "
                    "ggload()",
                    GgrustWarning,
                    stacklevel=3,
                )
                continue
            name = f"fonts/{len(fonts)}.font"
            entries[name] = data
            fonts.append({"entry": name, "family": family})
    entries["plot.json"] = json.dumps(tree, separators=(",", ":"), allow_nan=False).encode()
    manifest = {
        "format": FORMAT_NAME,
        "format_version": FORMAT_VERSION,
        "ggrust_version": _ggrust.engine_version(),
        "created": datetime.datetime.now(datetime.UTC).isoformat(timespec="seconds"),
        "frames": frames,
        "fonts": fonts,
        "entries": {
            name: {"sha256": hashlib.sha256(data).hexdigest(), "size": len(data)}
            for name, data in entries.items()
        },
    }
    buffer = io.BytesIO()
    # Every entry deflated (level 1: fast), Arrow included: deflate expands at most
    # about 1032 times, so the reader's expansion guard never rejects our own files.
    with zipfile.ZipFile(buffer, "w", zipfile.ZIP_DEFLATED, compresslevel=1) as archive:
        archive.writestr("manifest.json", json.dumps(manifest, indent=1))
        for name, data in entries.items():
            archive.writestr(name, data)
    return buffer.getvalue()


class _Encoder:
    """Turn plot objects into tagged JSON, collecting frames and font families."""

    def __init__(self) -> None:
        self.frames: list[tuple[object, DataKind]] = []
        self.families: set[str] = set()
        self._frame_index: dict[int, int] = {}
        self._classes = {cls: name for name, cls in _allowed_classes().items()}

    def encode(self, value: object, path: str) -> object:
        """Return the JSON form of ``value`` found at ``path``."""
        if value is None or isinstance(value, bool | str):
            return value
        if isinstance(value, int):
            return value
        if isinstance(value, float):
            return float(value) if math.isfinite(value) else {"$float": str(float(value))}
        kind = _data_kind(value)
        if kind is not None:
            return {"$frame": self._frame(value, kind)}
        if isinstance(value, pl.Series):
            return {"$series": self._frame(value.to_frame(), "polars")}
        if _is_numpy(value, "ndarray"):
            series = pl.Series("array", cast("Any", value))
            return {"$array": self._frame(series.to_frame(), "polars")}
        if _is_numpy(value, "generic"):
            return self.encode(cast("Any", value).item(), path)
        if isinstance(value, datetime.datetime):
            return {"$datetime": value.isoformat()}
        if isinstance(value, datetime.date):
            return {"$date": value.isoformat()}
        if isinstance(value, datetime.time):
            return {"$time": value.isoformat()}
        if isinstance(value, datetime.timedelta):
            return {"$timedelta": [value.days, value.seconds, value.microseconds]}
        if isinstance(value, bytes):
            return {"$bytes": base64.b64encode(value).decode()}
        if isinstance(value, list):
            return [self.encode(item, f"{path}[{index}]") for index, item in enumerate(value)]
        if isinstance(value, tuple):
            items = [self.encode(item, f"{path}[{index}]") for index, item in enumerate(value)]
            return {"$tuple": items}
        if isinstance(value, Mapping):
            return self._mapping(value, path)
        name = self._classes.get(type(value))
        if name is not None:
            state = {
                key: self.encode(item, f"{path}.{_public(key)}")
                for key, item in vars(value).items()
            }
            return {"$object": name, "state": state}
        if callable(value):
            message = (
                f"ggsave(): {path} is a Python function ({_describe(value)}); a plot "
                "file stores no code. Replace it with plain values before saving (a "
                "sec_axis without transform= shows the primary values; give breaks= and "
                "labels= lists for other ticks)"
            )
            raise ExportError(message)
        message = (
            f"ggsave(): {path} holds a {type(value).__module__}.{type(value).__qualname__} "
            "object, which has no safe stored form; replace it with plain values "
            "(numbers, strings, lists, dicts, data frames) before saving"
        )
        raise ExportError(message)

    def _mapping(self, value: Mapping[object, object], path: str) -> object:
        """Encode a dict: plain when keys are strings without ``$``, else as pairs."""
        if all(isinstance(key, str) and not key.startswith("$") for key in value):
            result = {}
            for key, item in value.items():
                key = cast("str", key)
                if key == "family" and isinstance(item, str):
                    self.families.add(item)
                if key == "data" and isinstance(item, Mapping) and item:
                    # A dict of columns is stored as Arrow, not inline JSON.
                    result[key] = {"$frame": self._frame(item, "columns")}
                    continue
                result[key] = self.encode(item, f"{path}.{key}")
            return result
        pairs = [
            [self.encode(key, f"{path} key"), self.encode(item, f"{path}[{key!r}]")]
            for key, item in value.items()
        ]
        return {"$dict": pairs}

    def _frame(self, data: object, kind: DataKind) -> int:
        """Return the index of ``data`` among the frames (the same object is stored once)."""
        index = self._frame_index.get(id(data))
        if index is None:
            index = self._frame_index[id(data)] = len(self.frames)
            self.frames.append((data, kind))
        return index


def _write_frame(data: object, kind: DataKind) -> bytes:
    """Return ``data`` as compressed Arrow IPC (file format)."""
    buffer = io.BytesIO()
    # Uncompressed IPC: the zip deflates it, so the expansion guard covers the data.
    if kind in ("polars", "columns"):
        if kind == "columns":
            from ggrust._data import as_arrow

            frame = cast("pl.DataFrame", as_arrow(data))
        else:
            frame = data.collect() if isinstance(data, pl.LazyFrame) else data
        cast("pl.DataFrame", frame).write_ipc(buffer, compression="uncompressed")
        return buffer.getvalue()
    import pyarrow
    import pyarrow.ipc

    if kind == "pandas":
        table = pyarrow.Table.from_pandas(data, preserve_index=False)
    elif isinstance(data, pyarrow.RecordBatch):
        table = pyarrow.Table.from_batches([data])
    else:
        table = cast("pyarrow.Table", data)
    with pyarrow.ipc.new_file(buffer, table.schema) as writer:
        writer.write_table(table)
    return buffer.getvalue()


def _data_kind(value: object) -> DataKind | None:
    """Return how a data frame is stored and restored, or None for other values."""
    if isinstance(value, pl.DataFrame | pl.LazyFrame):
        return "polars"
    if _is_pandas(value, "DataFrame"):
        return "pandas"
    pyarrow = sys.modules.get("pyarrow")
    if pyarrow is not None and isinstance(value, pyarrow.Table | pyarrow.RecordBatch):
        return "pyarrow"
    return None


# Loading -----------------------------------------------------------------------------


def ggload(path: str | os.PathLike[str]) -> PlotSpec | SupPlotsSpec:
    """Open a plot saved with ``ggsave(plot, "name.ggrs")``.

    The plot comes back as it was saved: add layers, render or save it again. Data
    comes back as the kind it was saved from (polars, pandas or pyarrow). Fonts stored
    in the file are registered under their family, over the faces of a family of
    that name registered before (with a warning).
    Opening a file never runs code from it.

    Parameters
    ----------
    path : str or os.PathLike
        A ``.ggrs`` file.

    Returns
    -------
    PlotSpec or SupPlotsSpec
        The figure.

    Raises
    ------
    FileNotFoundError
        If ``path`` does not exist.
    PlotFileError
        If the file is not a ggrust plot file, was written by a newer ggrust, or is
        damaged (an entry does not match its hash, is missing or too large).

    Examples
    --------
    >>> from ggrust import *
    >>> import os, tempfile
    >>> data = {"weight": [2.6, 2.9, 3.2, 3.4, 3.6, 4.1], "mpg": [31, 28, 24, 22, 19, 15]}
    >>> plot = ggplot(data, aes("weight", "mpg")) + geom_point()
    >>> path = ggsave(plot, "fuel.ggrs", path=tempfile.mkdtemp())
    >>> again = ggload(path) + geom_line()
    >>> [layer.geom for layer in again.layers]
    ['point', 'line']
    """
    source = os.fspath(path)
    with open(source, "rb") as file:
        content = file.read()
    return read_plot(content, source)


def read_plot(content: bytes, source: str = "<bytes>") -> PlotSpec | SupPlotsSpec:
    """Return the plot in ``.ggrs`` file content (see ``ggload``).

    Parameters
    ----------
    content : bytes
        The file content.
    source : str, default="<bytes>"
        Where the content came from, for error messages.

    Returns
    -------
    PlotSpec or SupPlotsSpec
        The figure.

    Raises
    ------
    PlotFileError
        If the content is not a valid plot file.
    """
    try:
        archive = zipfile.ZipFile(io.BytesIO(content))
    except zipfile.BadZipFile as error:
        message = f"{source}: not a ggrust plot file (not a zip archive: {error})"
        raise PlotFileError(message) from error
    with archive:
        reader = _Reader(archive, source, len(content))
        manifest = reader.manifest()
        frames = [reader.frame(item) for item in manifest["frames"]]
        fonts = [(reader.entry(item["entry"]), item["family"]) for item in manifest["fonts"]]
        tree = reader.json_entry("plot.json")
    decoder = _Decoder(frames, source)
    plot = decoder.decode(tree, "plot", 0)
    from ggrust.plot.core import PlotSpec
    from ggrust.plot.subplots import SupPlotsSpec

    if not isinstance(plot, PlotSpec | SupPlotsSpec):
        message = f"{source}: plot.json holds a {type(plot).__name__}, not a plot"
        raise PlotFileError(message)
    named = {family.lower() for family in decoder.families}
    for _, family in fonts:
        if family.lower() not in named:
            message = f"{source}: stored font family {family!r} is not used by the plot"
            raise PlotFileError(message)
    # Fonts change the session's registry: only once the whole file has loaded.
    _register_fonts(fonts, source)
    return plot


class _Reader:
    """Checked access to the entries of a plot file archive."""

    def __init__(self, archive: zipfile.ZipFile, source: str, file_size: int) -> None:
        self.archive = archive
        self.source = source
        self.hashes: dict[str, dict[str, Any]] = {}
        infos = archive.infolist()
        if len(infos) > MAXIMUM_ENTRIES:
            self._fail(f"it has {len(infos)} entries (at most {MAXIMUM_ENTRIES})")
        self.budget = MAXIMUM_EXPANSION * file_size + EXPANSION_ALLOWANCE
        self.infos = {}
        for info in infos:
            if info.filename in self.infos:
                self._fail(f"entry {info.filename!r} appears twice")
            if info.compress_type not in (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED):
                self._fail(
                    f"entry {info.filename!r} uses compression method {info.compress_type}; "
                    "plot files use stored or deflated entries"
                )
            if info.flag_bits & 0x1:
                self._fail(f"entry {info.filename!r} is encrypted")
            self.infos[info.filename] = info
        declared = sum(info.file_size for info in infos)
        if declared > self.budget:
            self._fail(
                f"its entries expand to {declared} bytes, more than {MAXIMUM_EXPANSION} times "
                "the file size (a zip bomb?)"
            )
        self.decoded = 0
        self.frames: dict[tuple[str, str], object] = {}

    def manifest(self) -> dict[str, Any]:
        """Read and check ``manifest.json``; every other entry must be listed in it."""
        if "manifest.json" not in self.infos:
            self._fail("it has no manifest.json (not a ggrust plot file)")
        try:
            manifest = json.loads(self._read("manifest.json", MAXIMUM_JSON_BYTES))
        except (ValueError, RecursionError) as error:
            self._fail(f"manifest.json is not valid JSON ({error})")
        if not isinstance(manifest, dict) or manifest.get("format") != FORMAT_NAME:
            self._fail("manifest.json does not name the 'ggrs' format (not a ggrust plot file)")
        version = manifest.get("format_version")
        if not isinstance(version, int) or isinstance(version, bool) or version < 1:
            self._fail(f"manifest.json has an invalid format_version {version!r}")
        if version > FORMAT_VERSION:
            saved_by = manifest.get("ggrust_version", "a newer ggrust")
            self._fail(
                f"it uses format version {version}, written by ggrust {saved_by}; this "
                f"ggrust ({_ggrust.engine_version()}) reads up to version {FORMAT_VERSION}. "
                "Upgrade ggrust to open it"
            )
        entries = manifest.get("entries")
        if not isinstance(entries, dict):
            self._fail("manifest.json has no 'entries' table")
        for name, record in entries.items():
            if (
                not isinstance(record, dict)
                or not isinstance(record.get("sha256"), str)
                or not isinstance(record.get("size"), int)
            ):
                self._fail(f"manifest.json has an invalid record for {name!r}")
        unlisted = sorted(set(self.infos) - set(entries) - {"manifest.json"})
        if unlisted:
            self._fail(f"entries not listed in manifest.json: {', '.join(map(repr, unlisted))}")
        self.hashes = entries
        frames = manifest.get("frames")
        fonts = manifest.get("fonts")
        if not isinstance(frames, list) or not all(
            isinstance(item, dict)
            and isinstance(item.get("entry"), str)
            and item.get("kind") in _DATA_KINDS
            for item in frames
        ):
            self._fail("manifest.json has an invalid 'frames' list")
        if not isinstance(fonts, list) or not all(
            isinstance(item, dict)
            and isinstance(item.get("entry"), str)
            and isinstance(item.get("family"), str)
            for item in fonts
        ):
            self._fail("manifest.json has an invalid 'fonts' list")
        return manifest

    def entry(self, name: str) -> bytes:
        """Return entry ``name`` after checking its size and hash against the manifest."""
        record = self.hashes.get(name)
        if record is None:
            self._fail(f"entry {name!r} is not listed in manifest.json")
        if name not in self.infos:
            self._fail(f"entry {name!r} is missing")
        if self.infos[name].file_size != record["size"]:
            self._fail(
                f"entry {name!r} has {self.infos[name].file_size} bytes, the manifest "
                f"says {record['size']} (the file is damaged or was edited)"
            )
        data = self._read(name, record["size"])
        if hashlib.sha256(data).hexdigest() != record["sha256"]:
            self._fail(
                f"entry {name!r} does not match its SHA-256 hash (the file is damaged "
                "or was edited)"
            )
        return data

    def json_entry(self, name: str) -> object:
        """Return the parsed JSON of a hashed entry."""
        if self.hashes.get(name, {}).get("size", 0) > MAXIMUM_JSON_BYTES:
            self._fail(f"entry {name!r} is larger than {MAXIMUM_JSON_BYTES} bytes")
        try:
            return json.loads(self.entry(name))
        except (ValueError, RecursionError) as error:
            self._fail(f"entry {name!r} is not valid JSON ({error})")

    def frame(self, item: dict[str, str]) -> object:
        """Return a stored data frame as the kind it was saved from (read once)."""
        key = (item["entry"], item["kind"])
        if key not in self.frames:
            self.frames[key] = self._decode_frame(item["entry"], cast("DataKind", item["kind"]))
        return self.frames[key]

    def _decode_frame(self, name: str, kind: DataKind) -> object:
        """Decode an Arrow IPC entry, keeping decoded data within the file's budget."""
        data = self.entry(name)
        if kind in ("pandas", "pyarrow"):
            try:
                import pyarrow
                import pyarrow.ipc
            except ImportError:
                warnings.warn(
                    f"{self.source}: data saved from {kind} is loaded as a polars "
                    "DataFrame (pyarrow is not installed)",
                    GgrustWarning,
                    stacklevel=5,
                )
            else:
                try:
                    table = pyarrow.ipc.open_file(pyarrow.BufferReader(data)).read_all()
                    table.validate(full=True)
                except Exception as error:
                    self._fail(f"entry {name!r} is not valid Arrow IPC ({error})")
                self._spend(int(table.nbytes), name)
                if kind == "pyarrow":
                    return table
                try:
                    return table.to_pandas()
                except ImportError:
                    warnings.warn(
                        f"{self.source}: data saved from pandas is loaded as a pyarrow "
                        "Table (pandas is not installed)",
                        GgrustWarning,
                        stacklevel=5,
                    )
                    return table
        try:
            frame = pl.read_ipc(io.BytesIO(data), memory_map=False)
        except Exception as error:
            self._fail(f"entry {name!r} is not valid Arrow IPC ({error})")
        self._spend(int(frame.estimated_size()), name)
        if kind == "columns":
            return frame.to_dict(as_series=False)
        return frame

    def _spend(self, size: int, name: str) -> None:
        """Count decoded bytes; fail once they pass the file's expansion budget."""
        self.decoded += size
        if self.decoded > self.budget:
            self._fail(
                f"entry {name!r} decodes to more data than {MAXIMUM_EXPANSION} times the "
                "file size (a compression bomb?)"
            )

    def _read(self, name: str, size: int) -> bytes:
        """Read at most ``size`` bytes of entry ``name``."""
        try:
            with self.archive.open(self.infos[name]) as file:
                data = file.read(size + 1)
        except Exception as error:
            self._fail(f"entry {name!r} could not be read ({type(error).__name__}: {error})")
        if len(data) > size:
            self._fail(f"entry {name!r} is larger than its recorded size")
        return data

    def _fail(self, reason: str) -> NoReturn:
        """Raise ``PlotFileError`` for this file."""
        raise PlotFileError(f"{self.source}: {reason}")


class _Decoder:
    """Rebuild plot objects from tagged JSON; only allowed classes are created."""

    def __init__(self, frames: list[object], source: str) -> None:
        self.frames = frames
        self.source = source
        self.classes = _allowed_classes()
        self.families: set[str] = set()

    def decode(self, value: object, path: str, depth: int) -> object:
        """Return the Python value of the JSON ``value`` at ``path``."""
        if depth > MAXIMUM_DEPTH:
            self._fail(path, f"nesting is deeper than {MAXIMUM_DEPTH}")
        if value is None or isinstance(value, bool | int | float | str):
            return value
        if isinstance(value, list):
            return [
                self.decode(item, f"{path}[{index}]", depth + 1) for index, item in enumerate(value)
            ]
        if not isinstance(value, dict):
            self._fail(path, f"unexpected JSON value {type(value).__name__}")
        tags = [key for key in value if key.startswith("$")]
        if not tags:
            if isinstance(value.get("family"), str):
                self.families.add(value["family"])
            return {
                key: self.decode(item, f"{path}.{key}", depth + 1) for key, item in value.items()
            }
        tag = tags[0]
        payload = value[tag]
        try:
            if tag in ("$frame", "$series", "$array"):
                frame = self._frame(payload, path)
                if tag == "$frame":
                    return frame
                series = cast("pl.DataFrame", frame).to_series(0)
                return series if tag == "$series" else series.to_numpy()
            if tag == "$tuple":
                return tuple(cast("list", self.decode(payload, path, depth + 1)))
            if tag == "$dict":
                return {
                    _hashable(self.decode(key, f"{path} key", depth + 1)): self.decode(
                        item, f"{path}[{key!r}]", depth + 1
                    )
                    for key, item in payload
                }
            if tag == "$float":
                if payload not in ("nan", "inf", "-inf"):
                    self._fail(path, f"invalid $float {payload!r}")
                return float(payload)
            if tag == "$datetime":
                return datetime.datetime.fromisoformat(payload)
            if tag == "$date":
                return datetime.date.fromisoformat(payload)
            if tag == "$time":
                return datetime.time.fromisoformat(payload)
            if tag == "$timedelta":
                days, seconds, microseconds = payload
                return datetime.timedelta(days=days, seconds=seconds, microseconds=microseconds)
            if tag == "$bytes":
                return base64.b64decode(payload, validate=True)
            if tag == "$object":
                return self._object(value, path, depth)
        except PlotFileError:
            raise
        except (TypeError, ValueError, OverflowError) as error:
            self._fail(path, f"invalid {tag} value ({error})")
        self._fail(path, f"unknown tag {tag!r}")

    def _object(self, value: dict[str, Any], path: str, depth: int) -> object:
        """Create an allowed class without calling ``__init__`` and set its state."""
        name = value["$object"]
        cls = self.classes.get(name) if isinstance(name, str) else None
        if cls is None:
            self._fail(path, f"class {name!r} is not a ggrust plot class")
        state = value.get("state")
        if not isinstance(state, dict):
            self._fail(path, "object has no 'state'")
        instance = object.__new__(cls)
        attributes = {
            key: self.decode(item, f"{path}.{_public(key)}", depth + 1)
            for key, item in state.items()
        }
        # object.__setattr__ also works for frozen dataclasses.
        for key, item in attributes.items():
            object.__setattr__(instance, key, item)
        return instance

    def _frame(self, index: object, path: str) -> object:
        """Return stored frame ``index``."""
        if (
            not isinstance(index, int)
            or isinstance(index, bool)
            or not 0 <= index < len(self.frames)
        ):
            self._fail(path, f"data frame reference {index!r} is out of range")
        return self.frames[cast("int", index)]

    def _fail(self, path: str, reason: str) -> NoReturn:
        """Raise ``PlotFileError`` naming the place in plot.json."""
        raise PlotFileError(f"{self.source}: plot.json at {path}: {reason}")


def _register_fonts(fonts: list[tuple[bytes, str]], source: str) -> None:
    """Register stored font files, warning when a family registered before changes."""
    before = {
        family.lower(): [file for file, _ in _ggrust.registered_font_files(family)]
        for _, family in fonts
    }
    changed: list[str] = []
    for data, family in fonts:
        current = before[family.lower()]
        if data in current:
            continue
        try:
            _ggrust.register_font_bytes(data, family)
        except ValueError as error:
            message = f"{source}: stored font of family {family!r} is not usable ({error})"
            raise PlotFileError(message) from error
        if current and family not in changed:
            changed.append(family)
    for family in changed:
        warnings.warn(
            f"{source}: the stored font of family {family!r} is registered over the "
            "family of that name registered in this session",
            GgrustWarning,
            stacklevel=4,
        )


# Shared ------------------------------------------------------------------------------


def _allowed_classes() -> dict[str, type]:
    """Return the classes a plot file may hold, by stable name ``module:qualname``."""
    from ggrust.mapping import MappingMeta
    from ggrust.plot.core import FeatureSpec
    from ggrust.plot.dendrogram import ClusterTree
    from ggrust.plot.subplots import SupPlotsLayoutSpec

    classes: list[type] = [MappingMeta, ClusterTree, SupPlotsLayoutSpec]
    pending: list[type] = [FeatureSpec]
    while pending:
        cls = pending.pop()
        classes.append(cls)
        pending.extend(cls.__subclasses__())
    return {
        f"{cls.__module__}:{cls.__qualname__}": cls
        for cls in classes
        if cls.__module__.startswith("ggrust.")
    }


def _public(attribute: str) -> str:
    """Return a private attribute name without its name-mangling prefix."""
    return (
        attribute.split("__", 1)[1]
        if attribute.startswith("_") and "__" in attribute
        else attribute
    )


def _describe(function: object) -> str:
    """Return a short name for a callable."""
    name = getattr(function, "__qualname__", None) or type(function).__name__
    return f"{name}()"


def _hashable(key: object) -> object:
    """Return a decoded dict key, lists turned into tuples."""
    if isinstance(key, list):
        return tuple(_hashable(item) for item in key)
    return key


def _is_pandas(value: object, class_name: str) -> bool:
    """Check for a pandas type without importing pandas."""
    pandas = sys.modules.get("pandas")
    return pandas is not None and isinstance(value, getattr(pandas, class_name))


def _is_numpy(value: object, class_name: str) -> bool:
    """Check for a numpy type without importing numpy."""
    numpy = sys.modules.get("numpy")
    return numpy is not None and isinstance(value, getattr(numpy, class_name))
