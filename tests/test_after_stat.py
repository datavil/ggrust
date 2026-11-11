"""``after_stat("name")``: ggplot2's spelling of lets-plot's ``"..name.."``."""

from __future__ import annotations

import ggrust as gg
import pytest
from ggrust.errors import SpecError, UnsupportedValueError

DATA = {"height": [150.0, 152, 158, 160, 161, 165, 170, 171, 178, 185]}


def test_after_stat_spells_the_dotted_name():
    assert gg.after_stat("density") == "..density.."
    assert gg.after_stat("..count..") == "..count.."
    assert gg.after_stat(".width") == "...width.."
    assert gg.after_stat("...width..") == "...width.."


def test_after_stat_renders_like_the_dotted_name():
    base = gg.ggplot(DATA) + gg.geom_histogram(bins=5)
    dotted = base + gg.aes("height", "..density..")
    spelled = base + gg.aes("height", gg.after_stat("density"))
    assert spelled.as_dict() == dotted.as_dict()
    assert spelled.scale_info("y")[0]["domain"] == dotted.scale_info("y")[0]["domain"]


@pytest.mark.parametrize(
    "bad", ["count / max(count)", "", "1count", "a.", "a..b", "..count..extra..", "....count...."]
)
def test_expressions_are_rejected(bad: str):
    with pytest.raises(SpecError, match=r"after_stat\(.*expressions are not supported"):
        gg.after_stat(bad)


def test_non_string_is_rejected():
    with pytest.raises(UnsupportedValueError, match="expected a computed variable name, got int"):
        gg.after_stat(3)  # type: ignore[arg-type]
