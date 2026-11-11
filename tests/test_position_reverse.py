"""``position_stack(reverse=)`` and ``position_fill(reverse=)``, as ggplot2."""

from __future__ import annotations

import ggrust as gg

DATA = {"month": ["Jan", "Jan"], "shop": ["north", "south"], "sales": [12.0, 8.0]}


def tops(position: gg.FeatureSpec) -> dict[str, float]:
    plot = gg.ggplot(DATA, gg.aes("month", "sales", fill="shop"))
    data = (plot + gg.geom_bar(stat="identity", position=position)).layer_data(0)
    return dict(zip(data["fill"].to_list(), data["ymax"].to_list(), strict=True))


def test_reverse_swaps_the_stacking_order():
    default = tops(gg.position_stack())
    reverse = tops(gg.position_stack(reverse=True))
    assert sorted(default.values()) == [12.0, 20.0]
    assert sorted(reverse.values()) == [8.0, 20.0]
    assert default != reverse


def test_fill_reverse():
    assert sorted(tops(gg.position_fill(reverse=True)).values()) == [0.4, 1.0]
    assert gg.position_fill(reverse=True).as_dict()["reverse"] is True
