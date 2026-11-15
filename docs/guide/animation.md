# Animation

A plot becomes an animation when a `transition_*` is added to it: the transition
splits the data into frames, and everything else (layers, scales, theme) stays an
ordinary plot. The functions and their parameters follow
[gganimate](https://cran.r-project.org/package=gganimate) by Thomas Lin Pedersen and
David Robinson; lets-plot has no animation.

!!! note "Provisional"

    Animation is complete and tested, but new: names and parameters of `animate`,
    `Animation`, `anim_save`, `last_animation`, `transition_*`, `enter_*`, `exit_*`,
    `shadow_*` and `ease_aes` may still change in a minor release, with a note in the
    changelog. See [Stability](stability.md#provisional-features).

## A first animation

`p + transition_manual("year")` draws one frame per value of `year`. Scales are
trained once over all the data, so the axes stay fixed; stats and positions run per
frame. In a notebook the plot plays in a small player (play, pause and a slider), as
the ones on this page.

```python
--8<-- "animation.py:code"
```

--8<-- "figures/animation.html"

`animate(plot, ...)` plans the frames and returns an
[`Animation`](../reference/api/Animation.md). Without it, a plot with a transition
is animated with the defaults (100 frames at 10 frames per second).

| `animate` option | Meaning |
|---|---|
| `nframes`, `fps`, `duration` | Number of frames, frames per second, length in seconds. Give any two. |
| `start_pause`, `end_pause` | Repeat the first or last frame this many times. |
| `rewind` | Play backwards to the start after the last frame. |
| `detail` | Frames computed per shown frame (default 1). |
| `width`, `height`, `dpi` | Size in pixels (default: the plot's `ggsize`) and resolution of the saved file. |

## Transitions

| Transition | Frames | Use it for |
|---|---|---|
| [`transition_manual(frames, cumulative=False)`](../reference/api/transition_manual.md) | One per value, no movement between them. | Flip-book steps; `cumulative=True` keeps earlier values. |
| [`transition_states(states, transition_length=1, state_length=1, wrap=True)`](../reference/api/transition_states.md) | Holds each state, then moves into the next. | Categories or a few time points. |
| [`transition_time(time, range=None)`](../reference/api/transition_time.md) | Evenly spaced over a number, date or date-time column. | Data observed over time. |
| [`transition_reveal(along, range=None, keep_last=True)`](../reference/api/transition_reveal.md) | Reveals the data along a column. | Lines and areas that grow. |

### States

`transition_states` holds each state for `state_length` and moves the marks into the
next one over `transition_length` (relative lengths), and with `wrap=True` from the
last state back to the first. Rows are matched by group: set it with
`aes(group=...)`; discrete mappings such as `fill` split groups too. Numbers and
sizes are eased, colors move through CIE Lab, and categories switch halfway.

```python
--8<-- "animation_states.py:code"
```

--8<-- "figures/animation_states.html"

### Time

`transition_time` spaces the frames evenly over the range of a time column and moves
the marks from each observed time to the next, matched by group as above. Each time
is drawn at its nearest frame; when two times fall on the same frame only the
earlier one is drawn and a warning says so (raise `nframes`).

```python
--8<-- "animation_time.py:code"
```

--8<-- "figures/animation_time.html"

### Reveal

`transition_reveal` keeps the rows reached so far: lines, areas and polygons end at
a point interpolated at the frame, while other geoms (points, texts) show only that
moving point, kept at the end of its group with `keep_last=True`.

```python
--8<-- "animation_reveal.py:code"
```

--8<-- "figures/animation_reveal.html"

## Easing

`ease_aes(default, **aesthetics)` sets how values move during a transition:
`"linear"` moves evenly, `"cubic-in-out"` starts and ends slowly, `"bounce-out"`
bounces at the end. The names are tweenr's: `linear`, or one of `quadratic`, `cubic`,
`quartic`, `quintic`, `sine`, `circular`, `exponential`, `elastic`, `back` and
`bounce`, followed by `-in`, `-out` or `-in-out`. Aesthetics can differ:

```python
plot + ease_aes("cubic-in-out", y="bounce-out")
```

## Enter and exit

Marks without a partner in the next state show only in their own state. Enter and
exit effects let them move in and out instead of popping:
`enter_fade(alpha=0)` / `exit_fade(alpha=0)` fade them, `enter_grow(size=0)` /
`exit_shrink(size=0)` grow points and texts from nothing, bars from the axis, lines
and polygons from their centre. Effects added with `+` apply one after the other, and
they work with `transition_states` and `transition_time`.

```python
--8<-- "animation_enter_exit.py:code"
```

--8<-- "figures/animation_enter_exit.html"

## Shadows

A shadow keeps earlier frames in view. `shadow_wake(wake_length)` trails moving marks
with their last frames (`wake_length` is a share of the animation; `size` and `alpha`
shrink and fade the trail, `falloff` is the ease of that change):

```python
--8<-- "animation_shadow.py:code"
```

--8<-- "figures/animation_shadow.html"

`shadow_mark(past=True, future=False, **styles)` keeps the marks of the states
already shown (or still to come) in the background instead, restyled by the constants
given: `plot + shadow_mark(alpha=0.3, size=1)`. Both take `exclude_layer` (layer
numbers counted from 1) and work with `transition_states` and `transition_time`. A
plot has one shadow: a later one replaces an earlier one.

## Labels

Text in braces in the title, subtitle or caption is filled in per frame:
`labs(title="Year: {frame_time}")`.

| Variable | Available with | Value |
|---|---|---|
| `{frame}`, `{nframes}`, `{progress}` | every transition | Frame number, frame count, share of the animation done (0 to 1). |
| `{current_frame}` | `transition_manual` | The value drawn. |
| `{closest_state}`, `{previous_state}`, `{next_state}` | `transition_states` | The nearest, last and coming state. |
| `{transitioning}` | `transition_states` | `True` while moving between two states. |
| `{frame_time}` | `transition_time` | The time of the frame. |
| `{frame_along}` | `transition_reveal` | The position reached along the column. |

Dates and date-times are written as dates, in one format for the whole animation.

## Saving

The file extension picks the format: `.gif`, `.png` (an animated PNG) or `.html` (a
page with the player).

```python
from ggrust import *
from ggrust.data import penguins

plot = (
    ggplot(penguins, aes("flipper_length_mm", "body_mass_g", color="species"))
    + geom_point(na_rm=True)
    + labs(title="{current_frame}")
    + transition_manual("year")
)

ggsave(plot, "penguins.gif")  # animated with the defaults

animation = animate(plot, nframes=3, fps=1, width=500, height=350)
animation.save("penguins.html")  # or anim_save("penguins.html", animation)
anim_save("penguins.png")  # the last animation made, as gganimate

animation.frame(0).to_png("first_frame.png")  # one frame is an ordinary plot
```

`animation.to_gif()`, `to_apng()` and `to_html()` return the bytes or text when no
path is given. GIF frames that come out identical are merged and later frames hold
only the rectangle that changed, so files stay small.
