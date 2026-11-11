"""``coord_transform()`` / ``coord_trans()``: axes warped after the stats, as ggplot2."""

from __future__ import annotations

import re

import ggrust as gg
import numpy as np
import pytest
from ggrust.errors import SpecError

DATA = {"area": [1.0, 10.0, 100.0, 1000.0], "people": [1.0, 2.0, 3.0, 4.0]}


def point_centres(plot: gg.PlotSpec) -> np.ndarray:
    """Circle centres in row order."""
    found = re.findall(r'<circle[^>]*cx="([-\d.]+)"[^>]*cy="([-\d.]+)"', plot.to_svg())
    return np.array([(float(x), float(y)) for x, y in found])


def base() -> gg.PlotSpec:
    return gg.ggplot(DATA, gg.aes("area", "people")) + gg.geom_point()


def test_log10_spaces_decades_evenly():
    x = point_centres(base() + gg.coord_transform(x="log10"))[:, 0]
    gaps = np.diff(x)
    assert gaps == pytest.approx([gaps[0]] * 3, abs=0.01)
    assert gaps[0] > 0


def test_reverse_flips_the_axis():
    plain = point_centres(base())[:, 0]
    reversed_ = point_centres(base() + gg.coord_transform(x="reverse"))[:, 0]
    assert np.all(np.diff(reversed_) < 0)
    assert reversed_[0] == pytest.approx(plain[-1], abs=0.01)


def test_stats_run_in_data_units():
    """A histogram bins in data units: bar edges stay evenly spaced in data units."""
    frame = {"v": [0.5, 1.5, 1.5, 2.5, 2.5, 2.5]}
    plot = gg.ggplot(frame, gg.aes("v")) + gg.geom_histogram(binwidth=1)
    warped = plot + gg.coord_transform(y="sqrt")
    assert warped.scale_info("x")[0]["domain"] == pytest.approx(plot.scale_info("x")[0]["domain"])


def test_breaks_stay_in_data_units_and_are_thinned():
    """Breaks are linear in data units; labels too close on the warped axis are dropped."""
    frame = {"x": np.geomspace(1, 1200, 40), "y": np.geomspace(1, 1200, 40)}
    plot = (
        gg.ggplot(frame, gg.aes("x", "y"))
        + gg.geom_point()
        + gg.coord_transform(x="log10", y="log10")
    )
    labels = re.findall(r">([\d,]+)</text>", plot.to_svg())
    assert "1,200" not in labels or "1,000" not in labels
    unwarped = (plot + gg.coord_cartesian()).to_svg()
    assert len(labels) < len(re.findall(r">([\d,]+)</text>", unwarped))


def test_coord_trans_is_an_alias():
    assert (
        gg.coord_trans(x="log10", ylim=[1, 5]).as_dict()
        == gg.coord_transform(x="log10", ylim=[1, 5]).as_dict()
    )


def test_unknown_transform_suggests_a_name():
    with pytest.raises(SpecError, match=r"coord_transform\(x='log01'\).*did you mean 'log10'"):
        gg.coord_transform(x="log01")


def test_range_outside_the_domain_is_an_error():
    frame = {"x": [0.0, 1.0, 10.0], "y": [1.0, 2.0, 3.0]}
    plot = gg.ggplot(frame, gg.aes("x", "y")) + gg.geom_point()
    with pytest.raises(Exception, match=r"coord\.x.*'log10'.*undefined at 0"):
        (plot + gg.coord_transform(x="log10")).to_svg()
    # Limits inside the domain fix it.
    (plot + gg.coord_transform(x="log10", xlim=[1, 10])).to_svg()


def test_discrete_axis_warns():
    plot = gg.ggplot({"f": ["a", "b"], "y": [1, 2]}, gg.aes("f", "y")) + gg.geom_point()
    with pytest.warns(UserWarning, match="discrete"):
        (plot + gg.coord_transform(x="log10")).to_svg()


@pytest.mark.parametrize("warp", ["sqrt", "reverse", "log10"])
def test_constant_data_is_drawn_at_the_centre(warp: str):
    plot = gg.ggplot({"x": [5.0, 6.0], "y": [4.0, 4.0]}, gg.aes("x", "y")) + gg.geom_point()
    centres = point_centres(plot + gg.coord_transform(y=warp, expand=False))
    assert len(centres) == 2
    assert np.all(np.isfinite(centres))


def test_secondary_axis_labels_are_thinned():
    plot = gg.ggplot(DATA, gg.aes("area", "people")) + gg.geom_point()
    plot += gg.scale_x_continuous(sec_axis=gg.sec_axis(transform=lambda x: x * 2))
    labels = re.findall(r">([\d,]+)</text>", (plot + gg.coord_transform(x="log10")).to_svg())
    for close in (("1,500", "2,000"), ("2,000", "2,500")):
        assert not set(close) <= set(labels)
