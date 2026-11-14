//! What the animation writers share: [`FrameSource`] (distinct scene frames and
//! the output timeline), runs of repeated frames with their display times, and a
//! chunked parallel encoder (frames drawn and encoded on the rayon pool a few at a
//! time, written in order). It does not plan frames (that is `crate::animate`) or
//! know any file format.

use rayon::prelude::*;

use crate::Result;
use crate::scene::Scene;

/// An animation as the writers see it.
pub trait FrameSource: Sync {
    /// Draw scene frame `frame` (an index the timeline holds).
    ///
    /// # Errors
    ///
    /// Any error of drawing the frame.
    fn scene(&self, frame: usize) -> Result<Scene>;
    /// Per output frame, the scene frame it shows.
    fn timeline(&self) -> &[usize];
    /// Output frames per second.
    fn fps(&self) -> f64;
}

/// Consecutive output frames showing the same scene frame, written once.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Run {
    /// The scene frame.
    pub frame: usize,
    /// How long it shows, in `1 / units` seconds (at least 1).
    pub delay: u64,
}

/// The longest delay one GIF or APNG frame holds.
const MAX_DELAY: u64 = u16::MAX as u64;

/// The timeline as runs, with delays in `1 / units` seconds. Delays are rounded
/// from each run's start and end times, so rounding never adds up.
pub(super) fn runs(source: &dyn FrameSource, units: f64) -> Vec<Run> {
    let fps = source.fps();
    let at = |index: usize| (index as f64 * units / fps).round() as u64;
    let timeline = source.timeline();
    let mut runs = Vec::new();
    let mut start = 0;
    while start < timeline.len() {
        let mut end = start + 1;
        while end < timeline.len() && timeline[end] == timeline[start] {
            end += 1;
        }
        // GIF and APNG delays are u16: a longer run is split into several frames.
        let mut delay = at(end).saturating_sub(at(start)).max(1);
        while delay > 0 {
            let part = delay.min(MAX_DELAY);
            runs.push(Run {
                frame: timeline[start],
                delay: part,
            });
            delay -= part;
        }
        start = end;
    }
    runs
}

/// Draw and `encode` every run on the rayon pool, a few at a time, and `write`
/// the results in order. One frame per thread is in flight at a time, so memory
/// holds about one picture per thread.
///
/// # Errors
///
/// The first error of drawing, encoding or writing, in timeline order.
pub(super) fn encode_runs<T: Send>(
    source: &dyn FrameSource,
    runs: &[Run],
    encode: impl Fn(&Scene) -> Result<T> + Sync,
    mut write: impl FnMut(&Run, T) -> Result<()>,
) -> Result<()> {
    let chunk = rayon::current_num_threads().max(1);
    for runs in runs.chunks(chunk) {
        let encoded: Vec<Result<T>> = runs
            .par_iter()
            .map(|run| source.scene(run.frame).and_then(|scene| encode(&scene)))
            .collect();
        for (run, frame) in runs.iter().zip(encoded) {
            write(run, frame?)?;
        }
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::scene::{Color, Node, PerItem, Rect, Rects};

    /// Scene frame `i` is a 20 x 10 picture with a filled square at `x = 2 * i`.
    pub struct Squares {
        pub timeline: Vec<usize>,
        pub fps: f64,
    }

    impl FrameSource for Squares {
        fn scene(&self, frame: usize) -> Result<Scene> {
            Ok(Scene {
                width: 20.0,
                height: 10.0,
                metadata: false,
                background: Some(Color::rgb(255, 255, 255)),
                nodes: vec![Node::Rects(Rects {
                    rects: vec![Rect {
                        x: 2.0 * frame as f64,
                        y: 2.0,
                        width: 4.0,
                        height: 4.0,
                    }],
                    fill: PerItem::Uniform(Some(Color::rgb(200, 30, 30))),
                    stroke: None,
                })],
            })
        }

        fn timeline(&self) -> &[usize] {
            &self.timeline
        }

        fn fps(&self) -> f64 {
            self.fps
        }
    }

    #[test]
    fn repeated_frames_merge_and_delays_do_not_drift() {
        let source = Squares {
            timeline: vec![0, 0, 1, 2, 2, 2],
            fps: 3.0,
        };
        let runs = runs(&source, 100.0);
        let frames: Vec<usize> = runs.iter().map(|run| run.frame).collect();
        assert_eq!(frames, vec![0, 1, 2]);
        // 2/3 s, 1/3 s, 1 s: 67 + 33 + 100 = 200 centiseconds in all.
        let delays: Vec<u64> = runs.iter().map(|run| run.delay).collect();
        assert_eq!(delays, vec![67, 33, 100]);
    }

    #[test]
    fn long_runs_split_into_frames_that_fit_a_u16_delay() {
        // 1000 s per frame in milliseconds: 65535 + 65535 + ... = 1_000_000.
        let source = Squares {
            timeline: vec![0, 1],
            fps: 0.001,
        };
        let runs = runs(&source, 1000.0);
        let total: u64 = runs.iter().map(|run| run.delay).sum();
        assert_eq!(total, 2_000_000);
        assert!(runs.iter().all(|run| run.delay <= MAX_DELAY));
        assert_eq!(runs.first().map(|run| run.frame), Some(0));
        assert_eq!(runs.last().map(|run| run.frame), Some(1));
    }
}
