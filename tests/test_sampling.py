"""Layer sampling (plan 11.1, reduced scope): kept counts, seeds, shortcuts, errors, specs."""

import re
import warnings

import lets_plot
import numpy as np
import pytest
from ggrust import (
    GgrustWarning,
    aes,
    facet_wrap,
    geom_boxplot,
    geom_histogram,
    geom_point,
    ggplot,
    layer_tooltips,
    sampling_fraction,
    sampling_group_random,
    sampling_group_systematic,
    sampling_none,
    sampling_pick,
    sampling_random,
    sampling_random_stratified,
    sampling_systematic,
    sampling_vertex_dp,
    sampling_vertex_vw,
)
from ggrust.errors import SpecError, UnsupportedValueError

RNG = np.random.default_rng(11)
DATA = {"x": RNG.normal(size=1000), "y": RNG.normal(size=1000), "f": ["a"] * 600 + ["b"] * 400}


def points(sampling: object = None, data: dict = DATA) -> str:
    return (ggplot(data, aes("x", "y")) + geom_point(sampling=sampling)).to_svg()


def circles(svg: str) -> int:
    return svg.count("<circle")


@pytest.mark.parametrize(
    ("sampling", "kept"),
    [
        (None, 1000),
        (sampling_none(), 1000),
        ("none", 1000),
        (sampling_random(50), 50),
        (sampling_random(50, seed=3), 50),
        (sampling_systematic(50), 50),
        (sampling_systematic(300), 250),  # every 4th row
        (sampling_fraction(0.2), 200),
        (sampling_fraction(1.0), 1000),
        (sampling_random(1000), 1000),
        (sampling_random(5000), 1000),
        (sampling_systematic(5000), 1000),
        (sampling_random(100) + sampling_systematic(10), 10),
        (50, 50),
    ],
)
def test_kept_counts(sampling: object, kept: int) -> None:
    assert circles(points(sampling)) == kept


def test_same_seed_same_rows_and_seeds_differ() -> None:
    assert points(sampling_random(50, seed=1)) == points(sampling_random(50, seed=1))
    assert points(sampling_random(50, seed=1)) != points(sampling_random(50, seed=2))
    assert points(sampling_fraction(0.1, seed=1)) == points(sampling_fraction(0.1, seed=1))
    assert points(sampling_fraction(0.1, seed=1)) != points(sampling_fraction(0.1, seed=2))


def test_int_shortcut_is_sampling_random() -> None:
    assert geom_point(sampling=500).as_dict()["sampling"] == sampling_random(500).as_dict()
    assert points(50) == points(sampling_random(50))
    assert points(np.int64(50)) == points(sampling_random(50))


def test_kept_rows_are_input_rows_in_order() -> None:
    data = {"x": np.arange(100.0), "y": np.arange(100.0) * 2}
    plot = ggplot(data, aes("x", "y")) + geom_point(sampling=sampling_random(10, seed=4))
    kept = plot.layer_data(0)
    assert kept.height == 10
    xs = kept["x"].to_numpy()
    np.testing.assert_array_equal(kept["y"].to_numpy(), xs * 2)
    assert list(xs) == sorted(set(xs))


def test_samples_the_stat_output_per_panel() -> None:
    # lets-plot samples the histogram's bins, not the input rows.
    histogram = ggplot(DATA) + geom_histogram(aes("x"), bins=30, sampling=sampling_random(3))
    bars = re.findall(r"<rect [^>]*x=", histogram.to_svg())
    assert len(bars) == 1 + 3  # the panel background, then the bars
    faceted = ggplot(DATA, aes("x", "y")) + geom_point(sampling=50) + facet_wrap("f")
    assert circles(faceted.to_svg()) == 100


def test_tooltips_follow_the_kept_rows() -> None:
    names = ["qqfirst", "qqsecond", "qqthird", "qqfourth"]
    data = {"x": [1.0, 2.0, 3.0, 4.0], "y": [1.0, 2.0, 3.0, 4.0], "name": names}
    plot = ggplot(data, aes("x", "y")) + geom_point(
        sampling=sampling_systematic(2), tooltips=layer_tooltips().line("@name")
    )
    html = plot.to_html()
    # Every 2nd row: the first and the third.
    assert [name for name in names if name in html] == ["qqfirst", "qqthird"]


def test_boxplot_rejects_sampling_as_lets_plot() -> None:
    with pytest.raises(TypeError):
        geom_boxplot(sampling="none")


@pytest.mark.parametrize(
    ("ggrust_spec", "lets_plot_spec"),
    [
        (sampling_random(10), lets_plot.sampling_random(10)),
        (sampling_random(10, seed=3), lets_plot.sampling_random(10, seed=3)),
        (sampling_systematic(7), lets_plot.sampling_systematic(7)),
        (sampling_pick(5), lets_plot.sampling_pick(5)),
        (
            sampling_random_stratified(5, seed=1, min_subsample=2),
            lets_plot.sampling_random_stratified(5, seed=1, min_subsample=2),
        ),
        (sampling_group_random(3, seed=2), lets_plot.sampling_group_random(3, seed=2)),
        (sampling_group_systematic(3), lets_plot.sampling_group_systematic(3)),
        (sampling_vertex_vw(30, polygon=True), lets_plot.sampling_vertex_vw(30, polygon=True)),
        (sampling_vertex_dp(30), lets_plot.sampling_vertex_dp(30)),
        (
            sampling_random(10) + sampling_systematic(5),
            lets_plot.sampling_random(10) + lets_plot.sampling_systematic(5),
        ),
    ],
)
def test_as_dict_matches_lets_plot(ggrust_spec: object, lets_plot_spec: object) -> None:
    assert ggrust_spec.as_dict() == lets_plot_spec.as_dict()
    ours = geom_point(sampling=ggrust_spec).as_dict()
    theirs = lets_plot.geom_point(sampling=lets_plot_spec).as_dict()
    assert ours["sampling"] == theirs["sampling"]


def test_sampling_none_is_lets_plots_none() -> None:
    ours = geom_point(sampling=sampling_none()).as_dict()
    assert ours == lets_plot.geom_point(sampling="none").as_dict()
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        points(sampling_none())


def test_skipped_samplers_warn_and_draw_every_row() -> None:
    with pytest.warns(GgrustWarning, match=r"layers\[0\]\.sampling' was ignored"):
        assert circles(points(sampling_pick(5))) == 1000
    with pytest.warns(GgrustWarning, match=r"feature-list\[0\]\.sampling' was ignored"):
        assert circles(points(sampling_vertex_dp(5) + sampling_random(7))) == 7


@pytest.mark.parametrize(
    ("build", "error", "message"),
    [
        (lambda: sampling_random(0), SpecError, r"n=0.*integer >= 1"),
        (lambda: sampling_random(-5), SpecError, r"integer >= 1"),
        (lambda: sampling_random(2.5), UnsupportedValueError, r"float is not accepted"),
        (lambda: sampling_random(True), UnsupportedValueError, r"bool is not accepted"),
        (lambda: sampling_random(5, seed="a"), UnsupportedValueError, r"integer seed"),
        (lambda: sampling_systematic(0), SpecError, r"integer >= 1"),
        (lambda: sampling_fraction(20), SpecError, r"for 20% use fraction=0\.2"),
        (lambda: sampling_fraction(0), SpecError, r"\(0, 1\]"),
        (lambda: sampling_fraction(1.5), SpecError, r"fraction=0\.015"),
        (lambda: sampling_fraction(float("nan")), SpecError, r"\(0, 1\]"),
        (lambda: sampling_fraction("half"), UnsupportedValueError, r"str is not accepted"),
        (lambda: geom_point(sampling=0), SpecError, r"integer >= 1"),
        (lambda: geom_point(sampling=0.2), UnsupportedValueError, r"sampling_fraction\(0\.2\)"),
        (lambda: geom_point(sampling="all"), SpecError, r"'all' is not accepted"),
        (lambda: geom_point(sampling=True), UnsupportedValueError, r"bool is not accepted"),
        (lambda: geom_point(sampling=[5]), UnsupportedValueError, r"list is not accepted"),
    ],
)
def test_errors_at_spec_building(build: object, error: type, message: str) -> None:
    with pytest.raises(error, match=message):
        build()


def test_engine_errors_name_the_path() -> None:
    from ggrust.plot.core import FeatureSpec

    typo = FeatureSpec("sampling", "randon", n=5)
    with pytest.raises(SpecError, match=r"layers\[0\]\.sampling\.name.*did you mean 'random'"):
        points(typo)
    extra = FeatureSpec("sampling", "random", n=5, seeds=1)
    with pytest.raises(SpecError, match=r"no option 'seeds'.*did you mean 'seed'"):
        points(extra)


def test_huge_n_keeps_every_row() -> None:
    assert circles(points(sampling_random(10**30))) == 1000
    assert circles(points(2**64)) == 1000
