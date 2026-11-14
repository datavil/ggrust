//! Frame planning: which scene frame every output frame shows, and at what speed.
//! A port of gganimate's `animate()` (`prepare_args` and the `frame_ind` logic,
//! gganimate 1.0, MIT): `nframes`, `fps`, `duration`, `detail`, `start_pause`,
//! `end_pause`, `rewind`.
//!
//! The transition decides how many scene frames exist (gganimate
//! `adjust_nframes`: one per state for `transition_manual`); the plan picks
//! evenly spaced scene frames, repeats the first and last for the pauses and
//! appends the frames backwards for `rewind`. When fewer frames remain than
//! asked for, `fps` shrinks so the duration stays (gganimate's "nframes and fps
//! adjusted to match transition"). It does not draw anything.

use super::spec::{AnimateOptions, PATH};
use crate::{Error, Result};

/// gganimate defaults (`options('gganimate.*')`).
const DEFAULT_NFRAMES: f64 = 100.0;
const DEFAULT_FPS: f64 = 10.0;

/// `animate()` options checked and with gganimate's defaults filled in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Settings {
    /// Output frames asked for, pauses and rewind included.
    pub nframes: usize,
    pub fps: f64,
    pub detail: usize,
    pub start_pause: usize,
    pub end_pause: usize,
    pub rewind: bool,
}

/// The planned frames of an animation.
#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    /// Per output frame, the scene frame it shows (0-based).
    pub frames: Vec<usize>,
    /// Output frames per second (adjusted when the transition has fewer frames).
    pub fps: f64,
    /// Scene frames the transition has (`{nframes}` in labels).
    pub scene_frames: usize,
}

impl Settings {
    /// gganimate `prepare_args`: defaults, and `duration` turned into `nframes`
    /// (when `fps` is given) or `fps`.
    ///
    /// # Errors
    ///
    /// A spec error at `animation.animate.*` for a value out of range.
    pub(crate) fn resolve(options: &AnimateOptions) -> Result<Settings> {
        let path = |key: &str| format!("{PATH}.animate.{key}");
        let positive = |value: Option<f64>, key: &str| -> Result<Option<f64>> {
            match value {
                Some(value) if !(value.is_finite() && value > 0.0) => Err(Error::spec(
                    path(key),
                    format!("expected a positive number, got {value}"),
                )),
                other => Ok(other),
            }
        };
        let whole = |value: Option<f64>, key: &str, minimum: f64| -> Result<Option<usize>> {
            match value {
                Some(value) if !(value.is_finite() && value >= minimum && value.fract() == 0.0) => {
                    Err(Error::spec(
                        path(key),
                        format!("expected a whole number >= {minimum}, got {value}"),
                    ))
                }
                Some(value) => Ok(Some(value as usize)),
                None => Ok(None),
            }
        };
        let fps_given = positive(options.fps, "fps")?;
        let duration = positive(options.duration, "duration")?;
        let mut nframes = whole(options.nframes, "nframes", 1.0)?.map(|value| value as f64);
        let mut fps = fps_given.unwrap_or(DEFAULT_FPS);
        if let Some(duration) = duration {
            // gganimate: a given fps keeps its value and sets the frame count;
            // otherwise the frame count stays and sets fps.
            if fps_given.is_some() || nframes.is_none() {
                nframes = Some((duration * fps).round().max(1.0));
            } else {
                fps = nframes.unwrap_or(DEFAULT_NFRAMES) / duration;
            }
        }
        Ok(Settings {
            nframes: nframes.unwrap_or(DEFAULT_NFRAMES) as usize,
            fps,
            detail: whole(options.detail, "detail", 1.0)?.unwrap_or(1),
            start_pause: whole(options.start_pause, "start_pause", 0.0)?.unwrap_or(0),
            end_pause: whole(options.end_pause, "end_pause", 0.0)?.unwrap_or(0),
            rewind: options.rewind.unwrap_or(false),
        })
    }

    /// gganimate `animate.gganim`: the output frames. `scene_frames` gets the
    /// number of scene frames asked for (`(nframes - 1) * detail + 1`) and returns
    /// how many the transition has.
    ///
    /// # Errors
    ///
    /// A spec error when the pauses leave no frame, or the transition has none.
    pub(crate) fn plan(&self, scene_frames: impl FnOnce(usize) -> usize) -> Result<Timeline> {
        let requested = self.nframes as i64;
        let mut shown = requested - self.start_pause as i64 - self.end_pause as i64;
        let mut end_pause = self.end_pause;
        if self.rewind {
            // gganimate subtracts start_pause a second time here.
            shown = ((shown - self.start_pause as i64) as f64 / 2.0).ceil() as i64;
            end_pause = end_pause.div_ceil(2);
        }
        if shown < 1 {
            return Err(Error::spec(
                format!("{PATH}.animate.nframes"),
                format!(
                    "{requested} frames leave no frame to animate after start_pause={} and \
                     end_pause={}{}; raise nframes or lower the pauses",
                    self.start_pause,
                    self.end_pause,
                    if self.rewind { " with rewind" } else { "" }
                ),
            ));
        }
        let shown = shown as usize;
        let asked = (shown - 1)
            .checked_mul(self.detail)
            .and_then(|frames| frames.checked_add(1))
            .filter(|&frames| shown <= MAX_FRAMES && frames <= MAX_FRAMES)
            .ok_or_else(|| {
                Error::spec(
                    format!("{PATH}.animate.nframes"),
                    format!(
                        "{requested} frames with detail={} is more than {MAX_FRAMES} frames; \
                         lower nframes or detail",
                        self.detail
                    ),
                )
            })?;
        let total = scene_frames(asked);
        if total == 0 {
            return Err(Error::spec(
                format!("{PATH}.transition"),
                "the transition has no frame to show",
            ));
        }
        // `unique(round(seq(1, total, length.out = shown)))`, 0-based.
        let mut frames: Vec<usize> = r_seq(1.0, total as f64, shown)
            .into_iter()
            .map(|value| value.round_ties_even() as usize - 1)
            .collect();
        frames.dedup();
        let fps = if frames.len() == shown {
            self.fps
        } else {
            self.fps * frames.len() as f64 / shown as f64
        };
        let (first, last) = (frames[0], frames[frames.len() - 1]);
        let mut output = Vec::with_capacity(self.nframes);
        output.extend(std::iter::repeat_n(first, self.start_pause));
        output.extend(&frames);
        output.extend(std::iter::repeat_n(last, end_pause));
        if self.rewind {
            // gganimate appends rows `rev(seq_len(nframes - rows))`; past the rows
            // it has (a transition with fewer frames) it would read missing rows,
            // so the count stops at the rows there are.
            let count = self.nframes.saturating_sub(output.len()).min(output.len());
            let back: Vec<usize> = output[..count].iter().rev().copied().collect();
            output.extend(back);
        }
        Ok(Timeline {
            frames: output,
            fps,
            scene_frames: total,
        })
    }
}

/// R `seq(from, to, length.out = n)`: both ends exact, steps of
/// `(to - from) / (n - 1)` in between.
fn r_seq(from: f64, to: f64, length: usize) -> Vec<f64> {
    match length {
        0 => Vec::new(),
        1 => vec![from],
        2 => vec![from, to],
        _ if from == to => vec![from; length],
        _ => {
            let steps = (length - 1) as f64;
            let by = (to - from) / steps;
            let mut values: Vec<f64> = (0..length - 1)
                .map(|index| from + index as f64 * by)
                .collect();
            values.push(to);
            values
        }
    }
}

/// The most frames an animation plans (`nframes`, and `nframes * detail`).
const MAX_FRAMES: usize = 1_000_000;

/// R `as.character()` of a number: 15 significant digits, shortest form.
pub(crate) fn r_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        return format!("{value}");
    }
    let rounded: f64 = format!("{value:.14e}").parse().unwrap_or(value);
    format!("{rounded}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(options: AnimateOptions) -> Settings {
        Settings::resolve(&options).unwrap()
    }

    fn manual(states: usize, options: AnimateOptions) -> Timeline {
        settings(options).plan(|_| states).unwrap()
    }

    #[test]
    fn defaults_are_gganimates() {
        let resolved = settings(AnimateOptions::default());
        assert_eq!(resolved.nframes, 100);
        assert_eq!(resolved.fps, 10.0);
        assert_eq!(resolved.detail, 1);
        assert!(!resolved.rewind);
    }

    #[test]
    fn duration_sets_frames_or_fps() {
        // fps given: frames follow.
        let resolved = settings(AnimateOptions {
            fps: Some(20.0),
            duration: Some(5.0),
            ..AnimateOptions::default()
        });
        assert_eq!((resolved.nframes, resolved.fps), (100, 20.0));
        // nframes given, fps not: fps follows.
        let resolved = settings(AnimateOptions {
            nframes: Some(50.0),
            duration: Some(10.0),
            ..AnimateOptions::default()
        });
        assert_eq!((resolved.nframes, resolved.fps), (50, 5.0));
        // Neither: default fps sets frames.
        let resolved = settings(AnimateOptions {
            duration: Some(3.0),
            ..AnimateOptions::default()
        });
        assert_eq!((resolved.nframes, resolved.fps), (30, 10.0));
    }

    #[test]
    fn manual_shows_each_state_once_and_keeps_the_duration() {
        // 100 frames at 10 fps asked, 4 states: 4 frames at 0.4 fps (10 s).
        let timeline = manual(4, AnimateOptions::default());
        assert_eq!(timeline.frames, vec![0, 1, 2, 3]);
        assert!((timeline.fps - 0.4).abs() < 1e-12);
        assert_eq!(timeline.scene_frames, 4);
    }

    #[test]
    fn frames_are_spread_evenly_with_r_rounding() {
        // seq(1, 10, length.out = 4) = 1, 4, 7, 10.
        let timeline = settings(AnimateOptions {
            nframes: Some(4.0),
            ..AnimateOptions::default()
        })
        .plan(|asked| {
            assert_eq!(asked, 4);
            10
        })
        .unwrap();
        assert_eq!(timeline.frames, vec![0, 3, 6, 9]);
        assert_eq!(timeline.fps, 10.0);
        // seq(1, 3, length.out = 5) = 1, 1.5, 2, 2.5, 3: round half to even
        // gives 1, 2, 2, 2, 3; unique 1, 2, 3.
        let timeline = settings(AnimateOptions {
            nframes: Some(5.0),
            ..AnimateOptions::default()
        })
        .plan(|_| 3)
        .unwrap();
        assert_eq!(timeline.frames, vec![0, 1, 2]);
        assert!((timeline.fps - 6.0).abs() < 1e-12);
    }

    #[test]
    fn detail_asks_for_more_scene_frames() {
        let mut asked = 0;
        settings(AnimateOptions {
            nframes: Some(10.0),
            detail: Some(3.0),
            ..AnimateOptions::default()
        })
        .plan(|count| {
            asked = count;
            count
        })
        .unwrap();
        assert_eq!(asked, 28);
    }

    #[test]
    fn pauses_repeat_the_first_and_last_frames() {
        let timeline = settings(AnimateOptions {
            nframes: Some(10.0),
            start_pause: Some(2.0),
            end_pause: Some(3.0),
            ..AnimateOptions::default()
        })
        .plan(|count| count)
        .unwrap();
        assert_eq!(timeline.frames, vec![0, 0, 0, 1, 2, 3, 4, 4, 4, 4]);
        assert_eq!(timeline.scene_frames, 5);
    }

    #[test]
    fn rewind_plays_back() {
        // 10 frames: 5 forwards, then the same 5 backwards.
        let timeline = settings(AnimateOptions {
            nframes: Some(10.0),
            rewind: Some(true),
            ..AnimateOptions::default()
        })
        .plan(|count| count)
        .unwrap();
        assert_eq!(timeline.frames, vec![0, 1, 2, 3, 4, 4, 3, 2, 1, 0]);
        // Odd count: 6 forwards, 5 back (gganimate drops the repeated last frame).
        let timeline = settings(AnimateOptions {
            nframes: Some(11.0),
            rewind: Some(true),
            ..AnimateOptions::default()
        })
        .plan(|count| count)
        .unwrap();
        assert_eq!(timeline.frames, vec![0, 1, 2, 3, 4, 5, 4, 3, 2, 1, 0]);
        // A transition with fewer frames: all of them back.
        let timeline = manual(
            3,
            AnimateOptions {
                rewind: Some(true),
                ..AnimateOptions::default()
            },
        );
        assert_eq!(timeline.frames, vec![0, 1, 2, 2, 1, 0]);
        // 50 forwards asked, 3 shown: fps 10 * 3 / 50.
        assert!((timeline.fps - 0.6).abs() < 1e-12);
    }

    #[test]
    fn rewind_with_pauses_follows_gganimate() {
        // nframes 20, start 2, end 4: forwards = ceil((20 - 2 - 4 - 2) / 2) = 6,
        // end pause ceil(4 / 2) = 2; rows 2 + 6 + 2 = 10; 10 rows back.
        let timeline = settings(AnimateOptions {
            nframes: Some(20.0),
            start_pause: Some(2.0),
            end_pause: Some(4.0),
            rewind: Some(true),
            ..AnimateOptions::default()
        })
        .plan(|count| count)
        .unwrap();
        let forwards = vec![0, 0, 0, 1, 2, 3, 4, 5, 5, 5];
        let mut expected = forwards.clone();
        expected.extend(forwards.iter().rev());
        assert_eq!(timeline.frames, expected);
    }

    #[test]
    fn bad_values_are_spec_errors() {
        let error = Settings::resolve(&AnimateOptions {
            nframes: Some(2.5),
            ..AnimateOptions::default()
        })
        .unwrap_err();
        assert!(
            error.to_string().contains("animation.animate.nframes"),
            "{error}"
        );
        let error = Settings::resolve(&AnimateOptions {
            fps: Some(0.0),
            ..AnimateOptions::default()
        })
        .unwrap_err();
        assert!(
            error.to_string().contains("animation.animate.fps"),
            "{error}"
        );
        let error = settings(AnimateOptions {
            nframes: Some(4.0),
            start_pause: Some(2.0),
            end_pause: Some(2.0),
            ..AnimateOptions::default()
        })
        .plan(|count| count)
        .unwrap_err();
        assert!(error.to_string().contains("leave no frame"), "{error}");
    }

    #[test]
    fn numbers_print_as_r_does() {
        assert_eq!(r_number(3.0), "3");
        assert_eq!(r_number(0.25), "0.25");
        assert_eq!(r_number(1.0 / 3.0), "0.333333333333333");
        assert_eq!(r_number(2.0 / 3.0), "0.666666666666667");
    }

    #[test]
    fn huge_frame_counts_are_spec_errors() {
        let options = AnimateOptions {
            nframes: Some(4e9),
            ..AnimateOptions::default()
        };
        let error = settings(options).plan(|_| 3).unwrap_err().to_string();
        assert!(error.contains("animation.animate.nframes"), "{error}");
        let options = AnimateOptions {
            detail: Some(2f64.powi(62)),
            ..AnimateOptions::default()
        };
        assert!(settings(options).plan(|_| 3).is_err());
    }
}
