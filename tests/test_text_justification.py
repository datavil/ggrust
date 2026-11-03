"""Mapped `hjust`/`vjust` on geom_text and geom_label must place texts as lets-plot (#1143).

Every row sits at the same point, so the texts' offsets from each other show the
justification alone. lets-plot draws hjust as a text anchor (start, middle, end)
and vjust as a baseline shift; ggrust centres each text, so hjust is compared by
direction and vjust by the exact shift in px.
"""

import re
import warnings

import polars as pl
import pytest

warnings.filterwarnings("ignore", category=SyntaxWarning)

import ggrust as gg  # noqa: E402
import lets_plot as lp  # noqa: E402

HJUST = [0.0, 0.5, 1.0]
VJUST = ["bottom", "center", "top"]
LABELS = ["aa", "bb", "cc"]


def _frame(**columns: list[float] | list[str]) -> pl.DataFrame:
    return pl.DataFrame({"x": [1.0] * 3, "y": [1.0] * 3, "t": LABELS, **columns})


def _ggrust_positions(svg: str) -> dict[str, tuple[float, float]]:
    """Text content -> (x, y) of its ggrust `<text>` element."""
    found = re.findall(r'<text x="([-\d.]+)" y="([-\d.]+)"[^>]*>(aa|bb|cc)<', svg)
    return {content: (float(x), float(y)) for x, y, content in found}


def _lets_plot_texts(svg: str) -> dict[str, tuple[float, float, str]]:
    """Text content -> (x, y, anchor) of a lets-plot text (anchor `start` by default)."""
    pattern = r'translate\(([-\d.]+) ([-\d.]+) \) ">\s*<text([^>]*)>\s*<tspan[^>]*>(aa|bb|cc)<'
    found = {}
    for x, y, attributes, content in re.findall(pattern, svg):
        anchor = re.search(r'text-anchor="(\w+)"', attributes)
        found[content] = (float(x), float(y), anchor.group(1) if anchor else "start")
    return found


def _svg(module, frame: pl.DataFrame, geom: str, mapping: dict[str, str], **constants) -> str:
    layer = getattr(module, f"geom_{geom}")(module.aes(label="t", **mapping), **constants)
    return (module.ggplot(frame, module.aes("x", "y")) + layer).to_svg()


@pytest.mark.parametrize("geom", ["text", "label"])
def test_mapped_hjust_places_texts_as_lets_plot(geom: str) -> None:
    for column in (HJUST, ["left", "middle", "right"]):
        frame = _frame(h=column)
        ours = _ggrust_positions(_svg(gg, frame, geom, {"hjust": "h"}))
        theirs = _lets_plot_texts(_svg(lp, frame, geom, {"hjust": "h"}))
        assert [theirs[label][2] for label in LABELS] == ["start", "middle", "end"]
        # hjust 0 puts the text right of the point, 1 left of it.
        assert ours["aa"][0] > ours["bb"][0] > ours["cc"][0]
        assert ours["aa"][1] == ours["bb"][1] == ours["cc"][1]


@pytest.mark.parametrize("geom", ["text", "label"])
def test_mapped_vjust_shifts_texts_as_lets_plot(geom: str) -> None:
    for column in (VJUST, [0.0, 0.5, 1.0]):
        frame = _frame(v=column)
        ours = _ggrust_positions(_svg(gg, frame, geom, {"vjust": "v"}))
        theirs = _lets_plot_texts(_svg(lp, frame, geom, {"vjust": "v"}))
        for label in ("aa", "cc"):
            our_shift = ours[label][1] - ours["bb"][1]
            their_shift = theirs[label][1] - theirs["bb"][1]
            assert our_shift == pytest.approx(their_shift, abs=0.05)


@pytest.mark.parametrize("geom", ["text", "label"])
def test_mapped_justification_draws_each_row_like_the_constant(geom: str) -> None:
    frame = _frame(h=HJUST, v=VJUST)
    mapped = _ggrust_positions(_svg(gg, frame, geom, {"hjust": "h", "vjust": "v"}))
    for label, hjust, vjust in zip(LABELS, HJUST, VJUST, strict=True):
        constant = _ggrust_positions(_svg(gg, frame, geom, {}, hjust=hjust, vjust=vjust))
        assert mapped[label] == constant[label]


def test_unknown_names_are_centred_with_a_warning() -> None:
    frame = _frame(h=["left", "Left", "sideways"])
    with pytest.warns(gg.GgrustWarning, match=r"mapping\.hjust.*'Left', 'sideways'"):
        svg = _svg(gg, frame, "text", {"hjust": "h"})
    centred = _ggrust_positions(_svg(gg, frame, "text", {}))
    positions = _ggrust_positions(svg)
    assert positions["bb"] == centred["bb"]
    assert positions["cc"] == centred["cc"]
    assert positions["aa"][0] > centred["aa"][0]


def test_missing_values_are_centred() -> None:
    # lets-plot fails to build the plot here; ggrust centres the text.
    frame = _frame(h=[None, 0.0, None])
    positions = _ggrust_positions(_svg(gg, frame, "text", {"hjust": "h"}))
    centred = _ggrust_positions(_svg(gg, frame, "text", {}))
    assert positions["aa"] == centred["aa"]
    assert positions["bb"][0] > centred["bb"][0]
