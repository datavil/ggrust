"""gganimate animations (plan.md an1): transition_manual, animate(), Animation, anim_save.

Frame planning follows gganimate's ``animate()``; R and gganimate are not installed
here, so the mapping is tested against values derived by hand from gganimate's source
(``animate.gganim``) instead of an R fixture. Outputs are decoded and compared per
frame, never byte for byte.
"""

from __future__ import annotations

import io
import json
import re
import warnings
from pathlib import Path

import numpy as np
import polars as pl
import pytest
from ggrust import (
    aes,
    anim_save,
    animate,
    display,
    geom_point,
    gggrid,
    ggload,
    ggplot,
    ggsave,
    ggsize,
    ggtitle,
    last_animation,
    transition_manual,
)
from ggrust.errors import ExportError, PlotIndexError, SpecError
from ggrust.plot.core import PlotSpec
from PIL import Image

SALES = pl.DataFrame(
    {
        "year": [2021, 2021, 2021, 2022, 2022, 2023, 2023, 2023, 2023],
        "fruit": ["apple", "pear", "plum", "apple", "pear", "apple", "pear", "plum", "plum"],
        "sold": [10.0, 4.0, 3.0, 12.0, 7.0, 15.0, 9.0, 8.0, 1.0],
        "price": [1.0, 1.5, 2.0, 1.1, 1.6, 1.3, 1.4, 2.4, 2.5],
    }
)


def sales_plot() -> PlotSpec:
    return (
        ggplot(SALES, aes("price", "sold", color="fruit"))
        + geom_point(size=4)
        + transition_manual("year")
        + ggtitle("Sales in {current_frame}", subtitle="{frame}/{nframes}")
        + ggsize(300, 200)
    )


def decoded_frames(data: bytes) -> list[tuple[np.ndarray, int]]:
    """Every frame of a GIF or APNG as RGB pixels, with its duration in ms."""
    image = Image.open(io.BytesIO(data))
    frames = []
    for index in range(getattr(image, "n_frames", 1)):
        image.seek(index)
        frames.append((np.asarray(image.convert("RGB"), dtype=np.int16), image.info["duration"]))
    return frames


def png_pixels(plot: PlotSpec) -> np.ndarray:
    buffer = io.BytesIO()
    plot.to_png(buffer, scale=1)
    return np.asarray(Image.open(io.BytesIO(buffer.getvalue())).convert("RGB"), dtype=np.int16)


def test_transition_manual_spec_and_merge():
    plot = sales_plot()
    assert plot.as_dict()["animation"] == {
        "transition": {"name": "manual", "frames": "year", "cumulative": False}
    }
    replaced = plot + transition_manual("fruit", cumulative=True)
    assert replaced.as_dict()["animation"]["transition"]["frames"] == "fruit"
    assert "animation" not in (ggplot() + geom_point()).as_dict()


@pytest.mark.parametrize(
    ("arguments", "path"),
    [
        ({"frames": ""}, "animation.transition.frames"),
        ({"frames": 3}, "animation.transition.frames"),
        ({"frames": "year", "cumulative": "yes"}, "animation.transition.cumulative"),
    ],
)
def test_transition_manual_rejects_bad_arguments(arguments, path):
    with pytest.raises(SpecError) as caught:
        transition_manual(**arguments)
    assert caught.value.path == path


def test_manual_has_one_frame_per_state_and_keeps_the_duration():
    animation = animate(sales_plot())
    assert (animation.nframes, animation.fps) == (3, pytest.approx(0.3))
    assert repr(animation) == "<Animation: 3 frames at 0.3 fps>"
    animation = animate(sales_plot(), nframes=3, fps=2, start_pause=1, end_pause=2)
    # 0 frames left after pauses would be an error; here 3 - 3 = 0.
    with pytest.raises(SpecError, match="leave no frame"):
        _ = animation.nframes
    animation = animate(sales_plot(), nframes=6, fps=2, start_pause=1, end_pause=2)
    assert (animation.nframes, animation.fps) == (6, 2.0)


def test_rewind_plays_back():
    animation = animate(sales_plot(), nframes=6, fps=1, rewind=True)
    titles = [
        re.findall(r"Sales in (\d+)", animation.frame(index).to_svg())[0]
        for index in range(animation.nframes)
    ]
    assert titles == ["2021", "2022", "2023", "2023", "2022", "2021"]


def test_frames_show_their_state_and_labels():
    animation = animate(sales_plot(), nframes=3, fps=1)
    last = animation.frame(-1).to_svg()
    assert "Sales in 2023" in last and "3/3" in last
    assert "Sales in 2021" in animation.frame(0).to_svg()
    for bad in (3, -4, 1.5, True):
        with pytest.raises(PlotIndexError) as caught:
            animation.frame(bad)
        assert caught.value.length == 3


def test_animate_rejects_bad_options():
    with pytest.raises(SpecError, match="no transition"):
        animate(ggplot(SALES, aes("price", "sold")) + geom_point())
    with pytest.raises(SpecError) as caught:
        animate(sales_plot(), nframes=2.5)
    assert caught.value.path == "animation.animate.nframes"
    with pytest.raises(SpecError) as caught:
        animate(sales_plot(), fps=0)
    assert caught.value.path == "animation.animate.fps"
    with pytest.raises(SpecError, match="both 'width' and 'height'"):
        animate(sales_plot(), width=300)


def test_engine_errors_name_the_spec_path():
    plot = sales_plot() + transition_manual("yeer")
    with pytest.raises(SpecError, match="did you mean 'year'") as caught:
        _ = animate(plot).nframes
    assert caught.value.path == "animation.transition.frames"
    plot = sales_plot() + ggtitle("Sales in {curent_frame}")
    with pytest.raises(SpecError, match="did you mean 'current_frame'") as caught:
        _ = animate(plot).nframes
    assert caught.value.path == "ggtitle.text"
    with pytest.raises(SpecError, match="not supported"):
        gggrid([sales_plot()]).to_svg()


def test_gif_frames_match_the_still_frames():
    animation = animate(sales_plot(), nframes=4, fps=4, end_pause=1)
    frames = decoded_frames(animation.to_gif())
    # 4 frames: 2021, 2022, 2023 held twice (one GIF frame, twice as long).
    assert [duration for _, duration in frames] == [250, 250, 500]
    for index, (pixels, _) in enumerate(frames):
        expected = png_pixels(animation.frame(index))
        assert pixels.shape == expected.shape
        # A 256-colour palette per frame: close, not exact.
        assert np.abs(pixels - expected).mean() < 2.0


def test_apng_frames_equal_the_still_frames():
    animation = animate(sales_plot(), nframes=3, fps=2)
    data = animation.to_apng()
    frames = decoded_frames(data)
    assert [duration for _, duration in frames] == [500, 500, 500]
    for index, (pixels, _) in enumerate(frames):
        assert np.array_equal(pixels, png_pixels(animation.frame(index)))


def test_dpi_scales_raster_frames():
    animation = animate(sales_plot(), nframes=3, fps=2, dpi=192)
    pixels, _ = decoded_frames(animation.to_gif())[0]
    assert pixels.shape[:2] == (400, 600)
    sized = animate(sales_plot(), width=200, height=150)
    pixels, _ = decoded_frames(sized.to_gif())[0]
    assert pixels.shape[:2] == (150, 200)


def test_html_player_payload():
    html = animate(sales_plot(), nframes=6, fps=2, rewind=True)._repr_html_()
    payload = re.search(
        r'<script type="application/vnd.ggrust-animation\+json"[^>]*>(.*?)</script>', html, re.S
    )
    assert payload is not None
    data = json.loads(payload.group(1))
    assert data["kind"] == "svg"
    assert data["timeline"] == [0, 1, 2, 2, 1, 0]
    assert len(data["frames"]) == 3
    assert 'type="application/vnd.ggrust-tooltips+json" class' not in html


def test_dense_points_use_png_frames():
    count = 6_000
    generator = np.random.default_rng(1)
    frame = pl.DataFrame(
        {
            "x": generator.normal(size=2 * count),
            "y": generator.normal(size=2 * count),
            "step": [1] * count + [2] * count,
        }
    )
    plot = ggplot(frame, aes("x", "y")) + geom_point() + transition_manual("step")
    html = animate(plot).to_html()
    assert '"kind":"png"' in html
    assert "data:image/png;base64," in html


def test_notebook_display_animates():
    plot = sales_plot()
    assert "vnd.ggrust-animation+json" in plot._repr_html_()
    display("png")
    try:
        assert plot._repr_html_().startswith('<img src="data:image/gif;base64,')
    finally:
        display("html")


def test_ggsave_and_anim_save(tmp_path):
    plot = sales_plot()
    gif = ggsave(plot, "sales.gif", path=str(tmp_path))
    assert len(decoded_frames(Path(gif).read_bytes())) == 3
    apng = ggsave(plot, "sales.png", path=str(tmp_path))
    assert Image.open(apng).n_frames == 3
    html = ggsave(plot, "sales.html", path=str(tmp_path))
    assert "vnd.ggrust-animation+json" in Path(html).read_text(encoding="utf-8")
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        ggsave(plot, "sales.svg", path=str(tmp_path))
    assert any("transition_manual()" in str(warning.message) for warning in caught)
    with pytest.raises(ExportError, match="needs an animated plot"):
        ggsave(ggplot(SALES, aes("price", "sold")) + geom_point(), "still.gif", path=str(tmp_path))
    animation = animate(plot, nframes=3, fps=1)
    assert last_animation() is animation
    written = anim_save("last.gif", path=str(tmp_path))
    assert decoded_frames(Path(written).read_bytes())[0][1] == 1000
    with pytest.raises(ExportError, match="did you mean 'gif'"):
        anim_save("sales.gfi", animation)


def test_plot_files_keep_the_animation(tmp_path):
    path = ggsave(sales_plot(), "sales.ggrs", path=str(tmp_path))
    loaded = ggload(path)
    assert loaded.as_dict()["animation"] == sales_plot().as_dict()["animation"]
    assert animate(loaded).nframes == 3


def test_cumulative_keeps_earlier_states():
    plot = sales_plot() + transition_manual("year", cumulative=True)
    animation = animate(plot, nframes=3, fps=1)
    counts = [animation.frame(index).to_svg().count("<circle") for index in range(3)]
    plain = animate(sales_plot(), nframes=3, fps=1)
    single = [plain.frame(index).to_svg().count("<circle") for index in range(3)]
    # Legend keys add the same circles to every frame.
    keys = single[0] - 3
    assert [count - keys for count in counts] == [3, 5, 9]


def test_gif_of_a_grid_names_the_grid_limit(tmp_path):
    with pytest.raises(SpecError, match="not supported"):
        ggsave(gggrid([sales_plot(), sales_plot()]), "grid.gif", path=str(tmp_path))
