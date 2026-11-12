"""Generic scales with ``aesthetic=`` (``scale_manual``, ``scale_gradient``, ...).

A generic scale on a list of aesthetics must equal the per-aesthetic scales added one
by one (same spec, same SVG), and its spec must match lets_plot's.
"""

import re
import warnings
from collections.abc import Callable

import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402
from ggrust.errors import SpecError, UnsupportedValueError  # noqa: E402

FRAME = {
    "x": [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
    "y": [3.0, 1.0, 4.0, 1.0, 5.0, 9.0],
    "group": ["a", "b", "c", "a", "b", "c"],
    "value": [-2.0, -1.0, 0.0, 1.0, 2.0, 3.0],
}

# name -> (generic call, color-specific call, fill-specific call), each a function
# of the module so the same case builds a ggrust or a lets_plot scale.
Case = tuple[Callable, Callable, Callable]
DISCRETE: dict[str, Case] = {
    "manual_list": (
        lambda m, a: m.scale_manual(a, values=["red", "green", "blue"]),
        lambda m: m.scale_color_manual(values=["red", "green", "blue"]),
        lambda m: m.scale_fill_manual(values=["red", "green", "blue"]),
    ),
    "manual_dict": (
        lambda m, a: m.scale_manual(a, values={"c": "red", "a": "blue"}, name="G"),
        lambda m: m.scale_color_manual(values={"c": "red", "a": "blue"}, name="G"),
        lambda m: m.scale_fill_manual(values={"c": "red", "a": "blue"}, name="G"),
    ),
    "hue": (
        lambda m, a: m.scale_hue(a, l=40, c=80),
        lambda m: m.scale_color_hue(l=40, c=80),
        lambda m: m.scale_fill_hue(l=40, c=80),
    ),
    "discrete": (
        lambda m, a: m.scale_discrete(a, direction=-1, name="Group"),
        lambda m: m.scale_color_discrete(direction=-1, name="Group"),
        lambda m: m.scale_fill_discrete(direction=-1, name="Group"),
    ),
    "grey": (
        lambda m, a: m.scale_grey(a, start=0.1, end=0.7),
        lambda m: m.scale_color_grey(start=0.1, end=0.7),
        lambda m: m.scale_fill_grey(start=0.1, end=0.7),
    ),
    "brewer": (
        lambda m, a: m.scale_brewer(a, palette="Dark2"),
        lambda m: m.scale_color_brewer(palette="Dark2"),
        lambda m: m.scale_fill_brewer(palette="Dark2"),
    ),
}
CONTINUOUS: dict[str, Case] = {
    "gradient": (
        lambda m, a: m.scale_gradient(a, low="gold", high="darkred"),
        lambda m: m.scale_color_gradient(low="gold", high="darkred"),
        lambda m: m.scale_fill_gradient(low="gold", high="darkred"),
    ),
    "gradient2": (
        lambda m, a: m.scale_gradient2(a, low="blue", high="red", midpoint=1),
        lambda m: m.scale_color_gradient2(low="blue", high="red", midpoint=1),
        lambda m: m.scale_fill_gradient2(low="blue", high="red", midpoint=1),
    ),
    "gradientn": (
        lambda m, a: m.scale_gradientn(a, colors=["navy", "white", "firebrick"]),
        lambda m: m.scale_color_gradientn(colors=["navy", "white", "firebrick"]),
        lambda m: m.scale_fill_gradientn(colors=["navy", "white", "firebrick"]),
    ),
    "viridis": (
        lambda m, a: m.scale_viridis(a, option="magma", end=0.9),
        lambda m: m.scale_color_viridis(option="magma", end=0.9),
        lambda m: m.scale_fill_viridis(option="magma", end=0.9),
    ),
    "continuous": (
        lambda m, a: m.scale_continuous(a, low="white", high="darkgreen"),
        lambda m: m.scale_color_continuous(low="white", high="darkgreen"),
        lambda m: m.scale_fill_continuous(low="white", high="darkgreen"),
    ),
    "grey": DISCRETE["grey"],
    "brewer": (
        lambda m, a: m.scale_brewer(a, palette="YlGn"),
        lambda m: m.scale_color_brewer(palette="YlGn"),
        lambda m: m.scale_fill_brewer(palette="YlGn"),
    ),
}
CASES = [(name, "group", case) for name, case in DISCRETE.items()] + [
    (name, "value", case) for name, case in CONTINUOUS.items()
]
IDS = [f"{name}-{column}" for name, column, _ in CASES]


def points(m, column: str):
    """Points with both ``color`` and ``fill`` mapped to ``column``."""
    mapping = m.aes("x", "y", color=column, fill=column)
    return m.ggplot(FRAME, mapping) + m.geom_point(shape=21, size=8, stroke=2)


def scale_dicts(plot) -> list[dict]:
    return plot.as_dict()["scales"]


@pytest.mark.parametrize(("name", "column", "case"), CASES, ids=IDS)
def test_list_equals_one_scale_per_aesthetic(name: str, column: str, case: Case) -> None:
    generic, color, fill = case
    combined = points(gg, column) + generic(gg, ["color", "fill"])
    separate = points(gg, column) + color(gg) + fill(gg)
    assert scale_dicts(combined) == scale_dicts(separate)
    assert combined.to_svg() == separate.to_svg()


@pytest.mark.parametrize(("name", "column", "case"), CASES, ids=IDS)
def test_single_aesthetic_equals_specific_scale(name: str, column: str, case: Case) -> None:
    generic, _, fill = case
    assert scale_dicts(points(gg, column) + generic(gg, "fill")) == scale_dicts(
        points(gg, column) + fill(gg)
    )


@pytest.mark.parametrize(("name", "column", "case"), CASES, ids=IDS)
def test_spec_matches_lets_plot(name: str, column: str, case: Case) -> None:
    generic = case[0]
    for aesthetic in ("fill", ["color", "fill"]):
        ours = scale_dicts(gg.ggplot() + generic(gg, aesthetic))
        theirs = scale_dicts(lp.ggplot() + generic(lp, aesthetic))
        assert ours == theirs


def fills(svg: str) -> list[str]:
    return re.findall(r'fill="(#[0-9a-fA-F]{6}|rgb\([^)]*\))"', svg)


def test_list_changes_both_aesthetics() -> None:
    plot = points(gg, "group") + gg.scale_manual(["color", "fill"], values=["red", "lime", "blue"])
    svg = plot.to_svg()
    assert "rgb(255,0,0)" in svg or "#ff0000" in svg.lower()
    # one legend: color and fill share the title and the keys
    assert svg.count(">group<") == 1


def test_continuous_color_defaults_to_gradient() -> None:
    assert gg.scale_continuous("fill").props()["scale_mapper_kind"] == "color_gradient"
    assert gg.scale_continuous("size").props().get("scale_mapper_kind") is None
    mixed = gg.scale_continuous(["color", "size"])
    assert all(spec.props().get("scale_mapper_kind") is None for spec in mixed.elements())


def test_position_list() -> None:
    plot = gg.ggplot(FRAME, gg.aes("x", "y")) + gg.geom_point()
    both = plot + gg.scale_continuous(["x", "y"], expand=[0, 0], oob="keep")
    separate = (
        plot
        + gg.scale_x_continuous(expand=[0, 0], oob="keep")
        + gg.scale_y_continuous(expand=[0, 0], oob="keep")
    )
    assert both.to_svg() == separate.to_svg()


def test_colour_alias() -> None:
    scale = gg.scale_manual(["colour", "fill"], values=["red", "blue"])
    assert [spec.props()["aesthetic"] for spec in scale.elements()] == ["color", "fill"]
    assert gg.scale_hue("colour").props()["aesthetic"] == "color"


def test_tuple_of_aesthetics() -> None:
    scale = gg.scale_gradient(("color", "fill"), low="white", high="black")
    assert [spec.props()["aesthetic"] for spec in scale.elements()] == ["color", "fill"]


@pytest.mark.parametrize(
    ("aesthetic", "error", "match"),
    [
        (5, UnsupportedValueError, r"scale_hue\(aesthetic=5\).*expected an aesthetic name"),
        ([], SpecError, r"scale_hue\(aesthetic=\[\]\).*at least one"),
        (["color", None], UnsupportedValueError, r"expected aesthetic names, got NoneType"),
    ],
)
def test_bad_aesthetic(aesthetic: object, error: type[Exception], match: str) -> None:
    with pytest.raises(error, match=match) as caught:
        gg.scale_hue(aesthetic)  # type: ignore[arg-type]
    assert caught.value.path == "aesthetic"  # type: ignore[attr-defined]


def test_unknown_aesthetic_suggests_a_name() -> None:
    plot = points(gg, "group") + gg.scale_manual(["colr", "fill"], values=["red"] * 3)
    with pytest.raises(SpecError, match=r"(?s)unknown aesthetic 'colr'.*did you mean 'color'"):
        plot.to_svg()


def test_new_scale_tags_every_aesthetic() -> None:
    plot = (
        gg.ggplot(FRAME, gg.aes("x", "y"))
        + gg.geom_point(gg.aes(color="group"))
        + gg.new_scale_color()
        + gg.geom_point(gg.aes(fill="group", color="group"), shape=21, size=6)
        + gg.scale_manual(["color", "fill"], values=["red", "lime", "blue"])
    )
    tagged = plot.as_dict()["scales"]
    assert {scale["aesthetic"]: scale.get("scale_group") for scale in tagged} == {
        "color": {"color": 1},
        "fill": None,
    }
    plot.to_svg()
