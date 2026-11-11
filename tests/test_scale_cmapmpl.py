"""scale_color_cmapmpl and scale_fill_cmapmpl, with matplotlib stood in by a small fake."""

from __future__ import annotations

import builtins
import sys
import types
from typing import Any

import pytest
from ggrust import aes, geom_point, ggplot, scale_color_cmapmpl, scale_fill_cmapmpl
from ggrust.errors import SpecError


class _Listed:
    colors = ((1.0, 0.0, 0.0), (0.0, 0.0, 1.0))

    def __call__(self, value: float) -> tuple[float, float, float]:
        return self.colors[round(value)]


def _ramp(value: float) -> tuple[float, float, float, float]:
    return (value, 0.0, 0.0, 1.0)


@pytest.fixture
def matplotlib(monkeypatch: pytest.MonkeyPatch) -> types.ModuleType:
    colors = types.ModuleType("matplotlib.colors")
    colors.ListedColormap = _Listed  # ty: ignore[unresolved-attribute]

    def to_hex(color: tuple[float, ...]) -> str:
        return "#" + "".join(f"{round(channel * 255):02x}" for channel in color[:3])

    colors.to_hex = to_hex  # ty: ignore[unresolved-attribute]
    module = types.ModuleType("matplotlib")
    module.colors = colors  # ty: ignore[unresolved-attribute]
    module.colormaps = {"ramp": _ramp, "pair": _Listed()}  # ty: ignore[unresolved-attribute]
    monkeypatch.setitem(sys.modules, "matplotlib", module)
    monkeypatch.setitem(sys.modules, "matplotlib.colors", colors)
    return module


@pytest.mark.usefixtures("matplotlib")
def test_a_continuous_map_is_sampled_256_times() -> None:
    spec = scale_color_cmapmpl("ramp", name="level").as_dict()
    assert spec["aesthetic"] == "color"
    assert spec["name"] == "level"
    assert len(spec["colors"]) == 256
    assert spec["colors"][0] == "#000000"
    assert spec["colors"][-1] == "#ff0000"


@pytest.mark.usefixtures("matplotlib")
def test_a_listed_map_keeps_its_own_colors() -> None:
    spec = scale_fill_cmapmpl("pair").as_dict()
    assert spec["aesthetic"] == "fill"
    assert spec["colors"] == ["#ff0000", "#0000ff"]


@pytest.mark.usefixtures("matplotlib")
def test_a_colormap_object_is_accepted() -> None:
    assert scale_color_cmapmpl(_ramp).as_dict()["colors"][-1] == "#ff0000"


@pytest.mark.usefixtures("matplotlib")
def test_the_scale_draws() -> None:
    data = {"x": [1, 2], "y": [1, 2], "z": [1, 2]}
    plot = ggplot(data, aes("x", "y", color="z")) + geom_point() + scale_color_cmapmpl("ramp")
    assert "#ff0000" in plot.to_svg()


@pytest.mark.usefixtures("matplotlib")
def test_a_typo_suggests_the_close_name() -> None:
    with pytest.raises(SpecError, match=r"unknown colormap 'rampp' at 'cmap'.*'ramp'"):
        scale_color_cmapmpl("rampp")


def test_a_one_color_map_fails_early(matplotlib: types.ModuleType) -> None:
    single = _Listed()
    single.colors = ((1.0, 0.0, 0.0),)
    matplotlib.colormaps["single"] = single
    with pytest.raises(SpecError, match="at 'cmap' has 1 color"):
        scale_color_cmapmpl("single")


@pytest.mark.usefixtures("matplotlib")
def test_a_wrong_type_names_the_parameter() -> None:
    with pytest.raises(SpecError, match="at 'cmap', got int"):
        scale_color_cmapmpl(3)  # ty: ignore[invalid-argument-type]


def test_missing_matplotlib_says_how_to_fix_it(monkeypatch: pytest.MonkeyPatch) -> None:
    real_import = builtins.__import__

    def no_matplotlib(name: str, *args: Any, **kwargs: Any) -> Any:
        if name.startswith("matplotlib"):
            raise ImportError(name)
        return real_import(name, *args, **kwargs)

    monkeypatch.setattr(builtins, "__import__", no_matplotlib)
    with pytest.raises(SpecError, match="pip install matplotlib"):
        scale_color_cmapmpl("viridis")
