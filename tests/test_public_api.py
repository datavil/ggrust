"""The 1.0 stability promise (docs/guide/stability.md), guarded by a snapshot.

``tests/fixtures/public_api.txt`` lists every stable name with its signature: parameter
names, positional order and which are keyword-only. Removing or reordering anything in
it is a breaking change; adding a name or a trailing/keyword-only parameter is not, and
is recorded by re-blessing the file on purpose (``GGRUST_BLESS=1``).
"""

from __future__ import annotations

import difflib
import inspect
import os
import warnings
from pathlib import Path

import ggrust
import ggrust.bistro
import ggrust.data
import ggrust.errors
from ggrust.plot.core import FeatureSpec, FeatureSpecArray, LayerSpec, PlotSpec

SNAPSHOT = Path(__file__).parent / "fixtures" / "public_api.txt"
KIND = inspect.Parameter


def _signature(function: object) -> str:
    """Return the parameters of ``function``: names, order, kinds, has-a-default."""
    try:
        signature = inspect.signature(function)  # ty: ignore[invalid-argument-type]
    except (TypeError, ValueError):
        return "(...)"
    parts: list[str] = []
    star = False
    for parameter in signature.parameters.values():
        if parameter.name in ("self", "cls"):
            continue
        if parameter.kind is KIND.VAR_POSITIONAL:
            parts.append(f"*{parameter.name}")
            star = True
            continue
        if parameter.kind is KIND.VAR_KEYWORD:
            parts.append(f"**{parameter.name}")
            continue
        if parameter.kind is KIND.KEYWORD_ONLY and not star:
            parts.append("*")
            star = True
        parts.append(parameter.name + ("" if parameter.default is KIND.empty else "="))
    return "(" + ", ".join(parts) + ")"


def _members(owner: type, prefix: str) -> list[str]:
    """Return the public members ``owner`` gets from ggrust classes (not from builtins)."""
    lines: list[str] = []
    own = {
        name
        for base in owner.__mro__
        if base.__module__.startswith("ggrust")
        for name in vars(base)
    }
    for name in sorted(own):
        if name.startswith("_"):
            continue
        member = inspect.getattr_static(owner, name)
        if isinstance(member, property):
            lines.append(f"{prefix}.{name}")
        elif callable(getattr(owner, name)):
            lines.append(f"{prefix}.{name}{_signature(getattr(owner, name))}")
        else:
            lines.append(f"{prefix}.{name}")
    return lines


def _public_api() -> list[str]:
    """Return one line per stable name, sorted."""
    lines: list[str] = []
    names = {name: getattr(ggrust, name) for name in ggrust.__all__}
    names["display"] = ggrust.display
    names |= {f"bistro.{name}": getattr(ggrust.bistro, name) for name in ggrust.bistro.__all__}
    names |= {f"errors.{name}": getattr(ggrust.errors, name) for name in ggrust.errors.__all__}
    names |= {f"data.{name}": None for name in ggrust.data.__all__}
    names |= {
        "plot.core.PlotSpec": PlotSpec,
        "plot.core.LayerSpec": LayerSpec,
        "plot.core.FeatureSpec": FeatureSpec,
        "plot.core.FeatureSpecArray": FeatureSpecArray,
    }
    for name, value in sorted(names.items()):
        if isinstance(value, type):
            bases = ", ".join(base.__name__ for base in value.__mro__[1:-1])
            lines.append(f"class {name}{_signature(value)} [{bases}]")
            lines.extend(_members(value, name))
        elif callable(value):
            lines.append(f"{name}{_signature(value)}")
        else:
            lines.append(name)
    return lines


def test_public_api_matches_the_snapshot():
    text = "\n".join(_public_api()) + "\n"
    if os.environ.get("GGRUST_BLESS") == "1":
        SNAPSHOT.write_text(text)
    difference = "\n".join(
        difflib.unified_diff(
            SNAPSHOT.read_text().splitlines(),
            text.splitlines(),
            "snapshot",
            "now",
            lineterm="",
            n=0,
        )
    )
    assert not difference, (
        "the public API changed (see docs/guide/stability.md):\n"
        f"{difference}\n"
        "A removed or reordered name or parameter is a breaking change: keep the old one. "
        "For an addition, re-bless on purpose: "
        "GGRUST_BLESS=1 uv run pytest tests/test_public_api.py"
    )


def test_positional_parameters_match_lets_plot():
    """A lets_plot call with positional arguments binds the same parameters in ggrust."""
    warnings.filterwarnings("ignore", category=SyntaxWarning)  # lets_plot's own docstrings
    import lets_plot

    def positional(function: object) -> list[str]:
        parameters = inspect.signature(function).parameters.values()  # ty: ignore[invalid-argument-type]
        return [p.name for p in parameters if p.kind is KIND.POSITIONAL_OR_KEYWORD]

    mismatches: list[str] = []
    for name in sorted(set(ggrust.__all__) & set(dir(lets_plot))):
        ours, theirs = getattr(ggrust, name), getattr(lets_plot, name)
        if isinstance(ours, type) or not callable(ours) or not callable(theirs):
            continue
        if positional(ours) != positional(theirs):
            mismatches.append(
                f"{name}: ggrust {positional(ours)} != lets_plot {positional(theirs)}"
            )
    assert not mismatches, (
        "positional parameters differ from lets_plot (make ggrust-only parameters "
        "keyword-only, after the lets_plot ones):\n" + "\n".join(mismatches)
    )
